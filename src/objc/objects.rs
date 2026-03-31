/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of Objective-C objects.

use super::{Class, ClassHostObject};
use crate::mem::{guest_size_of, GuestUSize, Mem, MutPtr, Ptr, SafeRead};
use std::any::Any;
use std::num::NonZeroU32;

#[repr(C, packed)]
pub struct objc_object {
    pub(super) isa: Class,
}
unsafe impl SafeRead for objc_object {}

#[allow(non_camel_case_types)]
pub type id = MutPtr<objc_object>;

#[allow(non_upper_case_globals)]
pub const nil: id = Ptr::null();

pub(super) struct HostObjectEntry {
    host_object: Box<dyn AnyHostObject>,
    refcount: Option<NonZeroU32>,
}

pub trait HostObject: Any + 'static {
    fn as_superclass<'a>(&'a self) -> Option<&'a (dyn AnyHostObject + 'static)> {
        None
    }
    fn as_superclass_mut<'a>(&'a mut self) -> Option<&'a mut (dyn AnyHostObject + 'static)> {
        None
    }
}

#[macro_export]
macro_rules! impl_HostObject_with_superclass {
    ( $ty:ty ) => {
        impl $crate::objc::HostObject for $ty {
            fn as_superclass<'a>(
                &'a self,
            ) -> Option<&'a (dyn $crate::objc::AnyHostObject + 'static)> {
                Some(&self.superclass)
            }
            fn as_superclass_mut<'a>(
                &'a mut self,
            ) -> Option<&'a mut (dyn $crate::objc::AnyHostObject + 'static)> {
                Some(&mut self.superclass)
            }
        }
    };
}
pub use crate::impl_HostObject_with_superclass;

pub trait AnyHostObject: HostObject {
    fn as_any<'a>(&'a self) -> &'a (dyn Any + 'static);
    fn as_any_mut<'a>(&'a mut self) -> &'a mut (dyn Any + 'static);
    fn type_name(&self) -> &'static str;
}
impl<T: HostObject> AnyHostObject for T {
    fn as_any<'a>(&'a self) -> &'a (dyn Any + 'static) {
        self
    }
    fn as_any_mut<'a>(&'a mut self) -> &'a mut (dyn Any + 'static) {
        self
    }
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

pub struct TrivialHostObject;
impl HostObject for TrivialHostObject {}

impl super::ObjC {
    pub fn read_isa(object: id, mem: &Mem) -> Class {
        mem.read(object).isa
    }

    fn alloc_object_inner(
        &mut self,
        isa: Class,
        instance_size: GuestUSize,
        host_object: Box<dyn AnyHostObject>,
        mem: &mut Mem,
        refcount: Option<NonZeroU32>,
    ) -> id {
        let guest_object = objc_object { isa };
        assert!(instance_size >= guest_size_of::<objc_object>());

        let ptr: MutPtr<objc_object> = mem.alloc(instance_size).cast();
        mem.write(ptr, guest_object);
        assert!(!self.objects.contains_key(&ptr));
        self.objects.insert(
            ptr,
            HostObjectEntry {
                host_object,
                refcount,
            },
        );
        ptr
    }

    pub fn alloc_object(
        &mut self,
        isa: Class,
        host_object: Box<dyn AnyHostObject>,
        mem: &mut Mem,
    ) -> id {
        let &ClassHostObject { instance_size, .. } = self.borrow(isa);
        self.alloc_object_inner(
            isa,
            instance_size,
            host_object,
            mem,
            Some(NonZeroU32::new(1).unwrap()),
        )
    }

    pub fn alloc_static_object(
        &mut self,
        isa: Class,
        host_object: Box<dyn AnyHostObject>,
        mem: &mut Mem,
    ) -> id {
        let size = guest_size_of::<objc_object>();
        self.alloc_object_inner(isa, size, host_object, mem, None)
    }

    pub fn register_static_object(
        &mut self,
        guest_object: id,
        host_object: Box<dyn AnyHostObject>,
    ) {
        assert!(!self.objects.contains_key(&guest_object));
        self.objects.insert(
            guest_object,
            HostObjectEntry {
                host_object,
                refcount: None,
            },
        );
    }

    pub fn get_host_object(&self, object: id) -> Option<&dyn AnyHostObject> {
        self.objects.get(&object).map(|entry| &*entry.host_object)
    }

    pub fn borrow<T: AnyHostObject + 'static>(&self, object: id) -> &T {
        let entry = self.objects.get(&object).expect(&format!(
            "Attempted to borrow unknown object {object:?}. This often happens when an asset (like bg.png) is missing."
        ));
        let mut host_object: &(dyn AnyHostObject + 'static) = &*entry.host_object;
        loop {
            if let Some(res) = host_object.as_any().downcast_ref() {
                return res;
            } else if let Some(next) = host_object.as_superclass() {
                host_object = next;
            } else {
                panic!(
                    "Could not find host object with type {:?}, found {:?} for {object:?}",
                    std::any::type_name::<T>(),
                    host_object.type_name(),
                );
            }
        }
    }

    pub fn borrow_mut<T: AnyHostObject + 'static>(&mut self, object: id) -> &mut T {
        type Aho = dyn AnyHostObject + 'static;
        let entry = self.objects.get_mut(&object).expect(&format!(
            "Attempted to borrow_mut unknown object {object:?}"
        ));
        let mut host_object: &mut Aho = &mut *entry.host_object;
        loop {
            if let Some(res) = unsafe { &mut *(host_object as *mut Aho) }
                .as_any_mut()
                .downcast_mut()
            {
                return res;
            } else if let Some(next) = host_object.as_superclass_mut() {
                host_object = next;
            } else {
                let entry = self.objects.get(&object).unwrap();
                let host_object: &Aho = &*entry.host_object;
                panic!(
                    "Could not find host object with type {:?}, found {:?} for {object:?}",
                    std::any::type_name::<T>(),
                    host_object.type_name(),
                );
            }
        }
    }

    pub fn get_refcount(&mut self, object: id) -> NonZeroU32 {
        if object == nil {
            return NonZeroU32::new(1).unwrap();
        }
        let Some(entry) = self.objects.get_mut(&object) else {
            panic!("No entry found for object {object:?}, it may have already been deallocated");
        };
        let Some(refcount) = entry.refcount.as_mut() else {
            panic!("Attempt to get refcount on static-lifetime object {object:?}!");
        };
        *refcount
    }

    pub fn increment_refcount(&mut self, object: id) {
        if object == nil { return; }
        let Some(entry) = self.objects.get_mut(&object) else {
            log!("Warning: Attempted to increment refcount on unknown object {:?}", object);
            return;
        };
        let Some(refcount) = entry.refcount.as_mut() else {
            panic!("Attempt to increment refcount on static-lifetime object {object:?}!");
        };
        *refcount = refcount.checked_add(1).unwrap();
    }

    #[must_use]
    pub fn decrement_refcount(&mut self, object: id) -> bool {
        if object == nil { return false; }
        let Some(entry) = self.objects.get_mut(&object) else {
            log!("Warning: Attempted to decrement refcount on unknown object {:?}", object);
            return false;
        };
        let Some(refcount) = entry.refcount.as_mut() else {
            panic!("Attempt to decrement refcount on static-lifetime object {object:?}!");
        };
        if refcount.get() == 1 {
            entry.refcount = None;
            true
        } else {
            *refcount = NonZeroU32::new(refcount.get() - 1).unwrap();
            false
        }
    }

    pub fn dealloc_object(&mut self, object: id, mem: &mut Mem) {
        // ИСПРАВЛЕНИЕ: Если объект nil или не найден в списке — просто выходим, а не падаем.
        if object == nil { return; }
        
        let Some(HostObjectEntry { host_object, refcount }) = self.objects.remove(&object) else {
            log!("Warning: Attempted to deallocate unknown object {:?}. Ignoring to prevent crash.", object);
            return;
        };

        if let Some(refcount) = refcount {
            log!(
                "Warning: {:?} was deallocated with non-zero reference count: {:?}",
                object,
                refcount
            );
        }

        std::mem::drop(host_object);
        mem.free(object.cast());
    }
}

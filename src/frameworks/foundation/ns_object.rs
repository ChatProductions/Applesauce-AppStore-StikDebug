/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//!
//! `NSObject`, the root of most class hierarchies in Objective-C.
//!
//! Resources:
//!
//! - Apple's [Advanced Memory Management Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/MemoryMgmt/Articles/MemoryMgmt.html)
//!
//! explains how reference counting works. Note that we are interested in what
//!
//! it calls "manual retain-release", not ARC.
//!
//! - Apple's [Key-Value Coding Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/KeyValueCoding/SearchImplementation.html)
//!   explains the algorithm `setValue:forKey:` should follow.
//!
//!
//! See also: [crate::objc], especially the `objects` module.

use super::ns_string::{from_rust_string, to_rust_string};
use super::{NSTimeInterval, NSUInteger};
use crate::frameworks::foundation::ns_run_loop::{add_perform_request, cancel_perform_requests};
use crate::frameworks::foundation::ns_thread::detach_new_thread_inner;
use crate::libc::semaphore::{host_destroy_semaphore, sem_wait};
use crate::mem::MutVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, msg_send_no_type_checking, nil, objc_classes,
    retain, Class, ClassExports, NSZonePtr, ObjC, TrivialHostObject, SEL,
};

// Хранилище для отмененных таймеров (target, имя селектора в виде строки)
pub static mut CANCELLED_PERFORMS: std::vec::Vec<(u32, std::option::Option<std::string::String>)> = std::vec::Vec::new();

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSObject

+ (id)alloc {
    msg![env; this allocWithZone:(MutVoidPtr::null())]
}
+ (id)allocWithZone:(NSZonePtr)_zone { // struct _NSZone*
    log_dbg!("[{:?} allocWithZone:]", this);
    env.objc.alloc_object(this, Box::new(TrivialHostObject), &mut env.mem)
}

+ (id)new {
    let new_object: id = msg![env; this alloc];
    msg![env; new_object init]
}

+ (Class)class {
    this
}
+ (bool)isSubclassOfClass:(Class)class {
    env.objc.class_is_subclass_of(this, class)
}

// See the instance method section for the normal versions of these.
+ (id)retain {
    this // classes are not refcounted
}
+ (())release {
    // classes are not refcounted
}
+ (())autorelease {
    // classes are not refcounted
}

+ (bool)instancesRespondToSelector:(SEL)selector {
    env.objc.class_has_method(this, selector)
}

// Возвращаем u32 (адрес), так как IMP не реализует GuestRet
+ (u32)instanceMethodForSelector:(SEL)_selector {
    log!("Warning: instanceMethodForSelector: for {:?} is stubbed", _selector);
    0
}

+ (bool)accessInstanceVariablesDirectly {
    true
}

+ (id)description {
    let name = env.objc.get_class_name(this);
    let str = from_rust_string(env, name.to_string());
    autorelease(env, str)
}

+ (id)debugDescription {
    msg![env; this description]
}

+ (())cancelPreviousPerformRequestsWithTarget:(id)target selector:(SEL)selector object:(id)arg {
    let run_loop: id = msg_class![env; NSRunLoop currentRunLoop];
    cancel_perform_requests(env, run_loop, target, selector, arg);
}

// ИЗМЕНЕНО: Поддержка глобальной отмены для объекта
+ (())cancelPreviousPerformRequestsWithTarget:(id)target {
    unsafe {
        crate::frameworks::foundation::ns_object::CANCELLED_PERFORMS.push((target.to_bits(), None));
    }
}

- (id)init {
    this
}

- (NSUInteger)retainCount {
    env.objc.get_refcount(this).into()
}

- (id)retain {
    log_dbg!("[{:?} retain]", this);
    env.objc.increment_refcount(this);
    this
}
- (())release {
    log_dbg!("[{:?} release]", this);
    if env.objc.decrement_refcount(this) {
        () = msg![env; this dealloc];
    }
}
- (id)autorelease {
    () = msg_class![env; NSAutoreleasePool addObject:this];
    this
}

- (())dealloc {
    log_dbg!("[{:?} dealloc]", this);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (Class)class {
    ObjC::read_isa(this, &env.mem)
}
- (bool)isMemberOfClass:(Class)class {
    let this_class: Class = msg![env; this class];
    class == this_class
}
- (bool)isKindOfClass:(Class)class {
    let this_class: Class = msg![env; this class];
    env.objc.class_is_subclass_of(this_class, class)
}

- (NSUInteger)hash {
    this.to_bits()
}

// To not confuse with isEqualTo:, which is
// a category of NSWhoseSpecifier!
// Reference https://nshipster.com/equality
- (bool)isEqual:(id)other {
    this == other
}

// Helper for NSCopying
- (id)copy {
    msg![env; this copyWithZone:(MutVoidPtr::null())]
}

// Helper for NSMutableCopying
- (id)mutableCopy {
    msg![env; this mutableCopyWithZone:(MutVoidPtr::null())]
}

// NSKeyValueCoding
- (())setValue:(id)value
       forKey:(id)key { // NSString*
    let key_string = to_rust_string(env, key);
    // TODO: avoid copy?
    assert!(key_string.is_ascii()); // TODO: do we have to handle non-ASCII keys?
    let camel_case_key_string = format!("{}{}", key_string.as_bytes()[0].to_ascii_uppercase() as char, &key_string[1..]);

    let class = msg![env; this class];

    assert!(value != nil);
    let value_class = msg![env; value class];
    let ns_value_class = env.objc.get_known_class("NSValue", &mut env.mem);
    assert!(!env.objc.class_is_subclass_of(value_class, ns_value_class));

    if let Some(sel) = env.objc.lookup_selector(&format!("set{camel_case_key_string}:")) {
        if env.objc.class_has_method(class, sel) {
            () = msg_send(env, (this, sel, value));
            return;
        }
    }

    if let Some(sel) = env.objc.lookup_selector(&format!("_set{camel_case_key_string}:")) {
        if env.objc.class_has_method(class, sel) {
            () = msg_send(env, (this, sel, value));
            return;
        }
    }

    let sel = env.objc.lookup_selector("accessInstanceVariablesDirectly").unwrap();
    let accessInstanceVariablesDirectly = msg_send(env, (class, sel));

    if accessInstanceVariablesDirectly {
        if let Some(ivar_ptr) = env.objc.object_lookup_ivar(&env.mem, this, &format!("_{key_string}"))
            .or_else(|| env.objc.object_lookup_ivar(&env.mem, this, &format!("_is{camel_case_key_string}")))
            .or_else(|| env.objc.object_lookup_ivar(&env.mem, this, &format!("{key_string}")))
            .or_else(|| env.objc.object_lookup_ivar(&env.mem, this, &format!("is{camel_case_key_string}"))
        ) {
            retain(env, value);
            env.mem.write(ivar_ptr.cast(), value);
            return;
        }
    }

    let sel = env.objc.lookup_selector("setValue:forUndefinedKey:").unwrap();
    () = msg_send(env, (this, sel, value, key));
}

- (())setValue:(id)_value
forUndefinedKey:(id)key { // NSString*
    let class: Class = ObjC::read_isa(this, &env.mem);
    let class_name_string = env.objc.get_class_name(class).to_owned();
    let key_string = to_rust_string(env, key);
    log!("Warning: Object {:?} of class {:?} does not have a setter for {} — ignoring",
        this, class_name_string, key_string);
}

- (bool)respondsToSelector:(SEL)selector {
    env.objc.object_has_method(&env.mem, this, selector)
}

- (bool)conformsToProtocol:(id)_protocol {
    true
}
    
// Возвращаем u32 (адрес), так как IMP не реализует GuestRet
- (u32)methodForSelector:(SEL)_selector {
    log!("Warning: methodForSelector: for {:?} is stubbed", _selector);
    0
}

- (id)performSelector:(SEL)sel {
    assert!(!sel.is_null());
    msg_send_no_type_checking(env, (this, sel))
}

- (id)performSelector:(SEL)sel
           withObject:(id)o1 {
    assert!(!sel.is_null());
    msg_send_no_type_checking(env, (this, sel, o1))
}

- (id)performSelector:(SEL)sel
           withObject:(id)o1
           withObject:(id)o2 {
    assert!(!sel.is_null());
    msg_send_no_type_checking(env, (this, sel, o1, o2))
}

- (())performSelectorInBackground:(SEL)sel
                       withObject:(id)arg {
    detach_new_thread_inner(env, sel, this, arg, /* tolerate_type_mismatch: */ true)
}

- (())performSelector:(SEL)sel withObject:(id)arg afterDelay:(NSTimeInterval)delay {
    let run_loop: id = msg_class![env; NSRunLoop currentRunLoop];
    add_perform_request(env, run_loop, this, sel, arg, Some(delay), false);
}

- (())performSelectorOnMainThread:(SEL)sel withObject:(id)arg waitUntilDone:(bool)wait {
    log_dbg!("performSelectorOnMainThread:{} withObject:{:?} waitUntilDone:{}", sel.as_str(&env.mem), arg, wait);

    if wait && env.current_thread == 0 {
        if sel.as_str(&env.mem).ends_with(':') {
            () = msg_send(env, (this, sel, arg));
        } else {
            assert!(arg.is_null());
            () = msg_send(env, (this, sel));
        }
        return;
    }

    let run_loop: id = msg_class![env; NSRunLoop mainRunLoop];
    let sem = add_perform_request(env, run_loop, this, sel, arg, None, wait);
    if wait {
        sem_wait(env, sem);
        host_destroy_semaphore(env, sem);
}

- (())awakeFromNib {
    // no-op
}

- (())performSelector:(SEL)sel
           onThread:(id)_thread
         withObject:(id)arg
      waitUntilDone:(bool)_wait {
    log_dbg!("performSelector:{} onThread:withObject:waitUntilDone: — scheduling on main thread instead", sel.as_str(&env.mem));
    msg![env; this performSelector:sel withObject:arg afterDelay:0.0]
}

- (())performSelector:(SEL)sel
           onThread:(id)_thread
         withObject:(id)arg
      waitUntilDone:(bool)_wait
              modes:(id)_modes {
    log_dbg!("performSelector:{} onThread:withObject:waitUntilDone:modes: — scheduling on main thread instead", sel.as_str(&env.mem));
    msg![env; this performSelector:sel withObject:arg afterDelay:0.0]
}

- (id)valueForKey:(id)key {
    // Try getter selector first
    let key_str = super::ns_string::to_rust_string(env, key);
    let sel_name = key_str.to_string();
    if let Some(sel) = env.objc.lookup_selector(&sel_name) {
        if env.objc.object_has_method(&env.mem, this, sel) {
            return msg_send(env, (this, sel));
        }
    }
    // Try isX for bool properties
    let is_sel_name = format!("is{}{}", &key_str[..1].to_uppercase(), &key_str[1..]);
    if let Some(sel) = env.objc.lookup_selector(&is_sel_name) {
        if env.objc.object_has_method(&env.mem, this, sel) {
            return msg_send(env, (this, sel));
        }
    }
    log!("Warning: valueForKey:{} not found on {:?} — returning nil", key_str, this);
    nil
}

- (id)valueForKeyPath:(id)key_path {
    // Simple implementation: treat as valueForKey: (no path traversal)
    msg![env; this valueForKey:key_path]
}

- (())setValue:(id)value forKeyPath:(id)key_path {
    msg![env; this setValue:value forKey:key_path]
}

@end

};


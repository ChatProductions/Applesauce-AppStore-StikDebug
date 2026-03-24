/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 */

use super::ns_string::to_rust_string;
use super::{NSRange, NSUInteger};
use crate::frameworks::foundation::ns_keyed_unarchiver::decode_current_data;
use crate::fs::GuestPath;
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr, MutVoidPtr, Ptr};
use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr,
};
use crate::{msg_class, Environment};

pub(super) struct NSDataHostObject {
    pub(super) bytes: MutVoidPtr,
    pub(super) length: NSUInteger,
    free_when_done: bool,
}

impl HostObject for NSDataHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSData: NSObject

+ (id)allocWithZone:(NSZonePtr)zone {
    let _ = zone;
    let host = Box::new(NSDataHostObject {
        bytes: Ptr::null(),
        length: 0,
        free_when_done: true,
    });
    env.objc.alloc_object(this, host, &mut env.mem)
}

+ (id)dataWithContentsOfURL:(id)url options:(NSUInteger)options error:(MutVoidPtr)error {
    let _ = options;
    let _ = error;
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithContentsOfURL:url];
    autorelease(env, new)
}

- (id)initWithContentsOfURL:(id)url {
    let url_str: id = msg![env; url absoluteString];
    let path = to_rust_string(env, url_str);

    // Простейшая реализация: считаем URL как файл
    if let Ok(bytes) = env.fs.read(GuestPath::new(&path)) {
        let size = bytes.len() as u32;
        let alloc = env.mem.alloc(size);

        let ptr: MutPtr<u8> = alloc.cast();
        let slice = env.mem.bytes_at_mut(ptr, size);
        slice.copy_from_slice(&bytes);

        let host = env.objc.borrow_mut::<NSDataHostObject>(this);
        host.bytes = alloc;
        host.length = size;

        return this;
    }

    nil
}

- (id)initWithContentsOfFile:(id)path {
    if path == nil {
        return nil;
    }

    let path = to_rust_string(env, path);

    let Ok(bytes) = env.fs.read(GuestPath::new(&path)) else {
        release(env, this);
        return nil;
    };

    let size = bytes.len() as u32;
    let alloc = env.mem.alloc(size);

    let ptr: MutPtr<u8> = alloc.cast();
    let slice = env.mem.bytes_at_mut(ptr, size);
    slice.copy_from_slice(&bytes);

    let host = env.objc.borrow_mut::<NSDataHostObject>(this);
    host.bytes = alloc;
    host.length = size;

    this
}

- (ConstVoidPtr)bytes {
    env.objc.borrow::<NSDataHostObject>(this).bytes.cast_const()
}

- (NSUInteger)length {
    env.objc.borrow::<NSDataHostObject>(this).length
}

- (())dealloc {
    let &NSDataHostObject { bytes, free_when_done, .. } =
        env.objc.borrow(this);

    if !bytes.is_null() && free_when_done {
        env.mem.free(bytes);
    }

    env.objc.dealloc_object(this, &mut env.mem)
}

@end

@implementation NSMutableData: NSData

- (id)initWithLength:(NSUInteger)length {
    let host = env.objc.borrow_mut::<NSDataHostObject>(this);

    assert!(host.bytes.is_null() && host.length == 0);

    let alloc = env.mem.calloc(length);

    host.bytes = alloc;
    host.length = length;

    this
}

@end

};

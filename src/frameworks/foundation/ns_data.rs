/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, you can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSData` and `NSMutableData`.

use super::ns_string::to_rust_string;
use super::NSUInteger;
use crate::frameworks::foundation::ns_keyed_unarchiver::decode_current_data;
use crate::fs::GuestPath;
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr, MutVoidPtr, Ptr};
use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
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
    let host_object = Box::new(NSDataHostObject {
        bytes: Ptr::null(),
        length: 0,
        free_when_done: true,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)dataWithBytesNoCopy:(MutVoidPtr)bytes length:(NSUInteger)length {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithBytesNoCopy:bytes length:length];
    autorelease(env, new)
}

+ (id)dataWithBytesNoCopy:(MutVoidPtr)bytes length:(NSUInteger)length freeWhenDone:(bool)free_when_done {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithBytesNoCopy:bytes length:length freeWhenDone:free_when_done];
    autorelease(env, new)
}

+ (id)dataWithBytes:(ConstVoidPtr)bytes length:(NSUInteger)length {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithBytes:bytes length:length];
    autorelease(env, new)
}

+ (id)dataWithContentsOfFile:(id)path {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithContentsOfFile:path];
    autorelease(env, new)
}

+ (id)dataWithContentsOfMappedFile:(id)path {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithContentsOfMappedFile:path];
    autorelease(env, new)
}

+ (id)dataWithContentsOfURL:(id)url {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithContentsOfURL:url];
    autorelease(env, new)
}

+ (id)dataWithContentsOfURL:(id)url options:(NSUInteger)options error:(MutVoidPtr)error {
    let _ = options;
    let _ = error;
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithContentsOfURL:url];
    autorelease(env, new)
}

+ (id)dataWithData:(id)data {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithData:data];
    autorelease(env, new)
}

- (id)initWithBytesNoCopy:(MutVoidPtr)bytes length:(NSUInteger)length {
    msg![env; this initWithBytesNoCopy:bytes length:length freeWhenDone:true]
}

- (id)initWithBytesNoCopy:(MutVoidPtr)bytes length:(NSUInteger)length freeWhenDone:(bool)free_when_done {
    let host_object = env.objc.borrow_mut::<NSDataHostObject>(this);
    assert!(host_object.bytes.is_null() && host_object.length == 0);
    host_object.bytes = bytes;
    host_object.length = length;
    host_object.free_when_done = free_when_done;
    this
}

- (id)initWithBytes:(ConstVoidPtr)bytes length:(NSUInteger)length {
    let host_object = env.objc.borrow_mut::<NSDataHostObject>(this);
    assert!(host_object.bytes.is_null() && host_object.length == 0);

    let alloc = env.mem.alloc(length);
    env.mem.memmove(alloc, bytes, length);

    host_object.bytes = alloc;
    host_object.length = length;
    this
}

- (id)initWithData:(id)data {
    let bytes: ConstVoidPtr = msg![env; data bytes];
    let length: NSUInteger = msg![env; data length];
    msg![env; this initWithBytes:bytes length:length]
}

- (id)initWithContentsOfURL:(id)url {
    let url_str_id: id = msg![env; url absoluteString];
    let path = to_rust_string(env, url_str_id);
    log!("TODO: NSData initWithContentsOfURL for {:?}", path);
    nil
}

- (id)initWithContentsOfURL:(id)url options:(NSUInteger)options error:(MutVoidPtr)error {
    let _ = options;
    let _ = error;
    msg![env; this initWithContentsOfURL:url]
}

- (id)initWithContentsOfFile:(id)path {
    if path == nil {
        return nil;
    }
    let path_str = to_rust_string(env, path);
    let Ok(bytes) = env.fs.read(GuestPath::new(&path_str)) else {
        release(env, this);
        return nil;
    };
    let size = bytes.len().try_into().unwrap();
    let alloc = env.mem.alloc(size);
    let casted_alloc: MutPtr<u8> = alloc.cast();
    let slice = env.mem.bytes_at_mut(casted_alloc, size);
    slice.copy_from_slice(&bytes);

    let host_object = env.objc.borrow_mut::<NSDataHostObject>(this);
    host_object.bytes = alloc;
    host_object.length = size;
    this
}

- (id)initWithContentsOfMappedFile:(id)path {
    msg![env; this initWithContentsOfFile:path]
}

- (bool)writeToFile:(id)path atomically:(bool)use_aux_file {
    let _ = use_aux_file;
    let file = to_rust_string(env, path);
    let host_object = env.objc.borrow::<NSDataHostObject>(this);
    let slice = if host_object.length == 0 {
        &[]
    } else {
        let casted_ptr: ConstPtr<u8> = host_object.bytes.cast_const().cast();
        env.mem.bytes_at(casted_ptr, host_object.length)
    };
    env.fs.write(GuestPath::new(&file), slice).is_ok()
}

- (())dealloc {
    let &NSDataHostObject { bytes, free_when_done, .. } = env.objc.borrow(this);
    if !bytes.is_null() && free_when_done {
        env.mem.free(bytes);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)copyWithZone:(NSZonePtr)zone {
    let _ = zone;
    retain(env, this)
}

- (id)initWithCoder:(id)coder {
    release(env, this);
    decode_current_data(env, coder, true)
}

- (id)mutableCopyWithZone:(NSZonePtr)zone {
    let _ = zone;
    let bytes: ConstVoidPtr = msg![env; this bytes];
    let length: NSUInteger = msg![env; this length];
    let new = msg_class![env; NSMutableData alloc];
    msg![env; new initWithBytes:bytes length:length]
}

- (ConstVoidPtr)bytes {
    env.objc.borrow::<NSDataHostObject>(this).bytes.cast_const()
}

- (NSUInteger)length {
    env.objc.borrow::<NSDataHostObject>(this).length
}

- (bool)isEqualToData:(id)other {
    let a = to_rust_slice(env, this).to_owned();
    let b = to_rust_slice(env, other);
    a == b
}

@end

@implementation NSMutableData: NSData

+ (id)dataWithLength:(NSUInteger)length {
    let data: id = msg![env; this alloc];
    let data: id = msg![env; data initWithLength:length];
    autorelease(env, data)
}

- (id)initWithLength:(NSUInteger)length {
    let data: id = msg![env; this init];
    let _: () = msg![env; data setLength:length];
    data
}

- (MutVoidPtr)mutableBytes {
    env.objc.borrow_mut::<NSDataHostObject>(this).bytes
}

- (())setLength:(NSUInteger)length {
    let host_object = env.objc.borrow_mut::<NSDataHostObject>(this);
    if host_object.length == length {
        return;
    }

    if length == 0 {
        if !host_object.bytes.is_null() && host_object.free_when_done {
            env.mem.free(host_object.bytes);
        }
        host_object.bytes = Ptr::null();
        host_object.length = 0;
        return;
    }

    if host_object.bytes.is_null() {
        let alloc = env.mem.alloc(length);
        env.mem.memset(alloc, 0, length);
        host_object.bytes = alloc;
        host_object.length = length;
        host_object.free_when_done = true;
    } else {
        let alloc = env.mem.realloc(host_object.bytes, length);
        if length > host_object.length {
            let diff = length - host_object.length;
            let offset_ptr = alloc.offset(host_object.length as i32);
            env.mem.memset(offset_ptr, 0, diff);
        }
        host_object.bytes = alloc;
        host_object.length = length;
        host_object.free_when_done = true;
    }
}

- (())appendBytes:(ConstVoidPtr)bytes length:(NSUInteger)length {
    if length == 0 {
        return;
    }
    let host_object = env.objc.borrow_mut::<NSDataHostObject>(this);
    let old_length = host_object.length;
    let new_length = old_length + length;
    
    let _: () = msg![env; this setLength:new_length];
    
    // Borrow again after method call to satisfy borrow checker
    let host_object = env.objc.borrow::<NSDataHostObject>(this);
    let offset_ptr = host_object.bytes.offset(old_length as i32);
    env.mem.memmove(offset_ptr, bytes, length);
}

- (())appendData:(id)other {
    let bytes: ConstVoidPtr = msg![env; other bytes];
    let length: NSUInteger = msg![env; other length];
    msg![env; this appendBytes:bytes length:length]
}

@end

};

pub fn to_rust_slice(env: &mut Environment, data: id) -> &[u8] {
    let borrowed_data = env.objc.borrow::<NSDataHostObject>(data);
    if borrowed_data.length == 0 {
        return &[];
    }
    let casted_ptr: ConstPtr<u8> = borrowed_data.bytes.cast_const().cast();
    env.mem.bytes_at(casted_ptr, borrowed_data.length)
}

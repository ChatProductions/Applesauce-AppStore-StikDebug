/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//!
//! `CFURL`.
//!
//! This is toll-free bridged to `NSURL` in Apple's implementation. Here it is
//! the same type.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::CFIndex;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::cf_string::{
    kCFStringEncodingASCII, CFStringConvertEncodingToNSStringEncoding, CFStringEncoding,
    CFStringRef,
};
use crate::frameworks::foundation::ns_string::{
    get_static_str, to_rust_string, NSUTF8StringEncoding,
};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::{ConstPtr, MutPtr, Ptr};
use crate::objc::{id, msg, msg_class, release, retain};
use crate::Environment;

pub type CFURLRef = super::CFTypeRef;

type CFURLPathStyle = CFIndex;
const kCFURLPOSIXPathStyle: CFURLPathStyle = 0;
#[allow(dead_code)]
const kCFURLHFSPathStyle: CFURLPathStyle = 1;
#[allow(dead_code)]
const kCFURLWindowsPathStyle: CFURLPathStyle = 2;

pub fn CFURLGetFileSystemRepresentation(
    env: &mut Environment,
    url: CFURLRef,
    resolve_against_base: bool,
    buffer: MutPtr<u8>,
    buffer_size: CFIndex,
) -> bool {
    if resolve_against_base {
        // this function usually called to resolve resources from the main
        // bundle
        // thus, the url should already be an absolute path name
        // TODO: use absoluteURL instead once implemented
    
        let path = msg![env; url path];
        // TODO: avoid copy
        assert!(to_rust_string(env, path).starts_with('/'));
    }
    let buffer_size: NSUInteger = buffer_size.try_into().unwrap();

    msg![env; url getFileSystemRepresentation:buffer
                                    maxLength:buffer_size]
}

pub fn CFURLCreateFromFileSystemRepresentation(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    buffer: ConstPtr<u8>,
    buffer_size: CFIndex,
    is_directory: bool,
) -> CFURLRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default());
    // unimplemented

    let buffer_size: NSUInteger = buffer_size.try_into().unwrap();

    let string: id = msg_class![env; NSString alloc];
    let string: id = msg![env; string initWithBytes:buffer
                                             length:buffer_size
                                           encoding:NSUTF8StringEncoding];
    let url: id = msg_class![env; NSURL alloc];
    let res = msg![env; url initFileURLWithPath:string isDirectory:is_directory];
    release(env, string);
    res
}

fn CFURLCreateWithBytes(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    url_bytes: ConstPtr<u8>,
    length: CFIndex,
    encoding: CFStringEncoding,
    base_url: CFURLRef,
) -> CFURLRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default());
    // unimplemented
    assert_eq!(encoding, kCFStringEncodingASCII); // TODO
    assert!(base_url.is_null());
    // TODO

    // TODO: interpret percent escape sequences using encoding as well
    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    let length: NSUInteger = length.try_into().unwrap();

    if length == 0 {
        return Ptr::null();
    }

    let string: id = msg_class![env; NSString alloc];
    let string: id = msg![env; string initWithBytes:url_bytes
                                             length:length
                                           encoding:encoding];
    assert!(!to_rust_string(env, string).contains("://")); // TODO

    // Assume file URL case here
    let url: id = msg_class![env; NSURL alloc];
    let res = msg![env; url initFileURLWithPath:string];
    release(env, string);
    res
}

fn CFURLCreateWithFileSystemPath(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    file_path: CFStringRef,
    style: CFURLPathStyle,
    is_directory: bool,
) -> CFURLRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default());
    // unimplemented
    assert_eq!(style, kCFURLPOSIXPathStyle);
    let url: id = msg_class![env; NSURL alloc];
    msg![env; url initFileURLWithPath:file_path isDirectory:is_directory]
}

pub fn CFURLCopyPathExtension(env: &mut Environment, url: CFURLRef) -> CFStringRef {
    let path = msg![env; url path];
    let ext = msg![env; path pathExtension];
    msg![env; ext copy]
}

fn CFURLCopyFileSystemPath(
    env: &mut Environment,
    url: CFURLRef,
    style: CFURLPathStyle,
) -> CFStringRef {
    assert_eq!(style, kCFURLPOSIXPathStyle);
    let path: CFStringRef = msg![env; url path];
    msg![env; path copy]
}

fn CFURLCreateCopyAppendingPathComponent(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    url: CFURLRef,
    path_component: CFStringRef,
    is_directory: bool,
) -> CFURLRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default());
    let new_url =
        msg![env; url URLByAppendingPathComponent:path_component isDirectory:is_directory];
    msg![env; new_url copy]
}

fn CFURLCreateCopyDeletingLastPathComponent(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    url: CFURLRef,
) -> CFURLRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default());
    let new_url = msg![env; url URLByDeletingLastPathComponent];
    msg![env; new_url copy]
}

fn CFURLHasDirectoryPath(env: &mut Environment, url: CFURLRef) -> bool {
    assert!(!url.is_null());
    let path = msg![env; url path];
    if msg![env; path isEqual:(get_static_str(env, "//"))] {
        // Special case
        return false;
    }
    // Note: cannot use `lastPathComponent` here!
    let components: id = msg![env; path pathComponents];
    let count: NSUInteger = msg![env; components count];
    if count == 0 {
        return false;
    }
    let last: id = msg![env; components objectAtIndex:(count - 1)];
    msg![env; last isEqual:(get_static_str(env, "/"))]
        || msg![env; last isEqual:(get_static_str(env, "."))]
        || msg![env; last isEqual:(get_static_str(env, ".."))]
}

fn CFURLCreateStringByAddingPercentEscapes(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    original_string: CFStringRef,
    _characters_to_leave_unescaped: CFStringRef,
    _legal_url_characters_to_be_escaped: CFStringRef,
    _encoding: u32,
) -> CFStringRef {
    log_once!("Stubbed CFURLCreateStringByAddingPercentEscapes");
    // As a stub, we just return the original string.
    // Retain it because "Create" or "Copy" functions in CoreFoundation 
    // transfer ownership to the caller.
    retain(env, original_string)
}

fn CFURLCreateWithString(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    url_string: CFStringRef,
    base_url: CFURLRef,
) -> CFURLRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default());

    if url_string.is_null() {
        return Ptr::null();
    }

    let url: id = msg_class![env; NSURL alloc];
    
    if base_url.is_null() {
        msg![env; url initWithString:url_string]
    } else {
        msg![env; url initWithString:url_string relativeToURL:base_url]
    }
}

fn CFURLCopyAbsoluteURL(env: &mut Environment, url: CFURLRef) -> CFURLRef {
    if url.is_null() {
        return Ptr::null();
    }
    let abs_url: id = msg![env; url absoluteURL];
    retain(env, abs_url)
}

fn CFURLCopyScheme(env: &mut Environment, url: CFURLRef) -> CFStringRef {
    let scheme: id = msg![env; url scheme];
    if scheme.is_null() {
        return Ptr::null();
    }
    msg![env; scheme copy]
}

fn CFURLCopyNetLocation(env: &mut Environment, url: CFURLRef) -> CFStringRef {
    // In NSURL, this is roughly equivalent to the 'host' or 'resourceSpecifier' 
    // depending on the context. For CFURL, it usually refers to the host.
    let host: id = msg![env; url host];
    if host.is_null() {
        return Ptr::null();
    }
    msg![env; host copy]
}

fn CFURLGetPortNumber(env: &mut Environment, url: CFURLRef) -> i32 {
    let port: id = msg![env; url port];
    if port.is_null() {
        return -1; // Standard CFURL return for no port
    }
    // NSURL port returns an NSNumber
    let val: i32 = msg![env; port intValue];
    val
}

fn CFURLCopyResourcePropertyForKey(
    env: &mut Environment,
    url: CFURLRef,
    key: CFStringRef,
    property_ptr: MutPtr<super::CFTypeRef>,
    error: MutPtr<super::CFTypeRef>,
) -> bool {
    // This is a common pattern for checking file existence or sizes.
    // Note: This is a simplified bridge to getResourceValue:forKey:error:
    let mut err: id = Ptr::null();
    let mut value: id = Ptr::null();
    
    let success: bool = msg![env; url getResourceValue:&value forKey:key error:&err];
    
    if !property_ptr.is_null() {
        env.mem.write(property_ptr, value);
        if !value.is_null() {
            retain(env, value);
        }
    }
    
    if !error.is_null() {
        env.mem.write(error, err);
        if !err.is_null() {
            retain(env, err);
        }
    }
    
    success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFURLGetFileSystemRepresentation(_, _, _, _)),
    export_c_func!(CFURLCreateFromFileSystemRepresentation(_, _, _, _)),
    export_c_func!(CFURLCreateWithBytes(_, _, _, _, _)),
    export_c_func!(CFURLCreateWithFileSystemPath(_, _, _, _)),
    export_c_func!(CFURLCopyPathExtension(_)),
    export_c_func!(CFURLCopyFileSystemPath(_, _)),
    export_c_func!(CFURLCreateCopyAppendingPathComponent(_, _, _, _)),
    export_c_func!(CFURLCreateCopyDeletingLastPathComponent(_, _)),
    export_c_func!(CFURLHasDirectoryPath(_)),
    export_c_func!(CFURLCreateStringByAddingPercentEscapes(_, _, _, _, _)),
    export_c_func!(CFURLCreateWithString(_, _, _)),
    export_c_func!(CFURLCopyAbsoluteURL(_)),
    export_c_func!(CFURLCopyScheme(_)),
    export_c_func!(CFURLCopyNetLocation(_)),
    export_c_func!(CFURLGetPortNumber(_)),
    export_c_func!(CFURLCopyResourcePropertyForKey(_, _, _, _)),
];

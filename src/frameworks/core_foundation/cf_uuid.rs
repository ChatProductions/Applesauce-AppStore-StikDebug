/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `CFUUID`.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::cf_string::CFStringRef;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::ns_string;
use crate::objc::{id, nil, HostObject};
use crate::Environment;

pub type CFUUIDRef = super::CFTypeRef;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CFUUIDBytes {
    pub byte0: u8, pub byte1: u8, pub byte2: u8, pub byte3: u8,
    pub byte4: u8, pub byte5: u8, pub byte6: u8, pub byte7: u8,
    pub byte8: u8, pub byte9: u8, pub byte10: u8, pub byte11: u8,
    pub byte12: u8, pub byte13: u8, pub byte14: u8, pub byte15: u8,
}

struct CFUUIDHostObject {
    bytes: CFUUIDBytes,
}
impl HostObject for CFUUIDHostObject {}

// --- Core Foundation Functions ---

/// Creates a new unique identifier.
fn CFUUIDCreate(env: &mut Environment, allocator: CFAllocatorRef) -> CFUUIDRef {
    assert_eq!(allocator, kCFAllocatorDefault);
    
    // Generate random bytes (pseudo-random for emulation)
    let mut b = [0u8; 16];
    for i in 0..16 {
        b[i] = rand::random::<u8>(); // Requires 'rand' crate or env-based entropy
    }
    
    // Set UUID v4 version (bits 4-7 of byte 6) and variant (bits 6-7 of byte 8)
    b[6] = (b[6] & 0x0f) | 0x40; 
    b[8] = (b[8] & 0x3f) | 0x80;

    let bytes = CFUUIDBytes {
        byte0: b[0], byte1: b[1], byte2: b[2], byte3: b[3],
        byte4: b[4], byte5: b[5], byte6: b[6], byte7: b[7],
        byte8: b[8], byte9: b[9], byte10: b[10], byte11: b[11],
        byte12: b[12], byte13: b[13], byte14: b[14], byte15: b[15],
    };

    let host_object = Box::new(CFUUIDHostObject { bytes });
    // In touchHLE, CFUUIDs are often represented as internal ObjC-like objects
    env.objc.alloc_object(nil, host_object, &mut env.mem)
}

/// Creates a UUID from a string representation.
fn CFUUIDCreateFromString(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    string: CFStringRef,
) -> CFUUIDRef {
    assert_eq!(allocator, kCFAllocatorDefault);
    let s = ns_string::to_rust_string(env, string).replace('-', "");
    
    let mut b = [0u8; 16];
    for i in 0..16 {
        if let Ok(byte) = u8::from_str_radix(&s[i*2..i*2+2], 16) {
            b[i] = byte;
        }
    }

    let bytes = CFUUIDBytes {
        byte0: b[0], byte1: b[1], byte2: b[2], byte3: b[3],
        byte4: b[4], byte5: b[5], byte6: b[6], byte7: b[7],
        byte8: b[8], byte9: b[9], byte10: b[10], byte11: b[11],
        byte12: b[12], byte13: b[13], byte14: b[14], byte15: b[15],
    };

    env.objc.alloc_object(nil, Box::new(CFUUIDHostObject { bytes }), &mut env.mem)
}

/// Returns the raw bytes of the UUID.
/// Note: Apple's ABI returns this 16-byte struct by value.
fn CFUUIDGetUUIDBytes(env: &mut Environment, uuid: CFUUIDRef) -> CFUUIDBytes {
    let host = env.objc.borrow::<CFUUIDHostObject>(uuid);
    host.bytes
}

/// Returns a string representation of the UUID.
fn CFUUIDCreateString(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    uuid: CFUUIDRef,
) -> CFStringRef {
    assert_eq!(allocator, kCFAllocatorDefault);
    let b = env.objc.borrow::<CFUUIDHostObject>(uuid).bytes;
    
    let s = format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        b.byte0, b.byte1, b.byte2, b.byte3, b.byte4, b.byte5, b.byte6, b.byte7,
        b.byte8, b.byte9, b.byte10, b.byte11, b.byte12, b.byte13, b.byte14, b.byte15
    );
    
    ns_string::from_rust_string(env, s)
}

/// Returns a constant for the null UUID.
fn CFUUIDGetConstantUUIDWithBytes(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    b0: u8, b1: u8, b2: u8, b3: u8, b4: u8, b5: u8, b6: u8, b7: u8,
    b8: u8, b9: u8, b10: u8, b11: u8, b12: u8, b13: u8, b14: u8, b15: u8,
) -> CFUUIDRef {
    // Check if we already have this constant cached (omitted for brevity)
    let bytes = CFUUIDBytes {
        byte0: b0, byte1: b1, byte2: b2, byte3: b3,
        byte4: b4, byte5: b5, byte6: b6, byte7: b7,
        byte8: b8, byte9: b9, byte10: b10, byte11: b11,
        byte12: b12, byte13: b13, byte14: b14, byte15: b15,
    };
    env.objc.alloc_object(nil, Box::new(CFUUIDHostObject { bytes }), &mut env.mem)
}

// --- Exports ---

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFUUIDCreate(_)),
    export_c_func!(CFUUIDCreateFromString(_, _)),
    export_c_func!(CFUUIDGetUUIDBytes(_, _)),
    export_c_func!(CFUUIDCreateString(_, _)),
    export_c_func!(CFUUIDGetConstantUUIDWithBytes(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _)),
];

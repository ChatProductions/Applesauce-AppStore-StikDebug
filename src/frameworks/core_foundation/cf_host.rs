/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(dead_code)]
//! `CFHost` — CFNetwork host resolution stub.

use crate::abi::GuestFunction;
use crate::frameworks::core_foundation::cf_allocator::CFAllocatorRef;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutVoidPtr};
use crate::objc::{nil, objc_classes, ClassExports, HostObject};
use crate::Environment;
use std::net::SocketAddrV4;

pub type CFHostRef = CFTypeRef;

type CFHostInfoType = i32;
const kCFHostAddresses:    CFHostInfoType = 0;
const kCFHostNames:        CFHostInfoType = 1;
const kCFHostReachability: CFHostInfoType = 2;

// CFStreamError — simple stub struct (domain + error)
type CFStreamError = u64; // two i32s packed; we never inspect it

pub(crate) struct CFHostHostObject {
    pub(crate) address: Option<SocketAddrV4>,
    pub(crate) name: Option<String>,
    pub(crate) callout: Option<GuestFunction>,
    pub(crate) context: MutVoidPtr,
}

impl HostObject for CFHostHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// Fix dealloc — no release needed, fields are plain Rust values:
@implementation _touchHLE_CFHost: NSObject
- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}
@end

};

// MARK: - Internal helpers

fn alloc_cfhost(
    env: &mut Environment,
    name: Option<String>,
    address: Option<SocketAddrV4>,
) -> CFHostRef {
    let class = env.objc.get_known_class("_touchHLE_CFHost", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CFHostHostObject {
            name,
            address,
            callout: None,
            context: MutVoidPtr::null(),
        }),
        &mut env.mem,
    )
}

// MARK: - Lifecycle

pub fn CFHostRetain(env: &mut Environment, host: CFHostRef) -> CFHostRef {
    if !host.is_null() { CFRetain(env, host) } else { host }
}

pub fn CFHostRelease(env: &mut Environment, host: CFHostRef) {
    if !host.is_null() { CFRelease(env, host); }
}

// MARK: - Constructors

fn CFHostCreateWithName(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    hostname: CFTypeRef, // CFStringRef
) -> CFHostRef {
    let name_str = ns_string::to_rust_string(env, hostname).into_owned();
    log_dbg!("CFHostCreateWithName: {}", name_str);
    alloc_cfhost(env, Some(name_str), None)
}

fn CFHostCreateWithAddress(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    addr: CFTypeRef, // CFDataRef
) -> CFHostRef {
    log_dbg!("CFHostCreateWithAddress: stubbed");
    // We don't parse the sockaddr bytes — store None for address.
    alloc_cfhost(env, None, None)
}

fn CFHostCreateCopy(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    host: CFHostRef,
) -> CFHostRef {
    if host.is_null() {
        return nil;
    }
    let h = env.objc.borrow::<CFHostHostObject>(host);
    let name    = h.name.clone();
    let address = h.address;
    drop(h);
    alloc_cfhost(env, name, address)
}

fn CFHostStartInfoResolution(
    env: &mut Environment,
    host: CFHostRef,
    info: CFHostInfoType,
    _error: MutVoidPtr,
) -> bool {
    let name_str = env
        .objc
        .borrow::<CFHostHostObject>(host)
        .name
        .clone()
        .unwrap_or_else(|| "<address>".to_string());
    log!(
        "CFHostStartInfoResolution: host={} info={} — stubbed, returning false",
        name_str, info
    );
    false
}

fn CFHostCancelInfoResolution(
    _env: &mut Environment,
    _host: CFHostRef,
    _info: CFHostInfoType,
) {
    log!("CFHostCancelInfoResolution: stubbed");
}

fn CFHostScheduleWithRunLoop(
    _env: &mut Environment,
    _host: CFHostRef,
    _run_loop: CFTypeRef,
    _run_loop_mode: CFTypeRef,
) {
    log!("CFHostScheduleWithRunLoop: stubbed");
}

fn CFHostUnscheduleFromRunLoop(
    _env: &mut Environment,
    _host: CFHostRef,
    _run_loop: CFTypeRef,
    _run_loop_mode: CFTypeRef,
) {
    log!("CFHostUnscheduleFromRunLoop: stubbed");
}

fn CFHostSetClient(
    _env: &mut Environment,
    _host: CFHostRef,
    _client_cb: MutVoidPtr,  // CFHostClientCallBack
    _client_ctx: MutVoidPtr, // CFHostClientContext*
) -> bool {
    log!("CFHostSetClient: stubbed, returning false");
    false
}

// MARK: - Info accessors (always return nil / false)

fn CFHostGetAddressing(
    _env: &mut Environment,
    _host: CFHostRef,
    _has_been_resolved: MutVoidPtr, // Boolean*
) -> CFTypeRef {
    // Returns CFArrayRef of CFDataRef addresses — nil since we never resolve.
    nil
}

fn CFHostGetNames(
    _env: &mut Environment,
    _host: CFHostRef,
    _has_been_resolved: MutVoidPtr, // Boolean*
) -> CFTypeRef {
    // Returns CFArrayRef of CFStringRef names.
    nil
}

fn CFHostGetReachability(
    _env: &mut Environment,
    _host: CFHostRef,
    _has_been_resolved: MutVoidPtr, // Boolean*
) -> CFTypeRef {
    // Returns CFDataRef reachability flags.
    nil
}

fn CFHostIsInfoResolved(
    _env: &mut Environment,
    _host: CFHostRef,
    _info: CFHostInfoType,
) -> bool {
    false
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFHostRetain(_)),
    export_c_func!(CFHostRelease(_)),
    export_c_func!(CFHostCreateWithName(_, _)),
    export_c_func!(CFHostCreateWithAddress(_, _)),
    export_c_func!(CFHostCreateCopy(_, _)),
    export_c_func!(CFHostStartInfoResolution(_, _, _)),
    export_c_func!(CFHostCancelInfoResolution(_, _)),
    export_c_func!(CFHostScheduleWithRunLoop(_, _, _)),
    export_c_func!(CFHostUnscheduleFromRunLoop(_, _, _)),
    export_c_func!(CFHostSetClient(_, _, _)),
    export_c_func!(CFHostGetAddressing(_, _)),
    export_c_func!(CFHostGetNames(_, _)),
    export_c_func!(CFHostGetReachability(_, _)),
    export_c_func!(CFHostIsInfoResolved(_, _)),
];


/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(dead_code)]
//! SCNetworkReachability

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::cf_allocator::CFAllocatorRef;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::libc::sys::socket::sockaddr;
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr, Ptr};
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;
use std::net::SocketAddrV4;

// SCNetworkReachabilityFlags
type SCNetworkReachabilityFlags = u32;
const kSCNetworkReachabilityFlagsTransientConnection:  SCNetworkReachabilityFlags = 1 << 0;
const kSCNetworkReachabilityFlagsReachable:            SCNetworkReachabilityFlags = 1 << 1;
const kSCNetworkReachabilityFlagsConnectionRequired:   SCNetworkReachabilityFlags = 1 << 2;
const kSCNetworkReachabilityFlagsConnectionOnTraffic:  SCNetworkReachabilityFlags = 1 << 3;
const kSCNetworkReachabilityFlagsInterventionRequired: SCNetworkReachabilityFlags = 1 << 4;
const kSCNetworkReachabilityFlagsConnectionOnDemand:   SCNetworkReachabilityFlags = 1 << 5;
const kSCNetworkReachabilityFlagsIsLocalAddress:       SCNetworkReachabilityFlags = 1 << 16;
const kSCNetworkReachabilityFlagsIsDirect:             SCNetworkReachabilityFlags = 1 << 17;
// iOS-only
const kSCNetworkReachabilityFlagsIsWWAN:               SCNetworkReachabilityFlags = 1 << 18;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_SCNetworkReachability: NSObject
- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}
@end

};

struct SCNetworkReachabilityHostObject {
    address: Option<SocketAddrV4>,
    /// Hostname passed to CreateWithName (for logging / future DNS).
    name: Option<String>,
    /// Guest callback registered via SetCallback.
    callout: Option<GuestFunction>,
    /// Opaque client data for the callback.
    context: MutVoidPtr,
}
impl HostObject for SCNetworkReachabilityHostObject {}

type SCNetworkReachabilityRef = CFTypeRef;

// MARK: - Retain / Release

pub fn SCNetworkReachabilityRetain(
    env: &mut Environment,
    target: SCNetworkReachabilityRef,
) -> SCNetworkReachabilityRef {
    if !target.is_null() { CFRetain(env, target) } else { target }
}

pub fn SCNetworkReachabilityRelease(env: &mut Environment, target: SCNetworkReachabilityRef) {
    if !target.is_null() { CFRelease(env, target); }
}

// MARK: - Constructors

fn SCNetworkReachabilityCreateWithName(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    name: ConstPtr<u8>,
) -> SCNetworkReachabilityRef {
    let name_str = env.mem.cstr_at_utf8(name).unwrap_or("").to_string();

    // Game-specific hack for Cut the Rope.
    if env
        .bundle
        .bundle_identifier()
        .starts_with("com.chillingo.cuttherope")
        && name_str == "chillingo-crystal.appspot.com"
    {
        log!("SCNetworkReachabilityCreateWithName: Cut the Rope hack — returning NULL");
        return Ptr::null();
    }

    let isa = env
        .objc
        .get_known_class("_touchHLE_SCNetworkReachability", &mut env.mem);

    let res = env.objc.alloc_object(
        isa,
        Box::new(SCNetworkReachabilityHostObject {
            address: None,
            name: Some(name_str.clone()),
            callout: None,
            context: MutVoidPtr::null(),
        }),
        &mut env.mem,
    );

    log!("SCNetworkReachabilityCreateWithName({:?}) -> {:?}", name_str, res);
    res
}

fn SCNetworkReachabilityCreateWithAddress(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    address: ConstPtr<sockaddr>,
) -> SCNetworkReachabilityRef {
    let address_val = env.mem.read(address);
    let sock_addr   = address_val.to_sockaddr_v4();

    let isa = env
        .objc
        .get_known_class("_touchHLE_SCNetworkReachability", &mut env.mem);

    let res = env.objc.alloc_object(
        isa,
        Box::new(SCNetworkReachabilityHostObject {
            address: Some(sock_addr),
            name: None,
            callout: None,
            context: MutVoidPtr::null(),
        }),
        &mut env.mem,
    );

    log_dbg!(
        "SCNetworkReachabilityCreateWithAddress({}) -> {:?}",
        sock_addr, res
    );

    res
}

fn SCNetworkReachabilityCreateWithAddressPair(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    local_address: ConstPtr<sockaddr>,
    remote_address: ConstPtr<sockaddr>,
) -> SCNetworkReachabilityRef {
    // Use the remote address as primary; local is informational only.
    let addr = if !remote_address.is_null() {
        remote_address
    } else {
        local_address
    };

    if addr.is_null() {
        log!("SCNetworkReachabilityCreateWithAddressPair: both addresses null, returning NULL");
        return Ptr::null();
    }

    let address_val = env.mem.read(addr);
    let sock_addr   = address_val.to_sockaddr_v4();

    let isa = env
        .objc
        .get_known_class("_touchHLE_SCNetworkReachability", &mut env.mem);

    let res = env.objc.alloc_object(
        isa,
        Box::new(SCNetworkReachabilityHostObject {
            address: Some(sock_addr),
            name: None,
            callout: None,
            context: MutVoidPtr::null(),
        }),
        &mut env.mem,
    );

    log_dbg!("SCNetworkReachabilityCreateWithAddressPair({}) -> {:?}", sock_addr, res);
    res
}

// MARK: - Flags

fn SCNetworkReachabilityGetFlags(
    env: &mut Environment,
    target: SCNetworkReachabilityRef,
    flags: MutPtr<SCNetworkReachabilityFlags>,
) -> bool {
    if !env.options.network_access {
        log_dbg!("SCNetworkReachabilityGetFlags: network disabled -> false");
        return false;
    }

    let host_object = env.objc.borrow::<SCNetworkReachabilityHostObject>(target);
    let address     = host_object.address;
    let name        = host_object.name.clone();
    drop(host_object);

    if let Some(addr) = address {
        if addr.ip().is_link_local() {
            // Local WiFi.
            let out = kSCNetworkReachabilityFlagsReachable
                    | kSCNetworkReachabilityFlagsIsDirect;
            env.mem.write(flags, out);
            log_dbg!("SCNetworkReachabilityGetFlags: link-local -> {:08x}", out);
            return true;
        }
        if addr.ip().is_loopback() || addr.ip().is_unspecified() {
            let out = kSCNetworkReachabilityFlagsReachable
                    | kSCNetworkReachabilityFlagsIsLocalAddress;
            env.mem.write(flags, out);
            log_dbg!("SCNetworkReachabilityGetFlags: loopback -> {:08x}", out);
            return true;
        }
    }

    // For named hosts: claim reachable so apps don't disable networking.
    if name.is_some() {
        let out = kSCNetworkReachabilityFlagsReachable;
        env.mem.write(flags, out);
        log_dbg!(
            "SCNetworkReachabilityGetFlags: named host {:?} -> {:08x}",
            name, out
        );
        return true;
    }

    log!("SCNetworkReachabilityGetFlags: unresolved target -> false");
    false
}

// MARK: - Callback

fn SCNetworkReachabilitySetCallback(
    env: &mut Environment,
    target: SCNetworkReachabilityRef,
    callout: GuestFunction,
    context: MutVoidPtr,
) -> bool {
    log!(
        "SCNetworkReachabilitySetCallback({:?}, {:?}, {:?}) stubbed -> false",
        target, callout, context
    );

    let host = env.objc.borrow_mut::<SCNetworkReachabilityHostObject>(target);
    host.callout = Some(callout);
    host.context = context;

    // Return false — callback will never fire since we don't run a real
    // network stack.
    false
}

// MARK: - Run loop scheduling

fn SCNetworkReachabilityScheduleWithRunLoop(
    _env: &mut Environment,
    _target: SCNetworkReachabilityRef,
    _run_loop: CFTypeRef,
    _run_loop_mode: CFTypeRef,
) -> bool {
    log!("SCNetworkReachabilityScheduleWithRunLoop: stubbed -> false");
    false
}

fn SCNetworkReachabilityUnscheduleFromRunLoop(
    _env: &mut Environment,
    _target: SCNetworkReachabilityRef,
    _run_loop: CFTypeRef,
    _run_loop_mode: CFTypeRef,
) -> bool {
    log!("SCNetworkReachabilityUnscheduleFromRunLoop: stubbed -> false");
    false
}

fn SCNetworkReachabilitySetDispatchQueue(
    _env: &mut Environment,
    _target: SCNetworkReachabilityRef,
    _queue: MutVoidPtr, // dispatch_queue_t
) -> bool {
    log!("SCNetworkReachabilitySetDispatchQueue: stubbed -> false");
    false
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(SCNetworkReachabilityRetain(_)),
    export_c_func!(SCNetworkReachabilityRelease(_)),
    export_c_func!(SCNetworkReachabilityCreateWithName(_, _)),
    export_c_func!(SCNetworkReachabilityCreateWithAddress(_, _)),
    export_c_func!(SCNetworkReachabilityCreateWithAddressPair(_, _, _)),
    export_c_func!(SCNetworkReachabilityGetFlags(_, _)),
    export_c_func!(SCNetworkReachabilitySetCallback(_, _, _)),
    export_c_func!(SCNetworkReachabilityScheduleWithRunLoop(_, _, _)),
    export_c_func!(SCNetworkReachabilityUnscheduleFromRunLoop(_, _, _)),
    export_c_func!(SCNetworkReachabilitySetDispatchQueue(_, _)),
];


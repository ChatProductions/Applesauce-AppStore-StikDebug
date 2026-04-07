/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `netdb.h` — host/service name resolution stubs.

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::libc::sys::socket::{sockaddr, AF_INET, SOCK_DGRAM, SOCK_STREAM};
use crate::mem::{guest_size_of, ConstPtr, MutPtr, Ptr, SafeRead};
use crate::Environment;

const AI_PASSIVE: i32 = 0x1;

pub const IPPROTO_TCP: i32 = 6;
pub const IPPROTO_UDP: i32 = 17;

const EAI_AGAIN:   i32 = 2;
const EAI_FAIL:    i32 = 4;
const EAI_FAMILY:  i32 = 5;
const EAI_SERVICE: i32 = 8;
#[allow(dead_code)]
const EAI_SYSTEM:  i32 = 11;

#[allow(dead_code)]
const HOST_NOT_FOUND: i32 = 1;
#[allow(dead_code)]
const NO_RECOVERY:    i32 = 3;

#[derive(Default)]
pub struct State {
    /// Raw guest address of the persistent dummy `hostent` block.
    /// Zero means not yet allocated.
    pub dummy_hostent_ptr: u32,
}

#[allow(non_camel_case_types)]
pub type socklen_t = u32;

#[allow(non_camel_case_types)]
struct hostent {}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
#[allow(non_camel_case_types)]
pub struct addrinfo {
    ai_flags:     i32,
    ai_family:    i32,
    ai_socktype:  i32,
    ai_protocol:  i32,
    ai_addrlen:   socklen_t,
    ai_canonname: MutPtr<u8>,
    ai_addr:      MutPtr<sockaddr>,
    ai_next:      MutPtr<addrinfo>,
}
unsafe impl SafeRead for addrinfo {}

fn getaddrinfo(
    env: &mut Environment,
    node_name: MutPtr<u8>,
    serv_name: MutPtr<u8>,
    hints: ConstPtr<addrinfo>,
    res: MutPtr<MutPtr<addrinfo>>,
) -> i32 {
    if !env.options.network_access {
        log_dbg!(
            "getaddrinfo: network access disabled (node={:?} serv={:?}) -> EAI_FAIL",
            node_name, serv_name,
        );
        return EAI_FAIL;
    }

    let hint = if hints.is_null() {
        addrinfo {
            ai_flags: 0, ai_family: 0, ai_socktype: 0, ai_protocol: 0,
            ai_addrlen: 0,
            ai_canonname: Ptr::null(), ai_addr: Ptr::null(), ai_next: Ptr::null(),
        }
    } else {
        env.mem.read(hints)
    };

    // Copy fields out of the packed struct before comparing — avoids
    // unaligned reference UB on ARM targets.
    let ai_flags    = hint.ai_flags;
    let ai_family   = hint.ai_family;
    let ai_socktype = hint.ai_socktype;
    let ai_protocol = hint.ai_protocol;

    if ai_family != AF_INET && ai_family != 0 {
        log!("getaddrinfo: unsupported ai_family {} -> EAI_FAMILY", ai_family);
        return EAI_FAMILY;
    }
    if ai_socktype != 0 && ai_socktype != SOCK_STREAM && ai_socktype != SOCK_DGRAM {
        log!("getaddrinfo: unsupported ai_socktype {} -> EAI_SERVICE", ai_socktype);
        return EAI_SERVICE;
    }
    if ai_protocol != 0 && ai_protocol != IPPROTO_TCP && ai_protocol != IPPROTO_UDP {
        log!("getaddrinfo: unsupported ai_protocol {} -> EAI_FAIL", ai_protocol);
        return EAI_FAIL;
    }

    let ip_octets: [u8; 4] = if node_name.is_null() {
        [0u8; 4]
    } else {
        let hostname = env.mem.cstr_at_utf8(node_name).unwrap_or_default().to_owned();
        if let Ok(addr) = hostname.parse::<std::net::Ipv4Addr>() {
            addr.octets()
        } else {
            log!("getaddrinfo: hostname \"{}\" not resolvable -> EAI_FAIL", hostname);
            return EAI_FAIL;
        }
    };

    let port: u16 = if serv_name.is_null() {
        0
    } else {
        let svc = env.mem.cstr_at_utf8(serv_name).unwrap_or_default().to_owned();
        match svc.parse::<u16>() {
            Ok(p) => p,
            Err(_) => match svc.as_str() {
                "http"  => 80,
                "https" => 443,
                "ftp"   => 21,
                "smtp"  => 25,
                "pop3"  => 110,
                "imap"  => 143,
                _ => {
                    log!("getaddrinfo: named service \"{}\" not supported -> EAI_SERVICE", svc);
                    return EAI_SERVICE;
                }
            },
        }
    };

    log_dbg!("getaddrinfo: ip={:?} port={}", ip_octets, port);

    let addr_ptr = env.mem.alloc_and_write(sockaddr::from_ipv4_parts(ip_octets, port));
    let result = addrinfo {
        ai_flags:     ai_flags,
        ai_family:    AF_INET,
        ai_socktype:  if ai_socktype != 0 { ai_socktype } else { SOCK_STREAM },
        ai_protocol:  if ai_protocol != 0 { ai_protocol } else { IPPROTO_TCP },
        ai_addrlen:   guest_size_of::<sockaddr>(),
        ai_canonname: Ptr::null(),
        ai_addr:      addr_ptr,
        ai_next:      Ptr::null(),
    };
    let result_ptr = env.mem.alloc_and_write(result);
    if !res.is_null() {
        env.mem.write(res, result_ptr);
    }
    0
}

fn freeaddrinfo(env: &mut Environment, ai: MutPtr<addrinfo>) {
    if ai.is_null() { return; }
    let mut cur = ai;
    while !cur.is_null() {
        let node         = env.mem.read(cur);
        let next         = node.ai_next;
        let ai_addr      = node.ai_addr;
        let ai_canonname = node.ai_canonname;
        if !ai_addr.is_null()      { env.mem.free(ai_addr.cast());      }
        if !ai_canonname.is_null() { env.mem.free(ai_canonname.cast()); }
        env.mem.free(cur.cast());
        cur = next;
    }
}

fn gethostbyname(env: &mut Environment, name: ConstPtr<u8>) -> MutPtr<hostent> {
    let hostname = if name.is_null() {
        "<null>".to_string()
    } else {
        env.mem.cstr_at_utf8(name).unwrap_or_default().to_owned()
    };

    // Lazy-init a persistent zeroed 64-byte dummy hostent.
    if env.libc_state.netdb.dummy_hostent_ptr == 0 {
        const DUMMY_SIZE: u32 = 64;

        let void_ptr = env.mem.alloc(DUMMY_SIZE);

        // Cast once to a byte pointer
        let byte_ptr = void_ptr.cast::<u8>();

        // Zero memory safely using pointer arithmetic
        for i in 0..DUMMY_SIZE {
            env.mem.write(byte_ptr.add(i), 0u8);
        }

        // Store raw address (since your state uses u32)
        env.libc_state.netdb.dummy_hostent_ptr = void_ptr.to_bits();
    }

    let dummy_addr = env.libc_state.netdb.dummy_hostent_ptr;

    log!(
        "gethostbyname(\"{}\") — networking not implemented, \
         returning zeroed dummy hostent at 0x{:08x}",
        hostname, dummy_addr,
    );

    MutPtr::from_bits(dummy_addr)
}

fn gai_strerror(env: &mut Environment, ecode: i32) -> ConstPtr<u8> {
    let msg: &[u8] = match ecode {
        0           => b"Success",
        2           => b"Temporary failure in name resolution",
        EAI_FAIL    => b"Non-recoverable failure in name resolution",
        EAI_FAMILY  => b"ai_family not supported",
        EAI_SERVICE => b"Servname not supported for ai_socktype",
        _           => b"Unknown error",
    };
    env.mem.alloc_and_write_cstr(msg).cast_const()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(getaddrinfo(_, _, _, _)),
    export_c_func!(freeaddrinfo(_)),
    export_c_func!(gethostbyname(_)),
    export_c_func!(gai_strerror(_)),
];

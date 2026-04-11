/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `arpa/inet.h` (Internet address manipulation routines)

use crate::dyld::FunctionExports;
use crate::libc::netdb::socklen_t;
use crate::libc::sys::socket::AF_INET;
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr, MutVoidPtr, SafeRead};
use crate::{export_c_func, Environment};
use std::net::Ipv4Addr;

#[allow(non_camel_case_types)]
type in_addr_t = u32;

const INADDR_NONE: in_addr_t = u32::MAX;
const INADDR_ANY: in_addr_t = 0x00000000;
const INADDR_BROADCAST: in_addr_t = 0xFFFFFFFF;
// 127.0.0.1 in host byte order
const INADDR_LOOPBACK: in_addr_t = 0x7F000001;

/// AF_INET6 \u2014 we don't support IPv6 but accept the constant gracefully.
const AF_INET6: i32 = 30;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
#[allow(non_camel_case_types)]
struct in_addr {
    s_addr: in_addr_t,
}
unsafe impl SafeRead for in_addr {}

// MARK: - inet_addr / inet_aton

fn inet_addr(env: &mut Environment, str: ConstPtr<u8>) -> in_addr_t {
    let s = env.mem.cstr_at_utf8(str).unwrap_or_default().to_owned();
    match s.parse::<Ipv4Addr>() {
        Ok(addr) => {
            let res = u32::from_le_bytes(addr.octets());
            log_dbg!("inet_addr({:?}) => {:#010x}", s, res);
            res
        }
        Err(_) => {
            log!("inet_addr({:?}): invalid address, returning INADDR_NONE", s);
            INADDR_NONE
        }
    }
}

/// `inet_aton` \u2014 converts a dotted-decimal string into an `in_addr`.
/// Returns 1 on success, 0 on failure (opposite of `inet_addr` convention).
fn inet_aton(
    env: &mut Environment,
    str: ConstPtr<u8>,
    addr_out: MutPtr<in_addr>,
) -> i32 {
    let s = env.mem.cstr_at_utf8(str).unwrap_or_default().to_owned();
    match s.parse::<Ipv4Addr>() {
        Ok(addr) => {
            if !addr_out.is_null() {
                env.mem.write(
                    addr_out,
                    in_addr {
                        s_addr: u32::from_le_bytes(addr.octets()),
                    },
                );
            }
            log_dbg!("inet_aton({:?}) => 1", s);
            1
        }
        Err(_) => {
            log!("inet_aton({:?}): invalid address, returning 0", s);
            0
        }
    }
}

// MARK: - inet_ntoa

/// `inet_ntoa` \u2014 converts an `in_addr` (passed as raw u32) to a
/// dotted-decimal string.  Returns a pointer to a per-call guest buffer.
fn inet_ntoa(env: &mut Environment, s_addr: in_addr_t) -> ConstPtr<u8> {
    let octets = s_addr.to_le_bytes();
    let s = format!(
        "{}.{}.{}.{}",
        octets[0], octets[1], octets[2], octets[3]
    );
    log_dbg!("inet_ntoa({:#010x}) => {:?}", s_addr, s);
    let bytes = s.as_bytes();
    // Allocate a fresh buffer each call (leaked \u2014 tiny and rare per real usage).
    let buf: MutPtr<u8> = env.mem.alloc(bytes.len() as u32 + 1).cast();
    env.mem
        .bytes_at_mut(buf, bytes.len() as u32)
        .copy_from_slice(bytes);
    env.mem.write(buf + bytes.len() as u32, b'\0');
    buf.cast_const()
}

// MARK: - inet_ntop / inet_pton

fn inet_ntop(
    env: &mut Environment,
    af: i32,
    src: ConstVoidPtr,
    dst: MutPtr<u8>,
    size: socklen_t,
) -> ConstPtr<u8> {
    match af {
        x if x == AF_INET => {
            let addr = env.mem.read(src.cast::<in_addr>());
            let octets = addr.s_addr.to_le_bytes();
            let s = format!(
                "{}.{}.{}.{}",
                octets[0], octets[1], octets[2], octets[3]
            );
            log_dbg!("inet_ntop AF_INET => {:?}", s);
            let bytes = s.as_bytes();
            let len = bytes.len() as u32;
            if len + 1 > size {
                log!("inet_ntop: buffer too small ({} < {})", size, len + 1);
                return ConstPtr::null();
            }
            env.mem.bytes_at_mut(dst, len).copy_from_slice(bytes);
            env.mem.write(dst + len, b'\0');
            dst.cast_const()
        }
        x if x == AF_INET6 => {
            log!("inet_ntop: AF_INET6 not supported, writing '::' stub");
            let s = b"::\0";
            if (s.len() as u32) > size {
                return ConstPtr::null();
            }
            env.mem
                .bytes_at_mut(dst, s.len() as u32)
                .copy_from_slice(s);
            dst.cast_const()
        }
        _ => {
            log!("inet_ntop: unsupported address family {}", af);
            ConstPtr::null()
        }
    }
}

fn inet_pton(
    env: &mut Environment,
    af: i32,
    src: ConstPtr<u8>,
    dst: MutVoidPtr,
) -> i32 {
    let s = env.mem.cstr_at_utf8(src).unwrap_or_default().to_owned();
    match af {
        x if x == AF_INET => match s.parse::<Ipv4Addr>() {
            Ok(addr) => {
                let ia = in_addr {
                    s_addr: u32::from_le_bytes(addr.octets()),
                };
                env.mem.write(dst.cast::<in_addr>(), ia);
                log_dbg!("inet_pton AF_INET {:?} => 1", s);
                1
            }
            Err(_) => {
                log!("inet_pton AF_INET {:?}: invalid, returning 0", s);
                0
            }
        },
        x if x == AF_INET6 => {
            log!("inet_pton: AF_INET6 not supported, returning 0");
            0
        }
        _ => {
            log!(
                "inet_pton: unsupported address family {}, returning -1",
                af
            );
            -1 // EAFNOSUPPORT
        }
    }
}

// MARK: - Byte order (htonl / htons / ntohl / ntohs)

/// `htonl` \u2014 host to network byte order (32-bit).
fn htonl(_env: &mut Environment, hostlong: u32) -> u32 {
    hostlong.to_be()
}

/// `htons` \u2014 host to network byte order (16-bit).
fn htons(_env: &mut Environment, hostshort: u16) -> u16 {
    hostshort.to_be()
}

/// `ntohl` \u2014 network to host byte order (32-bit).
fn ntohl(_env: &mut Environment, netlong: u32) -> u32 {
    u32::from_be(netlong)
}

/// `ntohs` \u2014 network to host byte order (16-bit).
fn ntohs(_env: &mut Environment, netshort: u16) -> u16 {
    u16::from_be(netshort)
}

// MARK: - inet_lnaof / inet_netof / inet_makeaddr / inet_network

/// `inet_lnaof` \u2014 extract host portion of a Class A/B/C address.
/// Accepts the raw `s_addr` field (u32) rather than the struct, because
/// `in_addr` does not implement `GuestArg`.
fn inet_lnaof(_env: &mut Environment, s_addr: in_addr_t) -> in_addr_t {
    let a = u32::from_be(s_addr);
    // Class A: top bit 0 -> 24-bit host part
    // Class B: top 2 bits 10 -> 16-bit host part
    // Class C: top 3 bits 110 -> 8-bit host part
    if a & 0x8000_0000 == 0 {
        a & 0x00FF_FFFF
    } else if a & 0x4000_0000 == 0 {
        a & 0x0000_FFFF
    } else {
        a & 0x0000_00FF
    }
}

/// `inet_netof` \u2014 extract network portion of a classful address.
fn inet_netof(_env: &mut Environment, s_addr: in_addr_t) -> in_addr_t {
    let a = u32::from_be(s_addr);
    if a & 0x8000_0000 == 0 {
        (a >> 24) & 0xFF
    } else if a & 0x4000_0000 == 0 {
        (a >> 16) & 0xFFFF
    } else {
        (a >> 8) & 0xFF_FFFF
    }
}

/// `inet_makeaddr` \u2014 construct an `in_addr` from a network number and host
/// part. Returns the raw `s_addr` value (u32) rather than the struct, because
/// `in_addr` does not implement `GuestRet`.
fn inet_makeaddr(_env: &mut Environment, net: u32, host: u32) -> in_addr_t {
    let addr = if net < 128 {
        (net << 24) | (host & 0x00FF_FFFF)
    } else if net < 65536 {
        (net << 16) | (host & 0x0000_FFFF)
    } else {
        (net << 8) | (host & 0x0000_00FF)
    };
    addr.to_be()
}

/// `inet_network` \u2014 parse a network address in the form `a.b.c.d` and return
/// it in *host* byte order (unlike `inet_addr` which returns network order).
fn inet_network(env: &mut Environment, str: ConstPtr<u8>) -> in_addr_t {
    let s = env.mem.cstr_at_utf8(str).unwrap_or_default().to_owned();
    match s.parse::<Ipv4Addr>() {
        Ok(addr) => {
            let val = u32::from_be_bytes(addr.octets()); // host byte order
            log_dbg!("inet_network({:?}) => {:#010x}", s, val);
            val
        }
        Err(_) => {
            log!("inet_network({:?}): invalid, returning INADDR_NONE", s);
            INADDR_NONE
        }
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(inet_addr(_)),
    export_c_func!(inet_aton(_, _)),
    export_c_func!(inet_ntoa(_)),
    export_c_func!(inet_ntop(_, _, _, _)),
    export_c_func!(inet_pton(_, _, _)),
    export_c_func!(htonl(_)),
    

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/socket.h` (Sockets)
//!
//! We currently support blocking TCP and UDP guest sockets on IPv4 addresses.
//!
//! Because fine grain control is needed, those are implemented as
//! _non-blocking_ host sockets. Moreover, app usage of select() is
//! (optimistically) assumed to check for data readiness before calling
//! any of blocking functions.
//! (Check related functions for more details and remediation.)
//!
//! Other note: Rust std::net APIs are "too high level" sometimes,
//! thus some workarounds need to be implemented.
//! (e.g. [TcpListener] does both bind() and listen() on a call
//! to [TcpListener::bind])
//!
//! Useful resources:
//! - [Beej's Guide to Network Programming](https://beej.us/guide/bgnet/html/index-wide.html)

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{set_errno, EACCES, ENOTCONN, EAGAIN, EADDRINUSE, EADDRNOTAVAIL, EBADF, ECONNRESET, ECONNREFUSED, EINVAL, EIO, EISCONN, ENETUNREACH, ESOCKTNOSUPPORT, ETIMEDOUT, EPROTONOSUPPORT};
use crate::libc::posix_io::{close, find_or_create_socket, is_socket, FileDescriptor};
use crate::libc::time::timeval;
use crate::mem::{
    guest_size_of, ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead,
};
use crate::Environment;

use crate::abi::DotDotDot;
use crate::libc::netdb::{socklen_t, IPPROTO_TCP, IPPROTO_UDP};
use std::collections::{HashMap, HashSet};
use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr, SocketAddrV4, TcpListener, TcpStream, UdpSocket};

pub const AF_INET: i32 = 2;
pub const SOCK_STREAM: i32 = 1;
pub const SOCK_DGRAM: i32 = 2;

const SOL_SOCKET: i32 = 0xffff;
const SO_DEBUG: i32 = 0x1;
const SO_REUSEADDR: i32 = 0x4;
const SO_BROADCAST: i32 = 0x20;
const SO_ERROR: i32 = 0x1007;

#[allow(non_camel_case_types)]
pub type sa_family_t = u8;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
#[allow(non_camel_case_types)]
pub struct sockaddr {
    sa_len: u8,
    sa_family: sa_family_t,
    sa_data: [u8; 14],
}
unsafe impl SafeRead for sockaddr {}
impl sockaddr {
    /// Makes an IPv4 sockaddr from 4 bytes for ip and a port.
    ///
    /// Port is expected to be native endian and
    /// will be converted to big endian internally.
    pub fn from_ipv4_parts(octets: [u8; 4], port: u16) -> Self {
        let mut addr = sockaddr {
            sa_len: 16,
            sa_family: AF_INET as u8,
            sa_data: [0; 14],
        };
        addr.sa_data[0..2].copy_from_slice(&port.to_be_bytes());
        addr.sa_data[2..6].copy_from_slice(&octets);
        addr
    }
    /// Returns 4 bytes for ip and a port.
    ///
    /// Port is returned in the native endian format.
    fn to_ipv4_parts(self) -> ([u8; 4], u16) {
        assert!(self.sa_len == 16 || self.sa_len == 0);
        assert_eq!(self.sa_family, AF_INET as u8);
        let port = u16::from_be_bytes([self.sa_data[0], self.sa_data[1]]);
        let ip = [
            self.sa_data[2],
            self.sa_data[3],
            self.sa_data[4],
            self.sa_data[5],
        ];
        (ip, port)
    }
    fn from_sockaddr_v4(addr: &SocketAddr) -> Self {
        // Only IPV4 for the moment
        assert!(addr.is_ipv4());
        let SocketAddr::V4(ipv4addr) = addr else {
            unreachable!()
        };
        sockaddr::from_ipv4_parts(ipv4addr.ip().octets(), ipv4addr.port())
    }
    pub fn to_sockaddr_v4(self) -> SocketAddrV4 {
        let (ip, port) = self.to_ipv4_parts();
        SocketAddrV4::new(ip.into(), port)
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
#[allow(non_camel_case_types)]
pub struct fd_set {
    // 32 4-byte ints should be enough for 1024 file descriptors
    fds_bits: [i32; 32],
}
unsafe impl SafeRead for fd_set {}

struct SocketHostObject {
    /// Type of the socket, [SOCK_STREAM] for TCP or [SOCK_DGRAM] for UDP
    type_: i32,
    /// Set of options
    options: HashSet<i32>,
    /// TCP socket which is yet to be connected
    tcp_listener: Option<TcpListener>,
    /// TCP socket which was connected on host, but not (yet) on the guest side
    pending_tcp_stream: Option<TcpStream>,
    /// Already connected TCP socket
    tcp_stream: Option<TcpStream>,
    /// UDP socket
    udp_socket: Option<UdpSocket>,
}

#[derive(Default)]
pub struct State {
    sockets: HashMap<i32, SocketHostObject>,
}
impl State {
    fn get(env: &Environment) -> &Self {
        &env.libc_state.socket
    }
    fn get_mut(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.socket
    }
}

fn socket(env: &mut Environment, domain: i32, type_: i32, protocol: i32) -> FileDescriptor {
    // TODO: handle errno properly
    set_errno(env, 0);

    if !env.options.network_access {
        log_dbg!(
            "Network access is disabled, socket({}, {}, {}) => -1",
            domain,
            type_,
            protocol
        );
        set_errno(env, EPROTONOSUPPORT);
        return -1;
    }

    assert_eq!(domain, AF_INET);
    assert!(type_ == SOCK_STREAM || type_ == SOCK_DGRAM);
    assert!(protocol == IPPROTO_TCP || protocol == IPPROTO_UDP || protocol == 0);

    let fd = find_or_create_socket(env);
    assert!(!State::get(env).sockets.contains_key(&fd));
    let host_object = SocketHostObject {
        type_,
        options: Default::default(),
        tcp_listener: None,
        pending_tcp_stream: None,
        tcp_stream: None,
        udp_socket: None,
    };
    State::get_mut(env).sockets.insert(fd, host_object);

    log_dbg!("socket({}, {}, {}) => {}", domain, type_, protocol, fd);
    fd
}

fn ioctl(env: &mut Environment, fd: i32, request: u32, _args: DotDotDot) -> i32 {
    assert!(is_socket(env, fd));
    log!("TODO: ioctl({} (socket), {:#x?}, ...) => -1", fd, request);
    -1
}

fn getsockopt(
    env: &mut Environment,
    socket: i32,
    level: i32,
    option_name: i32,
    option_value: MutVoidPtr,
    option_len: MutPtr<socklen_t>,
) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    log_dbg!(
        "getsockopt({}, {:#x}, {:#x}, {:?}, {:?})",
        socket,
        level,
        option_name,
        option_value,
        option_len
    );

    assert_eq!(level, SOL_SOCKET);
    // TODO: support other options
    assert_eq!(option_name, SO_ERROR);

    let option_len_val = env.mem.read(option_len);
    assert_eq!(option_len_val, 4);

    let option_value: MutPtr<i32> = option_value.cast();
    env.mem.write(option_value, 0); // no errors

    0 // Success
}

fn setsockopt(
    env: &mut Environment,
    socket: i32,
    level: i32,
    option_name: i32,
    option_value: ConstVoidPtr,
    option_len: socklen_t,
) -> i32 {
    set_errno(env, 0);
    log_dbg!(
        "setsockopt({}, {:#x}, {:#x}, {:?}, {})",
        socket, level, option_name, option_value, option_len
    );

    let Some(sock) = State::get(env).sockets.get(&socket) else {
        set_errno(env, EBADF);
        return -1;
    };
    let type_ = sock.type_;

    match (level, option_name) {
        (SOL_SOCKET, SO_DEBUG) => {
            // Silently ignore SO_DEBUG \u2014 requires elevated privileges on most
            // platforms; apps set this speculatively and don't check the result.
            log_dbg!("setsockopt: ignoring SO_DEBUG on socket {}", socket);
            0
        }
        (SOL_SOCKET, SO_REUSEADDR) | (SOL_SOCKET, SO_BROADCAST) => {
            assert_eq!(option_len, guest_size_of::<i32>());
            let val: i32 = env.mem.read(option_value.cast());
            if val != 0 {
                State::get_mut(env)
                    .sockets.get_mut(&socket).unwrap()
                    .options.insert(option_name);
            } else {
                State::get_mut(env)
                    .sockets.get_mut(&socket).unwrap()
                    .options.remove(&option_name);
            }
            // Apply SO_BROADCAST immediately if the UDP socket already exists.
            if option_name == SO_BROADCAST {
                if let Some(udp) = State::get(env)
                    .sockets.get(&socket).unwrap().udp_socket.as_ref()
                {
                    if let Err(e) = udp.set_broadcast(val != 0) {
                        log!("setsockopt: set_broadcast failed: {}", e);
                        set_errno(env, EIO);
                        return -1;
                    }
                }
            }
            0
        }
        (level, option_name) if level == IPPROTO_TCP as i32 => {
            // TCP_NODELAY (1) \u2014 disable Nagle's algorithm.
            const TCP_NODELAY: i32 = 1;
            if option_name == TCP_NODELAY {
                assert_eq!(option_len, guest_size_of::<i32>());
                let val: i32 = env.mem.read(option_value.cast());
                if type_ == SOCK_STREAM {
                    if let Some(stream) = State::get(env)
                        .sockets.get(&socket).unwrap().tcp_stream.as_ref()
                    {
                        if let Err(e) = stream.set_nodelay(val != 0) {
                            log!("setsockopt TCP_NODELAY failed: {}", e);
                            set_errno(env, EIO);
                            return -1;
                        }
                    }
                    // If stream doesn't exist yet, store it for later.
                    if val != 0 {
                        State::get_mut(env)
                            .sockets.get_mut(&socket).unwrap()
                            .options.insert(TCP_NODELAY);
                    }
                }
                0
            } else {
                log!("setsockopt: unhandled IPPROTO_TCP option {:#x}, ignoring", option_name);
                0
            }
        }
        (level, option_name) => {
            log!(
                "setsockopt: unhandled level={:#x} option={:#x} on socket {}, ignoring",
                level, option_name, socket
            );
            0 // Return success rather than crashing the app
        }
    }
}

fn bind(
    env: &mut Environment,
    socket: i32,
    address: ConstPtr<sockaddr>,
    address_len: socklen_t,
) -> i32 {
    set_errno(env, 0);

    let Some(sock) = State::get(env).sockets.get(&socket) else {
        set_errno(env, EBADF);
        return -1;
    };
    let type_ = sock.type_;

    if type_ != SOCK_STREAM && type_ != SOCK_DGRAM {
        set_errno(env, ESOCKTNOSUPPORT);
        return -1;
    }

    if address_len < guest_size_of::<sockaddr>() {
        set_errno(env, EINVAL);
        return -1;
    }

    let sockaddr_val = env.mem.read(address);
    let socket_address = sockaddr_val.to_sockaddr_v4();
    let type_str = match type_ {
        SOCK_STREAM => "TCP",
        SOCK_DGRAM  => "UDP",
        _           => unreachable!(),
    };
    log_dbg!(
        "bind({}, {:?} ({:?}), {}) -> {} {:?}",
        socket, address, sockaddr_val, address_len, type_str, socket_address
    );

    match type_ {
        SOCK_STREAM => {
            if State::get(env).sockets.get(&socket).unwrap().tcp_listener.is_some() {
                set_errno(env, EINVAL); // already bound
                return -1;
            }
            match TcpListener::bind(socket_address) {
                Ok(host_socket) => {
                    if let Err(e) = host_socket.set_nonblocking(true) {
                        log!("bind: TCP set_nonblocking failed: {}", e);
                        set_errno(env, EIO);
                        return -1;
                    }
                    // Apply SO_REUSEADDR if set (best-effort; std doesn't expose it directly)
                    State::get_mut(env)
                        .sockets.get_mut(&socket).unwrap()
                        .tcp_listener = Some(host_socket);
                }
                Err(e) => {
                    log!("bind: TcpListener::bind({:?}) failed: {}", socket_address, e);
                    let errno = match e.kind() {
                        io::ErrorKind::AddrInUse        => EADDRINUSE,
                        io::ErrorKind::AddrNotAvailable => EADDRNOTAVAIL,
                        io::ErrorKind::PermissionDenied => EACCES,
                        _                               => EIO,
                    };
                    set_errno(env, errno);
                    return -1;
                }
            }
        }
        SOCK_DGRAM => {
            if State::get(env).sockets.get(&socket).unwrap().udp_socket.is_some() {
                set_errno(env, EINVAL); // already bound
                return -1;
            }
            // Collect options before the mutable borrow below
            let options: Vec<i32> = State::get(env)
                .sockets.get(&socket).unwrap()
                .options.iter().copied().collect();
            match UdpSocket::bind(socket_address) {
                Ok(host_socket) => {
                    if let Err(e) = host_socket.set_nonblocking(true) {
                        log!("bind: UDP set_nonblocking failed: {}", e);
                        set_errno(env, EIO);
                        return -1;
                    }
                    for option in options {
                        if option == SO_BROADCAST {
                            if let Err(e) = host_socket.set_broadcast(true) {
                                log!("bind: set_broadcast failed: {}", e);
                                set_errno(env, EIO);
                                return -1;
                            }
                        }
                    }
                    State::get_mut(env)
                        .sockets.get_mut(&socket).unwrap()
                        .udp_socket = Some(host_socket);
                }
                Err(e) => {
                    log!("bind: UdpSocket::bind({:?}) failed: {}", socket_address, e);
                    let errno = match e.kind() {
                        io::ErrorKind::AddrInUse        => EADDRINUSE,
                        io::ErrorKind::AddrNotAvailable => EADDRNOTAVAIL,
                        io::ErrorKind::PermissionDenied => EACCES,
                        _                               => EIO,
                    };
                    set_errno(env, errno);
                    return -1;
                }
            }
        }
        _ => unreachable!(),
    }

    0 // Success
}

fn listen(env: &mut Environment, socket: i32, backlog: i32) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let type_ = match State::get(env).sockets.get(&socket) {
        Some(s) => s.type_,
        None => {
            log!("listen: unknown socket fd={}, returning EBADF", socket);
            set_errno(env, EBADF);
            return -1;
        }
    };
    if type_ != SOCK_STREAM {
        set_errno(env, ESOCKTNOSUPPORT);
        return -1;
    }

    log!(
        "Warning

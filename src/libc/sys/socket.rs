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
use crate::libc::errno::{set_errno, EBADF, ECONNRESET, EINVAL, ENOTCONN, EPROTONOSUPPORT};
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

pub const AF_INET:    i32 = 2;
pub const AF_INET6:   i32 = 30;
pub const AF_UNSPEC:  i32 = 0;
pub const SOCK_STREAM: i32 = 1;
pub const SOCK_DGRAM:  i32 = 2;

// MSG_ flags
const MSG_OOB:       i32 = 0x1;
const MSG_PEEK:      i32 = 0x2;
const MSG_DONTROUTE: i32 = 0x4;
const MSG_WAITALL:   i32 = 0x40;

const SOL_SOCKET:    i32 = 0xffff;
const SO_DEBUG:      i32 = 0x0001;
const SO_REUSEADDR:  i32 = 0x0004;
const SO_KEEPALIVE:  i32 = 0x0008;
const SO_DONTROUTE:  i32 = 0x0010;
const SO_BROADCAST:  i32 = 0x0020;
const SO_LINGER:     i32 = 0x0080;
const SO_SNDBUF:     i32 = 0x1001;
const SO_RCVBUF:     i32 = 0x1002;
const SO_SNDTIMEO:   i32 = 0x1005;
const SO_RCVTIMEO:   i32 = 0x1006;
const SO_ERROR:      i32 = 0x1007;
const SO_TYPE:       i32 = 0x1008;
const SO_NOSIGPIPE:  i32 = 0x1022;

const SHUT_RD:   i32 = 0;
const SHUT_WR:   i32 = 1;
const SHUT_RDWR: i32 = 2;

// IPPROTO_TCP options
const TCP_NODELAY:   i32 = 1;
const TCP_KEEPALIVE: i32 = 0x10;

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
        assert!(addr.is_ipv4());
        let SocketAddr::V4(ipv4addr) = addr else { unreachable!() };
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
    fds_bits: [i32; 32],
}
unsafe impl SafeRead for fd_set {}

struct SocketHostObject {
    type_: i32,
    options: HashSet<i32>,
    tcp_listener: Option<TcpListener>,
    pending_tcp_stream: Option<TcpStream>,
    tcp_stream: Option<TcpStream>,
    udp_socket: Option<UdpSocket>,
    /// Peer address stored for getpeername.
    peer_addr: Option<SocketAddr>,
    /// Local address stored at bind time for getsockname.
    local_addr: Option<SocketAddr>,
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

// =========================================================================
// MARK: - socket / ioctl
// =========================================================================

fn socket(env: &mut Environment, domain: i32, type_: i32, protocol: i32) -> FileDescriptor {
    set_errno(env, 0);

    if !env.options.network_access {
        log_dbg!(
            "Network access is disabled, socket({}, {}, {}) => -1",
            domain, type_, protocol
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
        peer_addr: None,
        local_addr: None,
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

// =========================================================================
// MARK: - getsockopt / setsockopt
// =========================================================================

fn getsockopt(
    env: &mut Environment,
    socket: i32,
    level: i32,
    option_name: i32,
    option_value: MutVoidPtr,
    option_len: MutPtr<socklen_t>,
) -> i32 {
    set_errno(env, 0);
    log_dbg!(
        "getsockopt({}, {:#x}, {:#x}, {:?}, {:?})",
        socket, level, option_name, option_value, option_len
    );

    let Some(sock) = State::get(env).sockets.get(&socket) else {
        set_errno(env, EBADF);
        return -1;
    };

    match (level, option_name) {
        (SOL_SOCKET, SO_ERROR) => {
            assert_eq!(env.mem.read(option_len), 4);
            let p: MutPtr<i32> = option_value.cast();
            env.mem.write(p, 0);
            env.mem.write(option_len, 4);
        }
        (SOL_SOCKET, SO_TYPE) => {
            assert_eq!(env.mem.read(option_len), 4);
            let p: MutPtr<i32> = option_value.cast();
            env.mem.write(p, sock.type_);
            env.mem.write(option_len, 4);
        }
        (SOL_SOCKET, SO_REUSEADDR)
        | (SOL_SOCKET, SO_BROADCAST)
        | (SOL_SOCKET, SO_KEEPALIVE)
        | (SOL_SOCKET, SO_NOSIGPIPE) => {
            let enabled = sock.options.contains(&option_name);
            assert_eq!(env.mem.read(option_len), 4);
            let p: MutPtr<i32> = option_value.cast();
            env.mem.write(p, if enabled { 1 } else { 0 });
            env.mem.write(option_len, 4);
        }
        (SOL_SOCKET, SO_SNDBUF) | (SOL_SOCKET, SO_RCVBUF) => {
            // Report a plausible default buffer size (128 KiB).
            assert_eq!(env.mem.read(option_len), 4);
            let p: MutPtr<i32> = option_value.cast();
            env.mem.write(p, 131072);
            env.mem.write(option_len, 4);
        }
        (IPPROTO_TCP, TCP_NODELAY) => {
            let enabled = sock.options.contains(&(TCP_NODELAY | 0x10000));
            assert_eq!(env.mem.read(option_len), 4);
            let p: MutPtr<i32> = option_value.cast();
            env.mem.write(p, if enabled { 1 } else { 0 });
            env.mem.write(option_len, 4);
        }
        _ => {
            log!(
                "TODO: getsockopt({}, level={:#x}, opt={:#x}) — returning 0",
                socket, level, option_name
            );
            if env.mem.read(option_len) >= 4 {
                let p: MutPtr<i32> = option_value.cast();
                env.mem.write(p, 0);
                env.mem.write(option_len, 4);
            }
        }
    }
    0
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

    if option_name == SO_DEBUG {
        set_errno(env, EINVAL);
        log!("Warning: setsockopt SO_DEBUG => -1");
        return -1;
    }

    let Some(sock) = State::get(env).sockets.get(&socket) else {
        set_errno(env, EBADF);
        return -1;
    };
    let type_ = sock.type_;
    assert!(type_ == SOCK_STREAM || type_ == SOCK_DGRAM);

    match (level, option_name) {
        (SOL_SOCKET, SO_REUSEADDR)
        | (SOL_SOCKET, SO_BROADCAST)
        | (SOL_SOCKET, SO_KEEPALIVE)
        | (SOL_SOCKET, SO_NOSIGPIPE)
        | (SOL_SOCKET, SO_DONTROUTE) => {
            assert_eq!(option_len, guest_size_of::<i32>());
            let val: i32 = env.mem.read(option_value.cast::<i32>());
            let opts = &mut State::get_mut(env).sockets.get_mut(&socket).unwrap().options;
            if val != 0 { opts.insert(option_name); } else { opts.remove(&option_name); }
        }
        (SOL_SOCKET, SO_SNDBUF) | (SOL_SOCKET, SO_RCVBUF) => {
            log_dbg!("setsockopt: SO_SNDBUF/SO_RCVBUF ignored (host manages buffer sizes)");
        }
        (SOL_SOCKET, SO_SNDTIMEO) | (SOL_SOCKET, SO_RCVTIMEO) => {
            log_dbg!("setsockopt: SO_SNDTIMEO/SO_RCVTIMEO ignored (non-blocking host sockets)");
        }
        (SOL_SOCKET, SO_LINGER) => {
            log_dbg!("setsockopt: SO_LINGER ignored");
        }
        (IPPROTO_TCP, TCP_NODELAY) => {
            // Store with a namespace bit so it doesn't clash with SOL_SOCKET options.
            let val: i32 = env.mem.read(option_value.cast::<i32>());
            let opts = &mut State::get_mut(env).sockets.get_mut(&socket).unwrap().options;
            if val != 0 { opts.insert(TCP_NODELAY | 0x10000); } else { opts.remove(&(TCP_NODELAY | 0x10000)); }
            log_dbg!("setsockopt: TCP_NODELAY={} (advisory only)", val);
        }
        (IPPROTO_TCP, TCP_KEEPALIVE) => {
            log_dbg!("setsockopt: TCP_KEEPALIVE ignored");
        }
        _ => {
            log!(
                "TODO: setsockopt({}, level={:#x}, opt={:#x}) — ignored",
                socket, level, option_name
            );
        }
    }
    0
}

// =========================================================================
// MARK: - bind / listen / connect
// =========================================================================

fn bind(
    env: &mut Environment,
    socket: i32,
    address: ConstPtr<sockaddr>,
    address_len: socklen_t,
) -> i32 {
    set_errno(env, 0);

    let socket_host_object = State::get(env).sockets.get(&socket).unwrap();
    let type_ = socket_host_object.type_;
    assert!(type_ == SOCK_STREAM || type_ == SOCK_DGRAM);
    assert_eq!(address_len, guest_size_of::<sockaddr>());

    let sockaddr_val = env.mem.read(address);
    let socket_address = sockaddr_val.to_sockaddr_v4();
    log_dbg!("bind({}, {:?} ({:?}), {})", socket, address, sockaddr_val, address_len);

    State::get_mut(env).sockets.get_mut(&socket).unwrap().local_addr =
        Some(SocketAddr::V4(socket_address));

    let socket_host_object = State::get(env).sockets.get(&socket).unwrap();
    match type_ {
        SOCK_STREAM => {
            assert!(socket_host_object.tcp_listener.is_none());
            let host_socket = TcpListener::bind(socket_address).unwrap();
            host_socket.set_nonblocking(true).unwrap();
            State::get_mut(env).sockets.get_mut(&socket).unwrap().tcp_listener = Some(host_socket);
        }
        SOCK_DGRAM => {
            assert!(socket_host_object.udp_socket.is_none());
            let host_socket = UdpSocket::bind(socket_address).unwrap();
            host_socket.set_nonblocking(true).unwrap();
            for &option in &socket_host_object.options {
                if option == SO_BROADCAST {
                    host_socket.set_broadcast(true).unwrap();
                }
            }
            State::get_mut(env).sockets.get_mut(&socket).unwrap().udp_socket = Some(host_socket);
        }
        _ => unreachable!(),
    }
    0
}

fn listen(env: &mut Environment, socket: i32, backlog: i32) -> i32 {
    set_errno(env, 0);
    let type_ = State::get(env).sockets.get(&socket).unwrap().type_;
    assert!(type_ == SOCK_STREAM);
    log!("Warning: listen(socket: {}, backlog: {}), ignoring", socket, backlog);
    0
}

fn connect(
    env: &mut Environment,
    socket: i32,
    address: ConstPtr<sockaddr>,
    address_len: socklen_t,
) -> i32 {
    set_errno(env, 0);
    let type_ = State::get(env).sockets.get(&socket).unwrap().type_;
    assert!(type_ == SOCK_STREAM);
    assert_eq!(address_len, guest_size_of::<sockaddr>());

    let sockaddr_val = env.mem.read(address);
    let socket_address = sockaddr_val.to_sockaddr_v4();
    log_dbg!("connect({:?} ({:?}), {})", address, sockaddr_val, address_len);

    assert!(State::get(env).sockets.get(&socket).unwrap().tcp_stream.is_none());
    let host_stream = TcpStream::connect(socket_address).unwrap();
    host_stream.set_nonblocking(true).unwrap();

    let peer = host_stream.peer_addr().ok();
    let local = host_stream.local_addr().ok();
    {
        let sock = State::get_mut(env).sockets.get_mut(&socket).unwrap();
        sock.tcp_stream = Some(host_stream);
        sock.peer_addr = peer;
        if let Some(la) = local {
            sock.local_addr = Some(la);
        }
    }
    0
}

// =========================================================================
// MARK: - select
// =========================================================================

fn select(
    env: &mut Environment,
    n_fds: i32,
    read_fds: MutPtr<fd_set>,
    write_fds: MutPtr<fd_set>,
    error_fds: MutPtr<fd_set>,
    timeout: MutPtr<timeval>,
) -> i32 {
    set_errno(env, 0);
    assert!(n_fds > 0 && n_fds < 1024);

    let should_block = if !timeout.is_null() {
        let timeval = env.mem.read(timeout);
        if timeval.tv_sec == 0 && timeval.tv_usec == 0 { false }
        else { log_dbg!("TODO: Ignore non-zero timeout {:?} in select()", timeval); true }
    } else { true };

    let mut count = 0;

    if !read_fds.is_null() {
        let mut read_set = env.mem.read(read_fds);
        count += process_set(env, &mut read_set, n_fds, |env, fd, bits, bit_index| {
            log_dbg!("select: bit set in read_set at fd: {}", fd);
            assert!(is_socket(env, fd));
            *bits &= !(1 << bit_index);
            let socket_host_object = State::get(env).sockets.get(&fd).unwrap();
            let type_ = socket_host_object.type_;
            match type_ {
                SOCK_DGRAM => {
                    let udp_socket = socket_host_object.udp_socket.as_ref().unwrap();
                    let mut buf = [0; 1];
                    match udp_socket.peek(&mut buf) {
                        Ok(received) => { log_dbg!("select: Socket {} peeked {} bytes", fd, received); *bits |= 1 << bit_index; true }
                        Err(ref e) if cfg!(target_os = "windows") && e.raw_os_error() == Some(10040) => { *bits |= 1 << bit_index; true }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => { assert!(!should_block); false }
                        Err(e) => { log!("select: Peek for socket {fd} failed: {e:?}"); false }
                    }
                }
                SOCK_STREAM => {
                    if socket_host_object.tcp_stream.is_none() {
                        let listener = socket_host_object.tcp_listener.as_ref().unwrap();
                        match listener.accept() {
                            Ok((stream, addr)) => {
                                log!("select: New client: {}", addr);
                                stream.set_nonblocking(true).unwrap();
                                assert!(socket_host_object.pending_tcp_stream.is_none());
                                State::get_mut(env).sockets.get_mut(&fd).unwrap().pending_tcp_stream = Some(stream);
                                *bits |= 1 << bit_index;
                                return true;
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => { assert!(!should_block); return false; }
                            Err(e) => { log!("select: Socket {fd} accept error: {e}"); return false; }
                        }
                    }
                    let stream = socket_host_object.tcp_stream.as_ref().unwrap();
                    let mut buf = [0; 1];
                    match stream.peek(&mut buf) {
                        Ok(received) => { log_dbg!("select: received {} bytes (at least)", received); *bits |= 1 << bit_index; true }
                        Err(ref e) if cfg!(target_os = "windows") && e.raw_os_error() == Some(10040) => { *bits |= 1 << bit_index; true }
                        Err(ref e) if e.kind() == io::ErrorKind::ConnectionReset => { log!("select: Peek for socket {}: ConnectionReset", fd); *bits |= 1 << bit_index; true }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => { assert!(!should_block); false }
                        Err(e) => { log!("select: Peek for socket {fd} failed: {e}"); false }
                    }
                }
                _ => unimplemented!(),
            }
        });
        env.mem.write(read_fds, read_set);
    }

    if !write_fds.is_null() {
        let mut write_set = env.mem.read(write_fds);
        count += process_set(env, &mut write_set, n_fds, |env, fd, bits, bit_index| {
            log_dbg!("select: bit set in write_set at fd: {}", fd);
            assert!(is_socket(env, fd));
            *bits &= !(1 << bit_index);
            let sock = State::get(env).sockets.get(&fd).unwrap();
            match sock.type_ {
                SOCK_STREAM => {
                    if sock.tcp_stream.is_some() { *bits |= 1 << bit_index; true }
                    else { assert!(!should_block); false }
                }
                SOCK_DGRAM => {
                    if sock.udp_socket.is_some() { *bits |= 1 << bit_index; true }
                    else { assert!(!should_block); false }
                }
                _ => unimplemented!(),
            }
        });
        env.mem.write(write_fds, write_set);
    }

    if !error_fds.is_null() {
        let mut error_set = env.mem.read(error_fds);
        count += process_set(env, &mut error_set, n_fds, |env, fd, bits, bit_index| {
            log_dbg!("select: bit set in error_set at fd: {}", fd);
            assert!(is_socket(env, fd));
            *bits &= !(1 << bit_index);
            let sock = State::get(env).sockets.get(&fd).unwrap();
            match sock.type_ {
                SOCK_STREAM => {
                    let stream = sock.tcp_stream.as_ref().unwrap();
                    match stream.take_error() {
                        Ok(None) => false,
                        Ok(Some(error)) => { log!("TCP socket {} error: {:?}", fd, error); *bits |= 1 << bit_index; true }
                        Err(error) => { log!("TCP socket {fd} take_error failed: {error:?}"); false }
                    }
                }
                SOCK_DGRAM => false,
                _ => unimplemented!(),
            }
        });
        env.mem.write(error_fds, error_set);
    }

    count
}

fn process_set<F: Fn(&mut Environment, FileDescriptor, &mut i32, i32) -> bool>(
    env: &mut Environment,
    set: &mut fd_set,
    n_fds: i32,
    process_bit: F,
) -> i32 {
    let mut fds_bits = set.fds_bits;
    let mut count = 0;
    'outer: for (i, bits) in fds_bits.iter_mut().enumerate() {
        for bit_index in 0..32i32 {
            let fd: FileDescriptor = (i as i32) * 32 + bit_index;
            if fd > n_fds { break 'outer; }
            if (*bits & (1 << bit_index)) != 0 && process_bit(env, fd, bits, bit_index) {
                count += 1;
            }
        }
    }
    set.fds_bits = fds_bits;
    count
}

// =========================================================================
// MARK: - accept
// =========================================================================

fn accept(
    env: &mut Environment,
    socket: i32,
    address: MutPtr<sockaddr>,
    address_len: MutPtr<socklen_t>,
) -> FileDescriptor {
    set_errno(env, 0);

    let Some(socket_host_object) = State::get(env).sockets.get(&socket) else {
        set_errno(env, EBADF);
        return -1;
    };
    let type_ = socket_host_object.type_;
    assert!(type_ == SOCK_STREAM);

    if let Some(stream) = State::get_mut(env).sockets.get_mut(&socket).unwrap().pending_tcp_stream.take() {
        let addr = stream.peer_addr().unwrap();
        let new_fd = find_or_create_socket(env);
        assert!(!State::get(env).sockets.contains_key(&new_fd));
        let host_object = SocketHostObject {
            type_: SOCK_STREAM,
            options: Default::default(),
            tcp_listener: None,
            pending_tcp_stream: None,
            tcp_stream: Some(stream),
            udp_socket: None,
            peer_addr: Some(addr),
            local_addr: None,
        };
        State::get_mut(env).sockets.insert(new_fd, host_object);
        if !address.is_null() {
            let peer_guest_addr = sockaddr::from_sockaddr_v4(&addr);
            env.mem.write(address, peer_guest_addr);
            assert_eq!(guest_size_of::<sockaddr>(), env.mem.read(address_len));
            env.mem.write(address_len, guest_size_of::<sockaddr>());
        }
        return new_fd;
    }

    let socket_host_object = State::get(env).sockets.get(&socket).unwrap();
    let listener = socket_host_object.tcp_listener.as_ref().unwrap();
    match listener.accept() {
        Ok((_, addr)) => { log!("accept: New client: {}", addr); unimplemented!() }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
            unimplemented!("accept: TCP listener for socket {} would block, thread {}", socket, env.current_thread)
        }
        Err(e) => { log!("accept: Socket {socket} error: {e}"); set_errno(env, EBADF); -1 }
    }
}

// =========================================================================
// MARK: - recv / recvfrom / recvmsg
// =========================================================================

fn recv(
    env: &mut Environment,
    socket: i32,
    buffer: MutVoidPtr,
    length: GuestUSize,
    flags: i32,
) -> i32 {
    recvfrom(env, socket, buffer, length, flags, Ptr::null(), Ptr::null())
}

fn recvfrom(
    env: &mut Environment,
    socket: i32,
    buffer: MutVoidPtr,
    length: GuestUSize,
    flags: i32,
    address: MutPtr<sockaddr>,
    address_len: MutPtr<socklen_t>,
) -> i32 {
    set_errno(env, 0);
    log_dbg!("recvfrom({}, {:?}, {}, {}, {:?}, {:?})", socket, buffer, length, flags, address, address_len);

    if !State::get(env).sockets.contains_key(&socket) {
        set_errno(env, EBADF);
        return -1;
    }

    let type_ = State::get(env).sockets.get(&socket).unwrap().type_;
    assert!(type_ == SOCK_STREAM || type_ == SOCK_DGRAM);

    // Handle MSG_PEEK — peek without consuming.
    let do_peek = (flags & MSG_PEEK) != 0;
    let remaining_flags = flags & !(MSG_PEEK | MSG_WAITALL);
    if remaining_flags != 0 {
        log!("Warning: recvfrom: unhandled flags {:#x}", remaining_flags);
    }

    let (num_bytes_read, addr) = match type_ {
        SOCK_DGRAM => {
            let udp_socket = env.libc_state.socket.sockets.get(&socket).unwrap().udp_socket.as_ref().unwrap();
            let buf = env.mem.bytes_at_mut(buffer.cast(), length);
            let (read, addr) = if do_peek {
                match udp_socket.peek_from(buf) {
                    Ok(n) => n,
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        unimplemented!("recvfrom peek: UDP socket {} would block, thread {}", socket, env.current_thread)
                    }
                    Err(e) => { log!("recvfrom: UDP socket {socket} IO error: {e}"); set_errno(env, EBADF); return -1; }
                }
            } else {
                match udp_socket.recv_from(buf) {
                    Ok(n) => n,
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        unimplemented!("recvfrom: UDP socket {} would block, thread {}", socket, env.current_thread)
                    }
                    Err(e) => { log!("recvfrom: UDP socket {socket} IO error: {e}"); set_errno(env, EBADF); return -1; }
                }
            };
            if !address.is_null() {
                let guest_addr = sockaddr::from_sockaddr_v4(&addr);
                env.mem.write(address, guest_addr);
                assert_eq!(guest_size_of::<sockaddr>(), env.mem.read(address_len));
                env.mem.write(address_len, guest_size_of::<sockaddr>());
            }
            (read, Ok(addr))
        }
        SOCK_STREAM => {
            assert!(address.is_null());
            assert!(address_len.is_null());
            let tcp_stream = env.libc_state.socket.sockets.get(&socket).unwrap().tcp_stream.as_ref().unwrap();
            let buf = env.mem.bytes_at_mut(buffer.cast(), length);
            let read = if do_peek {
                match tcp_stream.peek(buf) {
                    Ok(n) => n,
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        unimplemented!("recvfrom peek: TCP socket {} would block, thread {}", socket, env.current_thread)
                    }
                    Err(e) => { log!("recvfrom peek: TCP socket {socket} IO error: {e}"); set_errno(env, EBADF); return -1; }
                }
            } else {
                match tcp_stream.read(buf) {
                    Ok(n) => n,
                    Err(ref e) if e.kind() == io::ErrorKind::ConnectionReset => { set_errno(env, ECONNRESET); return -1; }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        unimplemented!("recvfrom: TCP socket {} would block, thread {}", socket, env.current_thread)
                    }
                    Err(e) => { log!("recvfrom: TCP socket {socket} IO error: {e}"); set_errno(env, EBADF); return -1; }
                }
            };
            (read, tcp_stream.peer_addr())
        }
        _ => unreachable!(),
    };
    log_dbg!("recvfrom: Socket {} received {} bytes", socket, num_bytes_read);
    num_bytes_read.try_into().unwrap()
}

// =========================================================================
// MARK: - send / sendto
// =========================================================================

fn send(
    env: &mut Environment,
    socket: i32,
    buffer: MutVoidPtr,
    length: GuestUSize,
    flags: i32,
) -> i32 {
    set_errno(env, 0);
    let type_ = State::get(env).sockets.get(&socket).unwrap().type_;
    assert!(type_ == SOCK_STREAM);
    if flags != 0 { log!("Warning: send: unhandled flags {:#x}", flags); }

    let tcp_stream = env.libc_state.socket.sockets.get(&socket).unwrap().tcp_stream.as_ref().unwrap();
    let buf = env.mem.bytes_at(buffer.cast(), length);
    match tcp_stream.write(buf) {
        Ok(written) => { log_dbg!("send: written {} bytes to TCP socket {}", written, socket); written.try_into().unwrap() }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
            unimplemented!("send: TCP socket {} would block, thread {}", socket, env.current_thread)
        }
        Err(e) => { log!("send: Socket {socket} IO error: {e}"); set_errno(env, EBADF); -1 }
    }
}

fn sendto(
    env: &mut Environment,
    socket: i32,
    buffer: MutVoidPtr,
    length: GuestUSize,
    flags: i32,
    dest_address: MutPtr<sockaddr>,
    dest_address_len: socklen_t,
) -> i32 {
    set_errno(env, 0);
    let type_ = State::get(env).sockets.get(&socket).unwrap().type_;
    assert!(type_ == SOCK_DGRAM);
    if flags != 0 { log!("Warning: sendto: unhandled flags {:#x}", flags); }

    assert_eq!(dest_address_len, guest_size_of::<sockaddr>());
    let sockaddr_val = env.mem.read(dest_address);
    let socket_address = sockaddr_val.to_sockaddr_v4();

    if State::get(env).sockets.get(&socket).unwrap().udp_socket.is_none() {
        assert!(socket_address.ip().is_broadcast());
        let host_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        host_socket.set_nonblocking(true).unwrap();
        for &option in &State::get(env).sockets.get(&socket).unwrap().options {
            if option == SO_BROADCAST { host_socket.set_broadcast(true).unwrap(); }
        }
        State::get_mut(env).sockets.get_mut(&socket).unwrap().udp_socket = Some(host_socket);
    }

    let udp_socket = env.libc_state.socket.sockets.get(&socket).unwrap().udp_socket.as_ref().unwrap();
    let buf = env.mem.bytes_at(buffer.cast(), length);
    match udp_socket.send_to(buf, socket_address) {
        Ok(written) => { log_dbg!("sendto: written {} bytes to UDP socket {}", written, socket); written.try_into().unwrap() }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
            unimplemented!("sendto: UDP socket {} would block, thread {}", socket, env.current_thread)
        }
        Err(e) => { log!("sendto: Socket {socket} IO error: {e}"); set_errno(env, EBADF); -1 }
    }
}

// =========================================================================
// MARK: - getsockname / getpeername
// =========================================================================

fn getsockname(
    env: &mut Environment,
    socket: i32,
    address: MutPtr<sockaddr>,
    address_len: MutPtr<socklen_t>,
) -> i32 {
    set_errno(env, 0);
    let Some(sock) = State::get(env).sockets.get(&socket) else {
        set_errno(env, EBADF);
        return -1;
    };
    let local = match &sock.local_addr {
        Some(a) => *a,
        None => {
            // Try to retrieve from the live socket.
            let fallback = match sock.type_ {
                SOCK_STREAM => sock.tcp_stream.as_ref().and_then(|s| s.local_addr().ok()),
                SOCK_DGRAM  => sock.udp_socket.as_ref().and_then(|s| s.local_addr().ok()),
                _ => None,
            };
            match fallback {
                Some(a) => a,
                None => {
                    log!("getsockname: socket {} has no local address", socket);
                    set_errno(env, ENOTCONN);
                    return -1;
                }
            }
        }
    };
    if !address.is_null() {
        let guest_addr = sockaddr::from_sockaddr_v4(&local);
        env.mem.write(address, guest_addr);
        env.mem.write(address_len, guest_size_of::<sockaddr>());
    }
    log_dbg!("getsockname({}) => {:?}", socket, local);
    0
}

fn getpeername(
    env: &mut Environment,
    socket: i32,
    address: MutPtr<sockaddr>,
    address_len: MutPtr<socklen_t>,
) -> i32 {
    set_errno(env, 0);
    let Some(sock) = State::get(env).sockets.get(&socket) else {
        set_errno(env, EBADF);
        return -1;
    };
    let peer = match &sock.peer_addr {
        Some(a) => *a,
        None => {
            let fallback = match sock.type_ {
                SOCK_STREAM => sock.tcp_stream.as_ref().and_then(|s| s.peer_addr().ok()),
                _ => None,
            };
            match fallback {
                Some(a) => a,
                None => {
                    log!("getpeername: socket {} not connected", socket);
                    set_errno(env, ENOTCONN);
                    return -1;
                }
            }
        }
    };
    if !address.is_null() {
        let guest_addr = sockaddr::from_sockaddr_v4(&peer);
        env.mem.write(address, guest_addr);
        env.mem.write(address_len, guest_size_of::<sockaddr>());
    }
    log_dbg!("getpeername({}) => {:?}", socket, peer);
    0
}

// =========================================================================
// MARK: - shutdown
// =========================================================================

fn shutdown(env: &mut Environment, socket: i32, how: i32) -> i32 {
    log_dbg!("shutdown({}, {})", socket, how);
    match how {
        SHUT_RD | SHUT_WR | SHUT_RDWR => {}
        _ => { log!("Warning: shutdown: unknown how={}", how); }
    }
    // For SHUT_RDWR close the socket entirely; for RD/WR-only we log and
    // return success since Rust's std::net has no partial shutdown.
    if how == SHUT_RDWR {
        return close(env, socket);
    }
    log!("Warning: shutdown({}, {}) partial shutdown not supported — ignoring", socket, how);
    0
}

// =========================================================================
// MARK: - socketpair (stub)
// =========================================================================

fn socketpair(
    _env: &mut Environment,
    _domain: i32,
    _type_: i32,
    _protocol: i32,
    _sv: MutPtr<[i32; 2]>,
) -> i32 {
    log!("TODO: socketpair() — returning -1 (not supported)");
    -1
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(socket(_, _, _)),
    export_c_func!(ioctl(_, _, _)),
    export_c_func!(getsockopt(_, _, _, _, _)),
    export_c_func!(setsockopt(_, _, _, _, _)),
    export_c_func!(bind(_, _, _)),
    export_c_func!(listen(_, _)),
    export_c_func!(connect(_, _, _)),
    export_c_func!(select(_, _, _, _, _)),
    export_c_func!(accept(_, _, _)),
    export_c_func!(recv(_, _, _, _)),
    export_c_func!(recvfrom(_, _, _, _, _, _)),
    export_c_func!(send(_, _, _, _)),
    export_c_func!(sendto(_, _, _, _, _, _)),
    export_c_func!(getsockname(_, _, _)),
    export_c_func!(getpeername(_, _, _)),
    export_c_func!(shutdown(_, _)),
    export_c_func!(socketpair(_, _, _, _)),
];

/// A helper to close a socket, not a part of API
pub fn close_socket(env: &mut Environment, socket: i32) -> bool {
    State::get_mut(env).sockets.remove(&socket).is_none()
}

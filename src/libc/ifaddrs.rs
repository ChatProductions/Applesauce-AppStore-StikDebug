/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `ifaddrs.h` and `net/if.h` (interface addresses and interface naming)

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::libc::errno::{set_errno, EINVAL, ENXIO};
use crate::mem::{ConstPtr, MutPtr, SafeRead};
use crate::Environment;

// Mirrors the POSIX `struct ifaddrs` layout as seen by 32-bit ARM guests.
// All pointer fields are 4-byte guest pointers.
#[allow(non_camel_case_types)]
#[repr(C, packed)]
pub struct ifaddrs {
    /// Next node in the linked list (NULL = end).
    pub ifa_next: MutPtr<ifaddrs>,
    /// NUL-terminated interface name, e.g. "en0".
    pub ifa_name: ConstPtr<u8>,
    /// Interface flags (IFF_UP, IFF_LOOPBACK, …).
    pub ifa_flags: u32,
    /// Primary address (may be NULL).
    pub ifa_addr: u32, // guest ptr to sockaddr – typed as u32 to avoid pulling in socket types
    /// Netmask (may be NULL).
    pub ifa_netmask: u32,
    /// Broadcast or point-to-point destination address (may be NULL).
    pub ifa_broadaddr: u32,
    /// Protocol-specific data (may be NULL).
    pub ifa_data: u32,
}
// SAFETY: the struct is plain data; every field is either a scalar or a guest
// pointer that touchHLE's pointer type already validates.
unsafe impl SafeRead for ifaddrs {}

const AF_INET: u8 = 2;
const IFF_UP: u32 = 0x1;
const IFF_BROADCAST: u32 = 0x2;
const IFF_LOOPBACK: u32 = 0x8;
const IFF_RUNNING: u32 = 0x40;
const IFF_MULTICAST: u32 = 0x8000;

fn alloc_sockaddr_in(env: &mut Environment, ip: [u8; 4]) -> u32 {
    let ptr = env.mem.alloc(16); // sizeof(struct sockaddr_in)
    
    env.mem.write(ptr.cast::<u8>(), 16u8); // sin_len
    env.mem.write((ptr + 1).cast::<u8>(), AF_INET); // sin_family
    env.mem.write((ptr + 2).cast::<u16>(), 0u16); // sin_port
    
    // IP-адрес должен быть в Network Byte Order (Big Endian), поэтому пишем побайтово
    env.mem.write((ptr + 4).cast::<u8>(), ip[0]);
    env.mem.write((ptr + 5).cast::<u8>(), ip[1]);
    env.mem.write((ptr + 6).cast::<u8>(), ip[2]);
    env.mem.write((ptr + 7).cast::<u8>(), ip[3]);
    
    // Обнуляем padding (sin_zero)
    for i in 8..16 {
        env.mem.write((ptr + i).cast::<u8>(), 0);
    }
    
    ptr.to_bits()
}

// ---------------------------------------------------------------------------
// getifaddrs / freeifaddrs
// ---------------------------------------------------------------------------

/// `int getifaddrs(struct ifaddrs **ifap)`
///
/// Stub: we do not enumerate real host interfaces. Returns -1 / EOPNOTSUPP so
/// that callers fall back gracefully instead of crashing on a null list.
fn getifaddrs(env: &mut Environment, ifap: MutPtr<MutPtr<ifaddrs>>) -> i32 {
    if ifap.is_null() {
        set_errno(env, EINVAL);
        return -1;
    }

    // 1. Создаем фейковый Wi-Fi интерфейс (en0)
    let en0_name = env.mem.alloc_and_write_cstr(b"en0");
    let en0_addr = alloc_sockaddr_in(env, [192, 168, 1, 100]);
    let en0_netmask = alloc_sockaddr_in(env, [255, 255, 255, 0]);
    let en0_bcast = alloc_sockaddr_in(env, [192, 168, 1, 255]);

    let en0_ptr: MutPtr<ifaddrs> = env.mem.alloc(std::mem::size_of::<ifaddrs>() as u32).cast();
    let en0_struct = ifaddrs {
        ifa_next: MutPtr::null(), // Конец списка
        ifa_name: en0_name.cast_const(),
        ifa_flags: IFF_UP | IFF_BROADCAST | IFF_RUNNING | IFF_MULTICAST,
        ifa_addr: en0_addr,
        ifa_netmask: en0_netmask,
        ifa_dstaddr: en0_bcast,
        ifa_data: 0,
    };
    env.mem.write(en0_ptr, en0_struct);

    // 2. Создаем фейковый Loopback интерфейс (lo0)
    let lo0_name = env.mem.alloc_and_write_cstr(b"lo0");
    let lo0_addr = alloc_sockaddr_in(env, [127, 0, 0, 1]);
    let lo0_netmask = alloc_sockaddr_in(env, [255, 0, 0, 0]);
    
    let lo0_ptr: MutPtr<ifaddrs> = env.mem.alloc(std::mem::size_of::<ifaddrs>() as u32).cast();
    let lo0_struct = ifaddrs {
        ifa_next: en0_ptr, // Связываем lo0 -> en0
        ifa_name: lo0_name.cast_const(),
        ifa_flags: IFF_UP | IFF_LOOPBACK | IFF_RUNNING | IFF_MULTICAST,
        ifa_addr: lo0_addr,
        ifa_netmask: lo0_netmask,
        ifa_dstaddr: 0,
        ifa_data: 0,
    };
    env.mem.write(lo0_ptr, lo0_struct);

    // Записываем указатель на начало цепочки (lo0) в переданный гостем указатель
    env.mem.write(ifap, lo0_ptr);

    log_dbg!("getifaddrs() -> Success, mocked lo0 and en0 interfaces");
    0
}

/// `void freeifaddrs(struct ifaddrs *ifa)`
///
/// Since our `getifaddrs` never allocates anything, this is a no-op. If a
/// future implementation does allocate, the deallocation logic belongs here.
fn freeifaddrs(env: &mut Environment, mut ifa: MutPtr<ifaddrs>) {
    while !ifa.is_null() {
        let current = env.mem.read(ifa);
        
        // Освобождаем строку с именем
        if !current.ifa_name.is_null() {
            env.mem.free(current.ifa_name.cast_mut().cast());
        }
        // Освобождаем адреса (преобразуем u32 обратно в указатель)
        if current.ifa_addr != 0 {
            env.mem.free(MutPtr::<u8>::new(current.ifa_addr).cast());
        }
        if current.ifa_netmask != 0 {
            env.mem.free(MutPtr::<u8>::new(current.ifa_netmask).cast());
        }
        if current.ifa_dstaddr != 0 {
            env.mem.free(MutPtr::<u8>::new(current.ifa_dstaddr).cast());
        }
        
        // Запоминаем следующий узел перед удалением текущего
        let next = current.ifa_next;
        env.mem.free(ifa.cast());
        ifa = next;
    }
}

// ---------------------------------------------------------------------------
// net/if.h – interface index / name mapping
// (commonly used together with ifaddrs by network-aware apps)
// ---------------------------------------------------------------------------

/// Maximum length of an interface name including the NUL terminator.
const IF_NAMESIZE: usize = 16;

/// `unsigned int if_nametoindex(const char *ifname)`
///
/// Returns the index for the named interface, or 0 on error.
/// Stub: we have no real interfaces, so always returns 0 / ENXIO.
fn if_nametoindex(env: &mut Environment, ifname: ConstPtr<u8>) -> u32 {
    let name = env.mem.cstr_at_utf8(ifname).unwrap_or("<invalid>");
    log!(
        "TODO: if_nametoindex(\"{}\") – returning 0 (not implemented)",
        name
    );
    set_errno(env, ENXIO);
    0
}

/// `char *if_indextoname(unsigned int ifindex, char *ifname)`
///
/// Writes the name of interface `ifindex` into `ifname` (at least
/// `IF_NAMESIZE` bytes) and returns `ifname`, or NULL on error.
/// Stub: always returns NULL / ENXIO.
fn if_indextoname(env: &mut Environment, ifindex: u32, ifname: MutPtr<u8>) -> MutPtr<u8> {
    log!(
        "TODO: if_indextoname({}) – returning NULL (not implemented)",
        ifindex
    );
    set_errno(env, ENXIO);
    MutPtr::null()
}

// `struct if_nameindex` used by if_nameindex() / if_freenameindex().
#[allow(non_camel_case_types)]
#[repr(C, packed)]
pub struct if_nameindex {
    pub if_index: u32,
    pub if_name: ConstPtr<u8>,
}
unsafe impl SafeRead for if_nameindex {}

/// `struct if_nameindex *if_nameindex(void)`
///
/// Returns an array of all interface name/index pairs terminated by an entry
/// with `if_index == 0` and `if_name == NULL`. Stub: returns NULL / EOPNOTSUPP.
fn if_nameindex(env: &mut Environment) -> MutPtr<if_nameindex> {
    log!("TODO: if_nameindex() – returning NULL (not implemented)");
    set_errno(env, 102 /* EOPNOTSUPP */);
    MutPtr::null()
}

/// `void if_freenameindex(struct if_nameindex *ptr)`
///
/// Frees the array returned by `if_nameindex`. No-op in the stub.
fn if_freenameindex(env: &mut Environment, ptr: MutPtr<if_nameindex>) {
    if !ptr.is_null() {
        log!(
            "TODO: if_freenameindex({:#x}) – not allocated by us, ignoring",
            ptr.to_bits()
        );
    }
}

// ---------------------------------------------------------------------------
// Export table
// ---------------------------------------------------------------------------

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(getifaddrs(_)),
    export_c_func!(freeifaddrs(_)),
    export_c_func!(if_nametoindex(_)),
    export_c_func!(if_indextoname(_, _)),
    export_c_func!(if_nameindex()),
    export_c_func!(if_freenameindex(_)),
];

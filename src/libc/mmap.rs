/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::libc::errno::{set_errno, EINVAL, ENOTSUP};
use crate::libc::posix_io;
use crate::libc::posix_io::{off_t, open_direct, FileDescriptor, SEEK_SET};
use crate::mem::{ConstPtr, GuestUSize, MutVoidPtr, PAGE_SIZE_ALIGN_MASK};
use std::collections::HashMap;

#[allow(dead_code)]
const MAP_FILE: i32 = 0x0000;
const MAP_ANON: i32 = 0x1000;

#[derive(Default)]
pub struct State {
    /// Keeping track of `mmap` allocations
    allocations: HashMap<MutVoidPtr, GuestUSize>,
}

/// For files, our implementation of mmap is really simple:
/// it's just load entirety of file in memory!
fn mmap(
    env: &mut Environment,
    addr: MutVoidPtr,
    len: GuestUSize,
    prot: i32,
    flags: i32,
    fd: FileDescriptor,
    offset: off_t,
) -> MutVoidPtr {
    // TODO: handle errno properly
    set_errno(env, 0);
    log_dbg!(
        "mmap({:?}, {}, {}, {}, {}, {})",
        addr,
        len,
        prot,
        flags,
        fd,
        offset
    );

    // TODO: use vm_allocate() instead
    let ptr = env.mem.calloc(len);

    if (flags & MAP_ANON) != 0 {
        assert!(ptr.to_bits() & PAGE_SIZE_ALIGN_MASK == 0);

        // Убираем жесткие assert_eq!(fd, -1) и assert_eq!(offset, 0).
        // В реальной iOS/Darwin при наличии флага MAP_ANON аргументы fd и
        // offset
        // просто игнорируются ОС. Движки вроде Adobe AIR передают сюда мусор.
        if fd != -1 || offset != 0 {
            log_dbg!("Warning: mmap MAP_ANON called with fd={} and offset={}. Ignoring them as per OS behavior.", fd, offset);
        }

        if !addr.is_null() {
            log!(
                "Warning: mmap MAP_ANON ignoring hint for address {:?}, actual is {:?}",
                addr,
                ptr
            );
        }
    } else {
        assert!(addr.is_null());
        // Смещение файла корректно отрабатывается через lseek
        let new_offset = posix_io::lseek(env, fd, offset, SEEK_SET);
        assert_eq!(new_offset, offset);

        let read = posix_io::read(env, fd, ptr, len);
        assert_eq!(read as u32, len);
    }

    assert!(!env.libc_state.mmap.allocations.contains_key(&ptr));
    env.libc_state.mmap.allocations.insert(ptr, len);

    ptr
}

fn munmap(env: &mut Environment, addr: MutVoidPtr, len: GuestUSize) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    log_dbg!("munmap({:?}, {})", addr, len);

    if len == 0 {
        set_errno(env, EINVAL);
        // TODO: should we clear allocations for `addr` here too?
        log!("Warning: munmap({:?}, {}) failed, returning -1", addr, len);
        return -1;
    }

    assert_eq!(*env.libc_state.mmap.allocations.get(&addr).unwrap(), len);
    env.mem.free(addr);
    env.libc_state.mmap.allocations.remove(&addr);
    0 // success
}

fn madvise(env: &mut Environment, addr: MutVoidPtr, len: GuestUSize, advice: i32) -> i32 {
    log!("TODO: madvise({:?}, {}, {}) -> -1", addr, len, advice);
    set_errno(env, ENOTSUP);
    -1
}

fn shm_open(env: &mut Environment, name: ConstPtr<u8>, oflag: i32, mode: u32) -> i32 {
    set_errno(env, 0);

    let name_str = env.mem.cstr_at_utf8(name).unwrap_or("<invalid>");
    log_dbg!("shm_open({:?}, {:#x}, {:#x})", name_str, oflag, mode);

    // Используем open_direct! Параметр mode для эмулятора здесь не нужен,
    // поэтому просто передаем env, name и oflag.
    open_direct(env, name, oflag)
}

fn mprotect(env: &mut Environment, addr: MutVoidPtr, len: GuestUSize, prot: i32) -> i32 {
    log!("TODO: mprotect({:?}, {}, {}) -> -1", addr, len, prot);
    set_errno(env, ENOTSUP);
    -1
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(mmap(_, _, _, _, _, _)),
    export_c_func!(munmap(_, _)),
    export_c_func!(madvise(_, _, _)),
    export_c_func!(shm_open(_, _, _)),
    export_c_func!(shm_open(_, _, _)),
    export_c_func!(mprotect(_, _, _)),
];

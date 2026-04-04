/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX I/O functions (`fcntl.h`, parts of `unistd.h`, etc)

pub mod stat;
pub mod statvfs;

use crate::abi::DotDotDot;
use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::{GuestFile, GuestOpenOptions, GuestPath};
use crate::libc::errno::{set_errno, EBADF, EINTR, EINVAL, EIO, EISDIR, EOVERFLOW, ESPIPE};
use crate::libc::sys::socket::close_socket;
use crate::libc::unistd::pid_t;
use crate::mem::{
    ConstPtr, ConstVoidPtr, GuestISize, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead,
};
use crate::Environment;
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Default)]
pub struct State {
    /// File descriptors _other than stdin, stdout, and stderr_
    files: Vec<Option<PosixFileHostObject>>,
}
impl State {
    fn file_for_fd(&mut self, fd: FileDescriptor) -> Option<&mut PosixFileHostObject> {
        if fd < NORMAL_FILENO_BASE {
            return None;
        }
        self.files
            .get_mut(fd_to_file_idx(fd))
            .and_then(|file_or_none| file_or_none.as_mut())
    }
}

struct PosixFileHostObject {
    file: GuestFile,
    needs_flush: bool,
    reached_eof: bool,
    flags: i32,
}

// TODO: stdin/stdout/stderr handling somehow
fn file_idx_to_fd(idx: usize) -> FileDescriptor {
    FileDescriptor::try_from(idx)
        .unwrap()
        .checked_add(NORMAL_FILENO_BASE)
        .unwrap()
}
fn fd_to_file_idx(fd: FileDescriptor) -> usize {
    fd.checked_sub(NORMAL_FILENO_BASE).unwrap_or(0) as usize
}

/// File descriptor type. This alias is for readability, POSIX just uses `int`.
pub type FileDescriptor = i32;
pub const STDIN_FILENO: FileDescriptor = 0;
pub const STDOUT_FILENO: FileDescriptor = 1;
pub const STDERR_FILENO: FileDescriptor = 2;
const NORMAL_FILENO_BASE: FileDescriptor = STDERR_FILENO + 1;

/// Flags bitfield for `open`.
/// This alias is for readability, POSIX just uses `int`.
pub type OpenFlag = i32;
pub const O_RDONLY: OpenFlag = 0x0;
pub const O_WRONLY: OpenFlag = 0x1;
pub const O_RDWR: OpenFlag = 0x2;
pub const O_ACCMODE: OpenFlag = O_RDWR | O_WRONLY | O_RDONLY;

pub const O_NONBLOCK: OpenFlag = 0x4;
pub const O_APPEND: OpenFlag = 0x8;
pub const O_SHLOCK: OpenFlag = 0x10;
pub const O_NOFOLLOW: OpenFlag = 0x100;
pub const O_CREAT: OpenFlag = 0x200;
pub const O_TRUNC: OpenFlag = 0x400;
pub const O_EXCL: OpenFlag = 0x800;

/// File control command flags.
/// This alias is for readability, POSIX just uses `int`.
pub type FileControlCommand = i32;
const F_GETFD: FileControlCommand = 1;
const F_SETFD: FileControlCommand = 2;
const F_GETLK: FileControlCommand = 7;
const F_SETLK: FileControlCommand = 8;
const F_RDADVISE: FileControlCommand = 44;
const F_NOCACHE: FileControlCommand = 48;

/// File Descriptor flags.
/// This alias is for readability, POSIX just uses `int`.
pub type FDFlag = i32;
pub const FD_CLOEXEC: FDFlag = 1;

/// Record Locking flags.
/// This alias is for readability, POSIX just uses `short`
pub type RecordLockingFlag = i16;
pub const F_RDLCK: RecordLockingFlag = 1;
pub const F_UNLCK: RecordLockingFlag = 2;
pub const F_WRLCK: RecordLockingFlag = 3;

#[repr(C, packed)]
#[derive(Debug)]
#[allow(non_camel_case_types)]
struct flock {
    start: off_t,
    len: off_t,
    pid: pid_t,
    lock_type: i16,
    whence: i16,
}
unsafe impl SafeRead for flock {}

pub type FLockFlag = i32;
pub const LOCK_SH: FLockFlag = 1;
#[allow(dead_code)]
pub const LOCK_EX: FLockFlag = 2;
#[allow(dead_code)]
pub const LOCK_NB: FLockFlag = 4;
#[allow(dead_code)]
pub const LOCK_UN: FLockFlag = 8;

#[repr(C, packed)]
struct iovec {
    iov_base: ConstPtr<u8>,
    iov_len: GuestUSize,
}
unsafe impl SafeRead for iovec {}

fn open(env: &mut Environment, path: ConstPtr<u8>, flags: i32, _args: DotDotDot) -> FileDescriptor {
    set_errno(env, 0);
    self::open_direct(env, path, flags)
}

pub fn open_direct(env: &mut Environment, path: ConstPtr<u8>, flags: i32) -> FileDescriptor {
    assert!(
        flags
            & !(O_ACCMODE
                | O_NONBLOCK
                | O_APPEND
                | O_SHLOCK
                | O_NOFOLLOW
                | O_CREAT
                | O_TRUNC
                | O_EXCL)
            == 0
    );
    assert!(flags & O_EXCL == 0);

    if path.is_null() {
        log_dbg!("open({:?}, {:#x}) => -1", path, flags);
        return -1;
    }

    let mut needs_flush = false;
    let mut options = GuestOpenOptions::new();
    match flags & O_ACCMODE {
        O_RDONLY => {
            options.read();
        }
        O_WRONLY => {
            options.write();
            needs_flush = true;
        }
        O_RDWR => {
            options.read().write();
            needs_flush = true;
        }
        _ => panic!(),
    };
    if (flags & O_APPEND) != 0 {
        options.append();
    }
    if (flags & O_CREAT) != 0 {
        options.create();
    }
    if (flags & O_TRUNC) != 0 {
        options.truncate();
    }

    let path_string = match env.mem.cstr_at_utf8(path) {
        Ok(path_str) => path_str.to_owned(),
        Err(err) => {
            log!("open() error, unable to treat {:?} as utf8 str: {:?}", path, err);
            return -1;
        }
    };
    
    if flags & O_NOFOLLOW != 0 {
        log!("Ignoring O_NOFOLLOW when opening {:?}", path_string);
    }

    // --- RETINA DEEP CASE-INSENSITIVE FALLBACK ---
    let mut actual_path_string = path_string.clone();

    if !env.fs.exists(GuestPath::new(&actual_path_string)) {
        let is_absolute = actual_path_string.starts_with('/');
        let parts: Vec<&str> = actual_path_string.split('/').filter(|s| !s.is_empty()).collect();
        
        let mut current_path = if is_absolute { String::from("/") } else { String::new() };

        for (i, part) in parts.iter().enumerate() {
            let mut test_path = current_path.clone();
            if !test_path.is_empty() && !test_path.ends_with('/') {
                test_path.push('/');
            }
            test_path.push_str(part);

            // Если текущий кусок пути существует, идем дальше
            if env.fs.exists(GuestPath::new(&test_path)) {
                current_path = test_path;
            } else {
                // Если не существует, ищем его без учета регистра
                let parent_to_search = if current_path.is_empty() { "." } else { &current_path };
                let target_lower = part.to_lowercase();
                let mut found_match = None;

                if let Ok(entries) = env.fs.enumerate(GuestPath::new(parent_to_search)) {
                    for entry in entries {
                        let entry_path = std::path::Path::new(&entry);
                        if let Some(file_name) = entry_path.file_name() {
                            if file_name.to_str().unwrap_or("").to_lowercase() == target_lower {
                                found_match = Some(file_name.to_str().unwrap_or("").to_string());
                                break;
                            }
                        }
                    }
                }

                if let Some(m) = found_match {
                    if !current_path.is_empty() && !current_path.ends_with('/') {
                        current_path.push('/');
                    }
                    current_path.push_str(&m);
                } else {
                    // Если совсем ничего не нашли, восстанавливаем остаток пути и прерываем поиск
                    current_path = test_path;
                    for remaining_part in parts.iter().skip(i + 1) {
                        if !current_path.ends_with('/') {
                            current_path.push('/');
                        }
                        current_path.push_str(remaining_part);
                    }
                    break;
                }
            }
        }
        actual_path_string = current_path;
    }
    // --- КОНЕЦ ПАТЧА ---

    let res = match env
        .fs
        .open_with_options(GuestPath::new(&actual_path_string), options)
    {
        Ok(file) => {
            let host_object = PosixFileHostObject {
                file,
                needs_flush,
                reached_eof: false,
                flags: 0,
            };
            find_or_create_fd(env, host_object)
        }
        Err(()) => {
            -1
        }
    };

    if res != -1 && (flags & O_SHLOCK) != 0 {
        flock(env, res, LOCK_SH);
    }
    log_dbg!("open({:?} {:?}, {:#x}) => {:?}", path, path_string, flags, res);
    res
}

pub fn read(
    env: &mut Environment,
    fd: FileDescriptor,
    buffer: MutVoidPtr,
    size: GuestUSize,
) -> GuestISize {
    set_errno(env, 0);
    if buffer.is_null() {
        return -1;
    }

    let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else {
        log!("Warning: read({:?}, {:?}, {:#x}) called with unknown fd, returning -1", fd, buffer, size);
        set_errno(env, EBADF);
        return -1;
    };

    let buffer_slice = env.mem.bytes_at_mut(buffer.cast(), size);
    match file.file.read(buffer_slice) {
        Ok(bytes_read) => {
            if bytes_read == 0 && size != 0 {
                file.reached_eof = true;
            }
            if bytes_read < buffer_slice.len() {
                log!("Warning: read({:?}, {:?}, {:#x}) read only {:#x} bytes", fd, buffer, size, bytes_read);
            } else {
                log_dbg!("read({:?}, {:?}, {:#x}) => {:#x}", fd, buffer, size, bytes_read);
            }
            bytes_read.try_into().unwrap_or(-1)
        }
        Err(e) => {
            let res = match e.kind() {
                std::io::ErrorKind::IsADirectory => {
                    set_errno(env, EISDIR);
                    0
                }
                _ => {
                    -1
                }
            };
            log!("Warning: read({:?}, {:?}, {:#x}) encountered error {:?}, returning {}", fd, buffer, size, e, res);
            res
        }
    }
}

pub fn pread(
    env: &mut Environment,
    fd: FileDescriptor,
    buffer: MutVoidPtr,
    size: GuestUSize,
    offset: off_t,
) -> GuestISize {
    let original_position = lseek(env, fd, 0, SEEK_CUR);
    if original_position == -1 {
        return -1;
    }

    if lseek(env, fd, offset, SEEK_SET) == -1 {
        return -1;
    }

    let bytes_read = read(env, fd, buffer, size);

    assert!(lseek(env, fd, original_position, SEEK_SET) != -1);
    bytes_read
}

pub(super) fn eof(env: &mut Environment, fd: FileDescriptor) -> i32 {
    let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else {
        return 1;
    };
    if file.reached_eof { 1 } else { 0 }
}

pub(super) fn clearerr(env: &mut Environment, fd: FileDescriptor) {
    set_errno(env, 0);
    if let Some(file) = env.libc_state.posix_io.file_for_fd(fd) {
        file.reached_eof = false;
    }
}

pub(super) fn fflush(env: &mut Environment, fd: FileDescriptor) -> i32 {
    set_errno(env, 0);
    let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else {
        return -1;
    };
    match file.file.flush() {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

pub fn write(
    env: &mut Environment,
    fd: FileDescriptor,
    buffer: ConstVoidPtr,
    size: GuestUSize,
) -> GuestISize {
    set_errno(env, 0);
    // ПЕРЕХВАТ КОНСОЛИ! Ловим stdout и stderr от Unity.
    if fd == STDOUT_FILENO ||
        fd == STDERR_FILENO {
        let buffer_slice = env.mem.bytes_at(buffer.cast(), size);
        let msg = String::from_utf8_lossy(buffer_slice);
        print!("{}", msg);
        return size as GuestISize;
    }

    let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else {
        set_errno(env, EBADF);
        return -1;
    };

    let buffer_slice = env.mem.bytes_at(buffer.cast(), size);
    match file.file.write(buffer_slice) {
        Ok(bytes_written) => {
            if bytes_written < buffer_slice.len() {
                log!("Warning: write({:?}, {:?}, {:#x}) wrote only {:#x} bytes", fd, buffer, size, bytes_written);
            } else {
                log_dbg!("write({:?}, {:?}, {:#x}) => {:#x}", fd, buffer, size, bytes_written);
            }
            bytes_written.try_into().unwrap_or(-1)
        }
        Err(e) => {
            log!("Warning: write({:?}, {:?}, {:#x}) encountered error {:?}, returning -1", fd, buffer, size, e);
            -1
        }
    }
}

pub fn pwrite(
    env: &mut Environment,
    fd: FileDescriptor,
    buffer: ConstVoidPtr,
    size: GuestUSize,
    offset: off_t,
) -> GuestISize {
    let original_position = lseek(env, fd, 0, SEEK_CUR);
    if original_position == -1 {
        return -1;
    }
    if lseek(env, fd, offset, SEEK_SET) == -1 {
        return -1;
    }
    let bytes_written = write(env, fd, buffer, size);
    assert!(lseek(env, fd, original_position, SEEK_SET) != -1);
    bytes_written
}

#[allow(non_camel_case_types)]
pub type off_t = i64;
pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;
pub fn lseek(env: &mut Environment, fd: FileDescriptor, offset: off_t, whence: i32) -> off_t {
    let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else {
        log!("lseek({:?}, {:#x}, {}) => {}", fd, offset, whence, -1);
        set_errno(env, EBADF);
        return -1;
    };

    if !file.file.is_seekable() {
        log!("Warning: lseek({:?}, {:#x}, {}) => -1. Called with unseekable fd.", fd, offset, whence);
        set_errno(env, ESPIPE);
        return -1;
    }

    let start_position = match whence {
        SEEK_SET => 0,
        SEEK_CUR => match file.file.stream_position() {
            Ok(pos) => pos,
            Err(seek_error) => {
                match seek_error.kind() {
                   
                    std::io::ErrorKind::IsADirectory => set_errno(env, EISDIR),
                    _ => unimplemented!("Unexpected seek error {:?}", seek_error),
                }
                return -1;
            }
        },
        SEEK_END => match file.file.stream_len() {
            Ok(len) => len,
            Err(seek_error) => {
                match seek_error.kind() {
                    std::io::ErrorKind::IsADirectory => set_errno(env, EISDIR),
           
                    _ => unimplemented!("Unexpected seek error {:?}", seek_error),
                }
                return -1;
            }
        },
        _ => {
            log!("Warning: lseek({:?}, {:#x}, {}) => -1. Called with invalid \"whence\".", fd, offset, whence);
            set_errno(env, EINVAL);
            return -1;
        }
    };

    let seek_position = match start_position.checked_add_signed(offset) {
        Some(position) => position,
        None => {
            let (error_msg, errno) = if offset >= 0 {
                ("Seek position does not fit in off_t.", EOVERFLOW)
            } else {
         
               ("Negative seek position.", EINVAL)
            };
            log!("Warning: lseek({:?}, {:#x}, {}) => -1. {}", fd, offset, whence, error_msg);
            set_errno(env, errno);
            return -1;
        }
    };
    if seek_position > off_t::MAX as u64 {
        log!("Warning: lseek({:?}, {:#x}, {}) => -1. Seek position does not fit in off_t.", fd, offset, whence);
        set_errno(env, EOVERFLOW);
        return -1;
    }

    let res = match file.file.seek(SeekFrom::Start(seek_position)) {
        Ok(new_offset) => {
            file.reached_eof = false;
            new_offset.try_into().unwrap_or(-1)
        }
        Err(seek_error) => {
            match seek_error.kind() {
                std::io::ErrorKind::InvalidInput => set_errno(env, EINVAL),
                std::io::ErrorKind::IsADirectory => set_errno(env, EISDIR),
                _ => unimplemented!("Unexpected seek error {:?}", seek_error),
        
            }
            log!("Warning: lseek({:?}, {:#x}, {}) failed with error: {:?}, returning -1", fd, offset, whence, seek_error);
            return -1;
        }
    };
    log_dbg!("lseek({:?}, {:#x}, {}) => {}", fd, offset, whence, res);
    res
}

pub fn close(env: &mut Environment, fd: FileDescriptor) -> i32 {
    set_errno(env, 0);
    if matches!(fd, STDIN_FILENO | STDOUT_FILENO | STDERR_FILENO) {
        log_dbg!("close({:?}) => 0", fd);
        return 0;
    }

    if fd < 0 ||
        env.libc_state.posix_io.files.get(fd_to_file_idx(fd)).is_none() {
        set_errno(env, EBADF);
        log!("Warning: close({:?}) failed, returning -1", fd);
        return -1;
    }

    let result = match env.libc_state.posix_io.files[fd_to_file_idx(fd)].take() {
        Some(file) => {
            match file.file {
                GuestFile::Directory => 0,
                GuestFile::Socket => {
                    close_socket(env, fd);
                    0
                }
                _ => {
                    if !file.needs_flush {
                        0
                    
                    } else {
                        match file.file.sync_all() {
                            Ok(()) => 0,
                            Err(_) => -1
            
                        }
                    }
                }
            }
        }
        None => {
            set_errno(env, EBADF);
            -1
        }
    };
    if result == 0 {
        log_dbg!("close({:?}) => 0", fd);
    } else {
        log!("Warning: close({:?}) failed, returning -1", fd);
    }
    result
}

fn rename(env: &mut Environment, old: ConstPtr<u8>, new: ConstPtr<u8>) -> i32 {
    set_errno(env, 0);
    let old_str = env.mem.cstr_at_utf8(old).unwrap_or_default();
    let new_str = env.mem.cstr_at_utf8(new).unwrap_or_default();
    let res = match env.fs.rename(GuestPath::new(&old_str), GuestPath::new(&new_str)) {
        Ok(_) => 0,
        Err(_) => -1,
    };
    log_dbg!("rename('{}', '{}') => {}", old_str, new_str, res);
    res
}

pub fn getcwd(env: &mut Environment, buf_ptr: MutPtr<u8>, buf_size: GuestUSize) -> MutPtr<u8> {
    let working_directory = env.fs.working_directory();
    if !env.fs.is_dir(working_directory) {
        log!("Warning: getcwd({:?}, {:#x}) failed, returning NULL", buf_ptr, buf_size);
        return Ptr::null();
    }

    let working_directory = env.fs.working_directory().as_str().as_bytes();

    if buf_ptr.is_null() {
        let res = env.mem.alloc_and_write_cstr(working_directory);
        log_dbg!("getcwd(NULL, _) => {:?} ({:?})", res, working_directory);
        return res;
    }

    let res_size: GuestUSize = u32::try_from(working_directory.len()).unwrap_or(0) + 1;
    if buf_size < res_size {
        log!("Warning: getcwd({:?}, {:#x}) failed, returning NULL", buf_ptr, buf_size);
        return Ptr::null();
    }

    let buf = env.mem.bytes_at_mut(buf_ptr, res_size);
    buf[..(res_size - 1) as usize].copy_from_slice(working_directory);
    buf[(res_size - 1) as usize] = b'\0';

    log_dbg!("getcwd({:?}, {:#x}) => {:?}, wrote {:?} ({:#x} bytes)", buf_ptr, buf_size, buf_ptr, working_directory, res_size);
    buf_ptr
}

fn chdir(env: &mut Environment, path_ptr: ConstPtr<u8>) -> i32 {
    set_errno(env, 0);

    let path_str = env.mem.cstr_at_utf8(path_ptr).unwrap_or_default();
    let path = GuestPath::new(&path_str);
    match env.fs.change_working_directory(path) {
        Ok(new) => {
            log_dbg!("chdir({:?}) => 0, new working directory: {:?}", path_ptr, new);
            0
        }
        Err(()) => {
            log!("Warning: chdir({:?}) failed, could not change working directory to {:?}, returning -1", path_ptr, path);
            -1
        }
    }
}

fn fcntl(
    env: &mut Environment,
    fd: FileDescriptor,
    cmd: FileControlCommand,
    args: DotDotDot,
) -> i32 {
    set_errno(env, 0);
    if fd >= NORMAL_FILENO_BASE && env.libc_state.posix_io.files.get(fd_to_file_idx(fd)).is_none() {
        set_errno(env, EBADF);
        return -1;
    }

    match cmd {
        F_GETFD => {
            let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else { return -1;
            };
            return file.flags;
        }
        F_SETFD => {
            let flags: i32 = args.start().next(env);
            assert!(matches!(flags, FD_CLOEXEC | 0));
            if flags & FD_CLOEXEC == FD_CLOEXEC {
                log!("TODO: fcntl({}, F_SETFD, {}) called. CLOEXEC currently not supported.", fd, flags);
            }
            if let Some(file) = env.libc_state.posix_io.file_for_fd(fd) {
                file.flags = flags;
            }
        }
        F_GETLK => {
            let lock_ptr: MutPtr<flock> = args.start().next(env);
            let mut lock = env.mem.read(lock_ptr);

            if let Err(error_code) = validate_lock(env, fd, &lock) {
                set_errno(env, error_code);
                return -1;
            }

            log!("TODO: fcntl({}, F_GETLK, {:?}) called. Locking unimplemented, any conflicts will be unreported.", fd, lock);
            lock.lock_type = F_UNLCK;
            env.mem.write(lock_ptr, lock);
        }
        F_SETLK => {
            let lock_ptr: MutPtr<flock> = args.start().next(env);
            let lock = env.mem.read(lock_ptr);

            if let Err(error_code) = validate_lock(env, fd, &lock) {
                set_errno(env, error_code);
                return -1;
            }

            log!("TODO: fcntl({}, F_SETLK, {:?}) called. Locking unimplemented, ignoring lock.", fd, lock);
        }
        F_NOCACHE => {
            let mut args = args.start();
            let arg: i32 = args.next(env);
            assert_eq!(arg, 1);
            log!("TODO: Ignoring enabling F_NOCACHE for file descriptor {}", fd);
        }
        F_RDADVISE => {
            log_dbg!("TODO: Ignoring F_RDADVISE for file descriptor {}", fd);
        }
        _ => unimplemented!(),
    }
    0 // success
}

fn flock(env: &mut Environment, fd: FileDescriptor, operation: FLockFlag) -> i32 {
    set_errno(env, 0);
    log!("TODO: flock({:?}, {:?})", fd, operation);
    0
}

fn fsync(env: &mut Environment, fd: FileDescriptor) -> i32 {
    let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else {
        log!("Warning: fsync({:?}) called with unknown fd, returning -1", fd);
        set_errno(env, EBADF);
        return -1;
    };

    match file.file.sync_all() {
        Ok(()) => 0,
        Err(error) => {
            match error.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    log!("Warning: fsync({:?}) sync failed with error: {:?}, returning 0", fd, error);
                    return 0;
                }
                std::io::ErrorKind::Unsupported => set_errno(env, EINVAL),
                std::io::ErrorKind::Interrupted => set_errno(env, EINTR),
                _ => set_errno(env, EIO),
            }

            log!("Warning: fsync({:?}) sync failed with error: {:?}, returning -1", fd, error);
            -1
        }
    }
}

fn ftruncate(env: &mut Environment, fd: FileDescriptor, len: off_t) -> i32 {
    set_errno(env, 0);
    let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else {
        set_errno(env, EBADF);
        return -1;
    };
    match file.file.set_len(len as u64) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

fn writev(
    env: &mut Environment,
    fd: FileDescriptor,
    iov: ConstPtr<iovec>,
    iovcnt: i32,
) -> GuestISize {
    let mut i = 0;
    let mut written_bytes: GuestISize = 0;
    while i != iovcnt {
        let iovec = env.mem.read(iov + i as u32);
        let bytes_written = write(env, fd, iovec.iov_base.cast(), iovec.iov_len);
        if bytes_written == -1 {
            return -1;
        }
        written_bytes += bytes_written;
        i += 1
    }
    written_bytes
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(open(_, _, _)),
    export_c_func!(read(_, _, _)),
    export_c_func!(pread(_, _, _, _)),
    export_c_func!(write(_, _, _)),
    export_c_func!(pwrite(_, _, _, _)),
    export_c_func!(lseek(_, _, _)),
    export_c_func!(close(_)),
    export_c_func!(rename(_, _)),
    export_c_func!(getcwd(_, _)),
    export_c_func!(chdir(_)),
    export_c_func!(fcntl(_, _, _)),
    export_c_func!(flock(_, 
_)),
    export_c_func!(fsync(_)),
    export_c_func!(ftruncate(_, _)),
    export_c_func!(writev(_, _, _)),
];

fn find_or_create_fd(env: &mut Environment, host_object: PosixFileHostObject) -> FileDescriptor {
    let idx = if let Some(free_idx) = env.libc_state.posix_io.files.iter().position(|f| f.is_none()) {
        env.libc_state.posix_io.files[free_idx] = Some(host_object);
        free_idx
    } else {
        let idx = env.libc_state.posix_io.files.len();
        env.libc_state.posix_io.files.push(Some(host_object));
        idx
    };
    file_idx_to_fd(idx)
}

pub fn find_or_create_socket(env: &mut Environment) -> FileDescriptor {
    let host_object = PosixFileHostObject {
        file: GuestFile::Socket,
        needs_flush: false,
        reached_eof: false,
        flags: 0,
    };
    find_or_create_fd(env, host_object)
}

pub fn is_socket(env: &mut Environment, fd: FileDescriptor) -> bool {
    if fd < NORMAL_FILENO_BASE {
        return false;
    }
    if let Some(Some(file_obj)) = env.libc_state.posix_io.files.get(fd_to_file_idx(fd)) {
        matches!(file_obj.file, GuestFile::Socket)
    } else {
        false
    }
}

fn validate_lock(env: &mut Environment, fd: FileDescriptor, lock: &flock) -> Result<(), i32> {
    let lock_type = lock.lock_type;
    if !matches!(lock_type, F_RDLCK | F_UNLCK | F_WRLCK) {
        return Err(EINVAL);
    }

    let whence = lock.whence as i32;
    let lock_start = match whence {
        SEEK_SET => lock.start,
        SEEK_CUR => {
            let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else { return Err(EBADF);
            };
            let file_position = file.file.stream_position().unwrap_or(0);
            file_position as i64 + lock.start
        }
        SEEK_END => {
            let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else { return Err(EBADF);
            };
            let size: i64 = file.file.stream_len().unwrap_or(0).try_into().unwrap_or(0);
            size + lock.start
        }
        _ => {
            return Err(EINVAL);
        }
    };

    if lock_start < 0 {
        return Err(EINVAL);
    }

    Ok(())
}


/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `NSFileHandle`.

use super::ns_string;
use super::NSUInteger;
use crate::libc::posix_io;
use crate::mem::{ConstPtr, ConstVoidPtr};
use crate::objc::{
   autorelease, id, msg, msg_class, nil, objc_classes, release, retain,
   ClassExports, HostObject, NSZonePtr,
};

struct NSFileHandleHostObject {
    fd: posix_io::FileDescriptor,
}
impl HostObject for NSFileHandleHostObject {}

struct NSPipeHostObject {
   /// NSFileHandle* for the reading end — retained.
   file_handle_for_reading: id,
   /// NSFileHandle* for the writing end — retained.
   file_handle_for_writing: id,
}
impl HostObject for NSPipeHostObject {}

// =========================================================================
// MARK: - NSTask host object
// =========================================================================

type NSTaskTerminationReason = i32;
#[allow(dead_code)]
const NSTaskTerminationReasonExit:             NSTaskTerminationReason = 1;
#[allow(dead_code)]
const NSTaskTerminationReasonUncaughtSignal:   NSTaskTerminationReason = 2;

struct NSTaskHostObject {
   /// NSString* — path to the executable. Retained.
   launch_path: id,
   /// NSArray<NSString*>* — launch arguments. Retained.
   arguments: id,
   /// NSDictionary<NSString*,NSString*>* — environment. Retained.
   environment: id,
   /// NSString* — current directory. Retained.
   current_directory_path: id,
   /// id — standard input (NSPipe* or NSFileHandle*). Retained.
   standard_input: id,
   /// id — standard output. Retained.
   standard_output: id,
   /// id — standard error. Retained.
   standard_error: id,
   /// Whether the task has been launched.
   is_running: bool,
   /// Exit status (valid only after task terminates).
   termination_status: i32,
   /// Termination reason.
   termination_reason: NSTaskTerminationReason,
   /// Termination handler block — retained.
   termination_handler: id,
}
impl HostObject for NSTaskHostObject {}

// =========================================================================
// MARK: - NSCache host object
// =========================================================================

struct NSCacheHostObject {
   /// Backing store — NSMutableDictionary* retained.
   dict: id,
   /// NSString* — cache name. Retained.
   name: id,
   /// Maximum object count (0 = unlimited).
   count_limit: u32,
   /// Maximum total cost (0 = unlimited).
   total_cost_limit: u32,
   /// Whether evicted objects are sent -cache:willEvictObject:.
   evicts_objects_with_discarded_content: bool,
   /// Delegate — weak.
   delegate: id,
}
impl HostObject for NSCacheHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSFileHandle: NSObject

+ (id)fileHandleForReadingAtPath:(id)path { // NSString*
    log_dbg!("fileHandleForReadingAtPath {}", ns_string::to_rust_string(env, path));
    let path_str: ConstPtr<u8> = msg![env; path UTF8String];
    match posix_io::open_direct(env, path_str, posix_io::O_RDONLY) {
        -1 => nil,
        fd => {
            let host_object = Box::new(NSFileHandleHostObject {
                fd
            });
            let new = env.objc.alloc_object(this, host_object, &mut env.mem);
            autorelease(env, new)
        },
    }
}

+ (id)fileHandleForWritingAtPath:(id)path { // NSString*
    log_dbg!("fileHandleForWritingAtPath {}", ns_string::to_rust_string(env, path));
    let path_str: ConstPtr<u8> = msg![env; path UTF8String];
    match posix_io::open_direct(env, path_str, posix_io::O_WRONLY) {
        -1 => nil,
        fd => {
            let host_object = Box::new(NSFileHandleHostObject {
                fd
            });
            let new = env.objc.alloc_object(this, host_object, &mut env.mem);
            autorelease(env, new)
        },
    }
}

+ (id)fileHandleForUpdatingAtPath:(id)path { // NSString*
    log_dbg!("fileHandleForUpdatingAtPath {}", ns_string::to_rust_string(env, path));
    let path_str: ConstPtr<u8> = msg![env; path UTF8String];
    match posix_io::open_direct(env, path_str, posix_io::O_RDWR) {
        -1 => nil,
        fd => {
            let host_object = Box::new(NSFileHandleHostObject {
                fd
            });
            let new = env.objc.alloc_object(this, host_object, &mut env.mem);
            autorelease(env, new)
        },
    }
}

- (i32)fileDescriptor {
    env.objc.borrow::<NSFileHandleHostObject>(this).fd
}

- (i64)offsetInFile {
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    match posix_io::lseek(env, fd, 0, posix_io::SEEK_CUR) {
        -1 => {
            log!("Warning: NSFileHandle offsetInFile failed for fd {}; returning 0.", fd);
            0
        }
        // TODO: What's the correct behaviour if the position is beyond 2GiB?
        cur_pos => cur_pos,
    }
}

- (())seekToFileOffset:(i64)offset {
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    if posix_io::lseek(env, fd, offset, posix_io::SEEK_SET) == -1 {
        log!(
            "Warning: NSFileHandle seekToFileOffset:{} failed for fd {}; ignoring.",
            offset, fd
        );
    }
}

- (i64)seekToEndOfFile {
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    match posix_io::lseek(env, fd, 0, posix_io::SEEK_END) {
        -1 => {
            log!("Warning: NSFileHandle seekToEndOfFile failed for fd {}; returning 0.", fd);
            0
        }
        cur_pos => cur_pos,
    }
}

- (id)readDataOfLength:(NSUInteger)length { // NSData*
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    let buffer = env.mem.alloc(length);
    match posix_io::read(env, fd, buffer, length) {
        -1 => {
            log!(
                "Warning: NSFileHandle readDataOfLength:{} failed for fd {}; \
                 returning empty NSData.",
                length, fd
            );
            env.mem.free(buffer);
            msg_class![env; NSData data]
        }
        bytes_read => {
            let bytes_read_u32: NSUInteger = bytes_read.try_into().unwrap_or(0);
            if bytes_read_u32 != length {
                log!(
                    "Warning: NSFileHandle readDataOfLength: short read on fd {} \
                     (wanted {}, got {}); returning what was read.",
                    fd, length, bytes_read_u32
                );
            }
            msg_class![env; NSData dataWithBytesNoCopy:buffer length:bytes_read_u32]
        }
    }
}

- (id)readDataToEndOfFile {
    let offset: i64 = msg![env; this offsetInFile];
    let eof: i64 = msg![env; this seekToEndOfFile];
    let _: () = msg![env; this seekToFileOffset:offset];
    let delta = eof.saturating_sub(offset);
    let Ok(length): Result<NSUInteger, _> = delta.try_into() else {
        log!(
            "Warning: NSFileHandle readDataToEndOfFile: negative or oversize range \
             ({} bytes); returning empty NSData.",
            delta
        );
        return msg_class![env; NSData data];
    };

    msg![env; this readDataOfLength:length]
}

- (id)availableData {
    // TODO: support non-files too
    msg![env; this readDataToEndOfFile]
}

- (())writeData:(id)data { // NSData *
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    let bytes: ConstVoidPtr = msg![env; data bytes];
    let length: NSUInteger = msg![env; data length];
    if posix_io::write(env, fd, bytes, length) == -1 {
        log!("Warning: NSFileHandle writeData: failed for fd {}; ignoring.", fd);
    }
}

- (())synchronizeFile {
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    // Causes all in-memory data and attributes of the file represented by the
    // handle to write to permanent storage.
    if posix_io::fsync(env, fd) == -1 {
        log_dbg!("synchronizeFile: fsync failed for fd {}", fd);
    }
}

- (())truncateFileAtOffset:(i64)offset {
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    // Truncates or extends the file represented by the file handle to a
    // specified offset
    if posix_io::ftruncate(env, fd, offset) == -1 {
        log!(
            "Warning: NSFileHandle truncateFileAtOffset:{} failed for fd {}; ignoring.",
            offset, fd
        );
    }
}

- (())closeFile {
    // file is closed on dealloc
    // TODO: keep closed state and raise an exception
    // if handle is used after the closing
}

- (())dealloc {
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    posix_io::close(env, fd);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end
    
@implementation NSPipe: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
   let host_object = Box::new(NSPipeHostObject {
       file_handle_for_reading: nil,
       file_handle_for_writing: nil,
   });
   env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)pipe {
   let inst: id = msg![env; this alloc];
   let inst: id = msg![env; inst init];
   crate::objc::autorelease(env, inst)
}

- (id)init {
   // Create stub file handles for both ends.
   let reading: id = msg_class![env; NSFileHandle alloc];
   let reading: id = msg![env; reading init];
   retain(env, reading);

   let writing: id = msg_class![env; NSFileHandle alloc];
   let writing: id = msg![env; writing init];
   retain(env, writing);

   {
       let h = env.objc.borrow_mut::<NSPipeHostObject>(this);
       h.file_handle_for_reading = reading;
       h.file_handle_for_writing = writing;
   }
   this
}

- (())dealloc {
   let (r, w) = {
       let h = env.objc.borrow::<NSPipeHostObject>(this);
       (h.file_handle_for_reading, h.file_handle_for_writing)
   };
   release(env, r);
   release(env, w);
   env.objc.dealloc_object(this, &mut env.mem)
}

- (id)fileHandleForReading { // NSFileHandle*
   env.objc.borrow::<NSPipeHostObject>(this).file_handle_for_reading
}

- (id)fileHandleForWriting { // NSFileHandle*
   env.objc.borrow::<NSPipeHostObject>(this).file_handle_for_writing
}

- (id)description {
   let s = format!("<NSPipe: {:?}>", this);
   let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
   msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

// =========================================================================
// NSTask
// =========================================================================

@implementation NSTask: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
   let host_object = Box::new(NSTaskHostObject {
       launch_path:              nil,
       arguments:                nil,
       environment:              nil,
       current_directory_path:   nil,
       standard_input:           nil,
       standard_output:          nil,
       standard_error:           nil,
       is_running:               false,
       termination_status:       0,
       termination_reason:       NSTaskTerminationReasonExit,
       termination_handler:      nil,
   });
   env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)launchedTaskWithLaunchPath:(id)path arguments:(id)args {
   let task: id = msg![env; this alloc];
   let task: id = msg![env; task init];
   let _: () = msg![env; task setLaunchPath:path];
   let _: () = msg![env; task setArguments:args];
   let _: () = msg![env; task launch];
   crate::objc::autorelease(env, task)
}

- (id)init {
   // Default current directory to "/".
   let default_dir = ns_string::get_static_str(env, "/");
   retain(env, default_dir);
   env.objc.borrow_mut::<NSTaskHostObject>(this)
       .current_directory_path = default_dir;
   this
}

- (())dealloc {
   let h = env.objc.borrow::<NSTaskHostObject>(this);
   let (lp, args, env_dict, cwd, si, so, se, th) = (
       h.launch_path,
       h.arguments,
       h.environment,
       h.current_directory_path,
       h.standard_input,
       h.standard_output,
       h.standard_error,
       h.termination_handler,
   );
   release(env, lp);
   release(env, args);
   release(env, env_dict);
   release(env, cwd);
   release(env, si);
   release(env, so);
   release(env, se);
   release(env, th);
   env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Configuration

- (id)launchPath { // NSString*
   env.objc.borrow::<NSTaskHostObject>(this).launch_path
}
- (())setLaunchPath:(id)path {
   let old = env.objc.borrow::<NSTaskHostObject>(this).launch_path;
   release(env, old);
   retain(env, path);
   env.objc.borrow_mut::<NSTaskHostObject>(this).launch_path = path;
}

- (id)arguments { // NSArray*
   env.objc.borrow::<NSTaskHostObject>(this).arguments
}
- (())setArguments:(id)args {
   let old = env.objc.borrow::<NSTaskHostObject>(this).arguments;
   release(env, old);
   retain(env, args);
   env.objc.borrow_mut::<NSTaskHostObject>(this).arguments = args;
}

- (id)environment { // NSDictionary*
   env.objc.borrow::<NSTaskHostObject>(this).environment
}
- (())setEnvironment:(id)env_dict {
   let old = env.objc.borrow::<NSTaskHostObject>(this).environment;
   release(env, old);
   retain(env, env_dict);
   env.objc.borrow_mut::<NSTaskHostObject>(this).environment = env_dict;
}

- (id)currentDirectoryPath { // NSString*
   env.objc.borrow::<NSTaskHostObject>(this).current_directory_path
}
- (())setCurrentDirectoryPath:(id)path {
   let old = env.objc.borrow::<NSTaskHostObject>(this).current_directory_path;
   release(env, old);
   retain(env, path);
   env.objc.borrow_mut::<NSTaskHostObject>(this).current_directory_path = path;
}

- (id)standardInput  { env.objc.borrow::<NSTaskHostObject>(this).standard_input }
- (())setStandardInput:(id)v {
   let old = env.objc.borrow::<NSTaskHostObject>(this).standard_input;
   release(env, old); retain(env, v);
   env.objc.borrow_mut::<NSTaskHostObject>(this).standard_input = v;
}

- (id)standardOutput { env.objc.borrow::<NSTaskHostObject>(this).standard_output }
- (())setStandardOutput:(id)v {
   let old = env.objc.borrow::<NSTaskHostObject>(this).standard_output;
   release(env, old); retain(env, v);
   env.objc.borrow_mut::<NSTaskHostObject>(this).standard_output = v;
}

- (id)standardError  { env.objc.borrow::<NSTaskHostObject>(this).standard_error }
- (())setStandardError:(id)v {
   let old = env.objc.borrow::<NSTaskHostObject>(this).standard_error;
   release(env, old); retain(env, v);
   env.objc.borrow_mut::<NSTaskHostObject>(this).standard_error = v;
}

- (id)terminationHandler {
   env.objc.borrow::<NSTaskHostObject>(this).termination_handler
}
- (())setTerminationHandler:(id)block {
   let old = env.objc.borrow::<NSTaskHostObject>(this).termination_handler;
   release(env, old);
   retain(env, block);
   env.objc.borrow_mut::<NSTaskHostObject>(this).termination_handler = block;
}

// MARK: Execution

- (())launch {
   let path = env.objc.borrow::<NSTaskHostObject>(this).launch_path;
   let path_str = if path != nil {
       ns_string::to_rust_string(env, path).into_owned()
   } else {
       "(nil)".into()
   };
   log!(
       "Warning: NSTask launch: {:?} — subprocess execution not supported in touchHLE",
       path_str
   );
   // Mark as immediately terminated with status 0.
   {
       let h = env.objc.borrow_mut::<NSTaskHostObject>(this);
       h.is_running          = false;
       h.termination_status  = 0;
       h.termination_reason  = NSTaskTerminationReasonExit;
   }
   // Call the termination handler if set.
   let handler = env.objc.borrow::<NSTaskHostObject>(this).termination_handler;
   if handler != nil {
       let sel = env.objc.lookup_selector("invokeWithId:")
           .unwrap_or_else(|| env.objc.register_host_selector(
               "invokeWithId:".to_string(), &mut env.mem));
       let responds: bool = msg![env; handler respondsToSelector:sel];
       if responds {
           let _: () = msg![env; handler invokeWithId:this];
       } else {
           let sel_plain = env.objc.lookup_selector("invoke")
               .unwrap_or_else(|| env.objc.register_host_selector(
                   "invoke".to_string(), &mut env.mem));
           let responds_plain: bool = msg![env; handler respondsToSelector:sel_plain];
           if responds_plain {
               let _: () = msg![env; handler invoke];
           }
       }
   }
}

- (())interrupt {
   log_dbg!("NSTask interrupt — no-op (task already terminated in stub)");
   env.objc.borrow_mut::<NSTaskHostObject>(this).is_running = false;
}

- (())terminate {
   log_dbg!("NSTask terminate — no-op");
   env.objc.borrow_mut::<NSTaskHostObject>(this).is_running = false;
}

- (())waitUntilExit {
   // Task already "terminated" synchronously in launch — no-op.
   log_dbg!("NSTask waitUntilExit — no-op");
}

- (bool)isRunning {
   env.objc.borrow::<NSTaskHostObject>(this).is_running
}

- (i32)terminationStatus {
   env.objc.borrow::<NSTaskHostObject>(this).termination_status
}

- (NSTaskTerminationReason)terminationReason {
   env.objc.borrow::<NSTaskHostObject>(this).termination_reason
}

// MARK: Description

- (id)description {
   let (is_running, status) = {
       let h = env.objc.borrow::<NSTaskHostObject>(this);
       (h.is_running, h.termination_status)
   };
   let s = format!(
       "<NSTask: {:?}; running={}; status={}>",
       this, is_running, status
   );
   let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
   msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

// =========================================================================
// NSCache
// =========================================================================

@implementation NSCache: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
   let host_object = Box::new(NSCacheHostObject {
       dict:                                   nil,
       name:                                   nil,
       count_limit:                            0,
       total_cost_limit:                       0,
       evicts_objects_with_discarded_content:  true,
       delegate:                               nil,
   });
   env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
   let dict: id = msg_class![env; NSMutableDictionary new];
   retain(env, dict);
   let empty_name = ns_string::get_static_str(env, "");
   retain(env, empty_name);
   {
       let h = env.objc.borrow_mut::<NSCacheHostObject>(this);
       h.dict = dict;
       h.name = empty_name;
   }
   this
}

- (())dealloc {
   let (dict, name) = {
       let h = env.objc.borrow::<NSCacheHostObject>(this);
       (h.dict, h.name)
   };
   release(env, dict);
   release(env, name);
   env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Name

- (id)name { // NSString*
   env.objc.borrow::<NSCacheHostObject>(this).name
}
- (())setName:(id)name {
   let old = env.objc.borrow::<NSCacheHostObject>(this).name;
   release(env, old);
   retain(env, name);
   env.objc.borrow_mut::<NSCacheHostObject>(this).name = name;
}

// MARK: Limits

- (u32)countLimit {
   env.objc.borrow::<NSCacheHostObject>(this).count_limit
}
- (())setCountLimit:(u32)limit {
   env.objc.borrow_mut::<NSCacheHostObject>(this).count_limit = limit;
}

- (u32)totalCostLimit {
   env.objc.borrow::<NSCacheHostObject>(this).total_cost_limit
}
- (())setTotalCostLimit:(u32)limit {
   env.objc.borrow_mut::<NSCacheHostObject>(this).total_cost_limit = limit;
}

- (bool)evictsObjectsWithDiscardedContent {
   env.objc.borrow::<NSCacheHostObject>(this).evicts_objects_with_discarded_content
}
- (())setEvictsObjectsWithDiscardedContent:(bool)v {
   env.objc.borrow_mut::<NSCacheHostObject>(this).evicts_objects_with_discarded_content = v;
}

// MARK: Delegate

- (id)delegate {
   env.objc.borrow::<NSCacheHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
   // Weak — no retain.
   env.objc.borrow_mut::<NSCacheHostObject>(this).delegate = delegate;
}

// MARK: Access

- (id)objectForKey:(id)key {
   let dict = env.objc.borrow::<NSCacheHostObject>(this).dict;
   if dict == nil || key == nil { return nil; }
   msg![env; dict objectForKey:key]
}

- (())setObject:(id)obj forKey:(id)key {
   let dict = env.objc.borrow::<NSCacheHostObject>(this).dict;
   if dict == nil || key == nil || obj == nil { return; }
   // Enforce count limit if set.
   let (limit, total_cost_limit) = {
       let h = env.objc.borrow::<NSCacheHostObject>(this);
       (h.count_limit, h.total_cost_limit)
   };
   if limit > 0 {
       let count: u32 = msg![env; dict count];
       if count >= limit {
           log_dbg!("NSCache setObject:forKey: count limit {} reached — evicting oldest", limit);
           // Evict first object (NSDictionary has no ordering guarantee, but
           // this at least prevents unbounded growth).
           let all_keys: id = msg![env; dict allKeys];
           let first_key: id = msg![env; all_keys objectAtIndex:0u32];
           let _: () = msg![env; dict removeObjectForKey:first_key];
       }
   }
   let _: () = msg![env; dict setObject:obj forKey:key];
}

- (())setObject:(id)obj forKey:(id)key cost:(u32)_cost {
   // Cost tracking is not implemented — delegate to the costless variant.
   let _: () = msg![env; this setObject:obj forKey:key];
}

- (())removeObjectForKey:(id)key {
   let dict = env.objc.borrow::<NSCacheHostObject>(this).dict;
   if dict == nil || key == nil { return; }
   let _: () = msg![env; dict removeObjectForKey:key];
}

- (())removeAllObjects {
   let dict = env.objc.borrow::<NSCacheHostObject>(this).dict;
   if dict == nil { return; }
   let _: () = msg![env; dict removeAllObjects];
}

// MARK: Description

- (id)description {
   let (name, count) = {
       let h = env.objc.borrow::<NSCacheHostObject>(this);
       let name = h.name;
       let dict = h.dict;
       let count: u32 = if dict != nil { msg![env; dict count] } else { 0 };
       let name_str = if name != nil {
           ns_string::to_rust_string(env, name).into_owned()
       } else {
           String::new()
       };
       (name_str, count)
   };
   let s = format!("<NSCache: {:?}; name={:?}; count={}>", this, name, count);
   let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
   msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};

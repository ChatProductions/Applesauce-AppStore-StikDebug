/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSFileManager` etc.

use super::{ns_array, ns_string, NSUInteger};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::ns_error::{NSCocoaErrorDomain, NSFileReadNoSuchFileError};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::fs::{FsError, GuestPath, GuestPathBuf};
use crate::mem::{ConstPtr, MutPtr, Ptr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject, NSZonePtr,
};
use crate::Environment;

// Search path directories
type NSSearchPathDirectory = NSUInteger;
const NSApplicationDirectory: NSSearchPathDirectory = 1;
const NSDemoApplicationDirectory: NSSearchPathDirectory = 2;
const NSDeveloperApplicationDirectory: NSSearchPathDirectory = 3;
const NSAdminApplicationDirectory: NSSearchPathDirectory = 4;
const NSLibraryDirectory: NSSearchPathDirectory = 5;
const NSDeveloperDirectory: NSSearchPathDirectory = 6;
const NSUserDirectory: NSSearchPathDirectory = 7;
const NSDocumentationDirectory: NSSearchPathDirectory = 8;
const NSDocumentDirectory: NSSearchPathDirectory = 9;
const NSCoreServiceDirectory: NSSearchPathDirectory = 10;
const NSAutosavedInformationDirectory: NSSearchPathDirectory = 11;
const NSDesktopDirectory: NSSearchPathDirectory = 12;
const NSCachesDirectory: NSSearchPathDirectory = 13;
const NSApplicationSupportDirectory: NSSearchPathDirectory = 14;
const NSDownloadsDirectory: NSSearchPathDirectory = 15;
const NSInputMethodsDirectory: NSSearchPathDirectory = 16;
const NSMoviesDirectory: NSSearchPathDirectory = 17;
const NSMusicDirectory: NSSearchPathDirectory = 18;
const NSPicturesDirectory: NSSearchPathDirectory = 19;
const NSPrinterDescriptionDirectory: NSSearchPathDirectory = 20;
const NSSharedPublicDirectory: NSSearchPathDirectory = 21;
const NSPreferencePanesDirectory: NSSearchPathDirectory = 22;
const NSItemReplacementDirectory: NSSearchPathDirectory = 99;
const NSAllApplicationsDirectory: NSSearchPathDirectory = 100;
const NSAllLibrariesDirectory: NSSearchPathDirectory = 101;

// Search path domain masks
type NSSearchPathDomainMask = NSUInteger;
const NSUserDomainMask: NSSearchPathDomainMask = 1;
const NSLocalDomainMask: NSSearchPathDomainMask = 2;
const NSNetworkDomainMask: NSSearchPathDomainMask = 4;
const NSSystemDomainMask: NSSearchPathDomainMask = 8;
const NSAllDomainsMask: NSSearchPathDomainMask = 0x0ffff;

// File attribute keys
pub const NSFileModificationDate: &str = "NSFileModificationDate";
pub const NSFileCreationDate: &str = "NSFileCreationDate";
pub const NSFileSize: &str = "NSFileSize";
const NSFileSystemFreeSize: &str = "NSFileSystemFreeSize";
const NSFileSystemSize: &str = "NSFileSystemSize";
const NSFileSystemNodes: &str = "NSFileSystemNodes";
const NSFileSystemFreeNodes: &str = "NSFileSystemFreeNodes";
pub const NSFileType: &str = "NSFileType";
pub const NSFileOwnerAccountName: &str = "NSFileOwnerAccountName";
pub const NSFileGroupOwnerAccountName: &str = "NSFileGroupOwnerAccountName";
pub const NSFilePosixPermissions: &str = "NSFilePosixPermissions";
pub const NSFileReferenceCount: &str = "NSFileReferenceCount";
pub const NSFileDeviceIdentifier: &str = "NSFileDeviceIdentifier";
pub const NSFileSystemNumber: &str = "NSFileSystemNumber";
pub const NSFileExtensionHidden: &str = "NSFileExtensionHidden";
pub const NSFileHFSCreatorCode: &str = "NSFileHFSCreatorCode";
pub const NSFileHFSTypeCode: &str = "NSFileHFSTypeCode";
pub const NSFileImmutable: &str = "NSFileImmutable";
pub const NSFileAppendOnly: &str = "NSFileAppendOnly";
pub const NSFileBusy: &str = "NSFileBusy";

// File type values
pub const NSFileTypeDirectory: &str = "NSFileTypeDirectory";
pub const NSFileTypeRegular: &str = "NSFileTypeRegular";
pub const NSFileTypeSymbolicLink: &str = "NSFileTypeSymbolicLink";
pub const NSFileTypeSocket: &str = "NSFileTypeSocket";
pub const NSFileTypeCharacterSpecial: &str = "NSFileTypeCharacterSpecial";
pub const NSFileTypeBlockSpecial: &str = "NSFileTypeBlockSpecial";
pub const NSFileTypeUnknown: &str = "NSFileTypeUnknown";

// Volume attribute keys
pub const NSURLVolumeNameKey: &str = "NSURLVolumeNameKey";
pub const NSURLVolumeLocalizedNameKey: &str = "NSURLVolumeLocalizedNameKey";
pub const NSURLVolumeTotalCapacityKey: &str = "NSURLVolumeTotalCapacityKey";
pub const NSURLVolumeAvailableCapacityKey: &str = "NSURLVolumeAvailableCapacityKey";

// Item replacement options
type NSFileManagerItemReplacementOptions = NSUInteger;
const NSFileManagerItemReplacementUsingNewMetadataOnly: NSFileManagerItemReplacementOptions = 1;
const NSFileManagerItemReplacementWithoutDeletingBackupItem: NSFileManagerItemReplacementOptions = 2;

// Directory enumeration options
type NSDirectoryEnumerationOptions = NSUInteger;
const NSDirectoryEnumerationSkipsSubdirectoryDescendants: NSDirectoryEnumerationOptions = 1;
const NSDirectoryEnumerationSkipsPackageDescendants: NSDirectoryEnumerationOptions = 2;
const NSDirectoryEnumerationSkipsHiddenFiles: NSDirectoryEnumerationOptions = 4;

pub const CONSTANTS: ConstantExports = &[
    ("_NSFileModificationDate", HostConstant::NSString(NSFileModificationDate)),
    ("_NSFileCreationDate", HostConstant::NSString(NSFileCreationDate)),
    ("_NSFileSize", HostConstant::NSString(NSFileSize)),
    ("_NSFileSystemFreeSize", HostConstant::NSString(NSFileSystemFreeSize)),
    ("_NSFileSystemSize", HostConstant::NSString(NSFileSystemSize)),
    ("_NSFileType", HostConstant::NSString(NSFileType)),
    ("_NSFileTypeDirectory", HostConstant::NSString(NSFileTypeDirectory)),
    ("_NSFileTypeRegular", HostConstant::NSString(NSFileTypeRegular)),
    ("_NSFileTypeSymbolicLink", HostConstant::NSString(NSFileTypeSymbolicLink)),
    ("_NSFileTypeSocket", HostConstant::NSString(NSFileTypeSocket)),
    ("_NSFileTypeCharacterSpecial", HostConstant::NSString(NSFileTypeCharacterSpecial)),
    ("_NSFileTypeBlockSpecial", HostConstant::NSString(NSFileTypeBlockSpecial)),
    ("_NSFileTypeUnknown", HostConstant::NSString(NSFileTypeUnknown)),
    ("_NSFileOwnerAccountName", HostConstant::NSString(NSFileOwnerAccountName)),
    ("_NSFileGroupOwnerAccountName", HostConstant::NSString(NSFileGroupOwnerAccountName)),
    ("_NSFilePosixPermissions", HostConstant::NSString(NSFilePosixPermissions)),
    ("_NSFileReferenceCount", HostConstant::NSString(NSFileReferenceCount)),
    ("_NSFileDeviceIdentifier", HostConstant::NSString(NSFileDeviceIdentifier)),
    ("_NSFileExtensionHidden", HostConstant::NSString(NSFileExtensionHidden)),
    ("_NSFileImmutable", HostConstant::NSString(NSFileImmutable)),
    ("_NSFileAppendOnly", HostConstant::NSString(NSFileAppendOnly)),
    ("_NSFileBusy", HostConstant::NSString(NSFileBusy)),
];

fn NSSearchPathForDirectoriesInDomains(
    env: &mut Environment,
    directory: NSSearchPathDirectory,
    domain_mask: NSSearchPathDomainMask,
    expand_tilde: bool,
) -> id {
    // Only user domain supported for now
    if domain_mask != NSUserDomainMask && domain_mask != NSAllDomainsMask {
        log!("Warning: NSSearchPathForDirectoriesInDomains called with unsupported domain_mask: {}", domain_mask);
    }

    let _ = expand_tilde; // Always expand for simplicity

    let dir = match directory {
        NSApplicationDirectory => {
            GuestPath::new(crate::fs::APPLICATIONS).to_owned()
        }
        NSDocumentDirectory => env.fs.home_directory().join("Documents"),
        NSLibraryDirectory => env.fs.home_directory().join("Library"),
        NSCachesDirectory => env.fs.home_directory().join("Library/Caches"),
        NSApplicationSupportDirectory => env.fs.home_directory().join("Library/Application Support"),
        NSDesktopDirectory => env.fs.home_directory().join("Desktop"),
        NSDownloadsDirectory => env.fs.home_directory().join("Downloads"),
        NSMoviesDirectory => env.fs.home_directory().join("Movies"),
        NSMusicDirectory => env.fs.home_directory().join("Music"),
        NSPicturesDirectory => env.fs.home_directory().join("Pictures"),
        NSUserDirectory => env.fs.home_directory().to_owned(),
        NSPreferencePanesDirectory => env.fs.home_directory().join("Library/PreferencePanes"),
        NSAutosavedInformationDirectory => env.fs.home_directory().join("Library/Autosave Information"),
        _ => {
            log!("Warning: Unimplemented NSSearchPathDirectory {}, returning home directory", directory);
            env.fs.home_directory().to_owned()
        }
    };

    let dir = ns_string::from_rust_string(env, String::from(dir));
    let dir_list = ns_array::from_vec(env, vec![dir]);
    autorelease(env, dir_list)
}

fn NSHomeDirectory(env: &mut Environment) -> id {
    let dir = env.fs.home_directory();
    let dir = ns_string::from_rust_string(env, String::from(dir.as_str()));
    autorelease(env, dir)
}

fn NSUserName(_env: &mut Environment) -> id {
    // Return a default username
    let username = ns_string::from_rust_string(_env, String::from("touchHLE_user"));
    autorelease(_env, username)
}

fn NSFullUserName(_env: &mut Environment) -> id {
    // Return a default full user name
    let full_name = ns_string::from_rust_string(_env, String::from("touchHLE User"));
    autorelease(_env, full_name)
}

/// Check [crate::fs::Fs::new] for more info for
/// how temporary folder is setup on startup
fn NSTemporaryDirectory(env: &mut Environment) -> id {
    let dir = env.fs.home_directory().join("tmp");
    let dir = ns_string::from_rust_string(env, String::from(dir.as_str()));
    autorelease(env, dir)
}

fn NSOpenStepRootDirectory() -> id {
    // Return root directory
    nil // Not typically used on iOS
}

fn NSAllocateObject(
    env: &mut Environment,
    class: id,
    extra_bytes: NSUInteger,
    _zone: NSZonePtr,
) -> id {
    if extra_bytes > 0 {
        log!("Warning: NSAllocateObject called with extra_bytes={}, which is currently unhandled!", extra_bytes);
    }
    
    msg![env; class alloc]
}

fn NSDeallocateObject(env: &mut Environment, object: id) {
    if !object.is_null() {
        release(env, object);
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(NSHomeDirectory()),
    export_c_func!(NSTemporaryDirectory()),
    export_c_func!(NSSearchPathForDirectoriesInDomains(_, _, _)),
    export_c_func!(NSUserName()),
    export_c_func!(NSFullUserName()),
    export_c_func!(NSOpenStepRootDirectory()),
    export_c_func!(NSAllocateObject(_, _, _)),
    export_c_func!(NSDeallocateObject(_)),
];

#[derive(Default)]
pub struct State {
    default_manager: Option<id>,
}

struct NSDirectoryEnumeratorHostObject {
    iterator: std::vec::IntoIter<GuestPathBuf>,
    base_path: GuestPathBuf,
}
impl HostObject for NSDirectoryEnumeratorHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSFileManager: NSObject

+ (id)defaultManager {
    if let Some(existing) = env.framework_state.foundation.ns_file_manager.default_manager {
        existing
    } else {
        let new: id = msg![env; this new];
        env.framework_state.foundation.ns_file_manager.default_manager = Some(new);
        new
    }
}

// MARK: - Locating System Directories

- (id)currentDirectoryPath {
    ns_string::from_rust_string(env, env.fs.working_directory().as_str().to_string())
}

- (bool)changeCurrentDirectoryPath:(id)path {
    if path.is_null() {
        return false;
    }
    
    let path = ns_string::to_rust_string(env, path);
    let path = GuestPath::new(&path);
    match env.fs.change_working_directory(path) {
        Ok(_) => true,
        Err(()) => false
    }
}

// MARK: - Discovering Directory Contents

- (id)contentsOfDirectoryAtPath:(id)path
                          error:(MutPtr<id>)error {
    let contents: id = msg![env; this directoryContentsAtPath:path];
    if contents == nil && !error.is_null() {
        let domain = get_static_str(env, NSCocoaErrorDomain);
        let ns_error = msg_class![env; NSError alloc];
        let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
        env.mem.write(error, ns_error);
    }
    contents
}

- (id)directoryContentsAtPath:(id)path {
    if path.is_null() {
        return nil;
    }
    
    let path = ns_string::to_rust_string(env, path);
    let Ok(paths) = env.fs.enumerate(GuestPath::new(&path)) else {
        return nil;
    };
    let paths: Vec<GuestPathBuf> = paths
        .map(|path| GuestPathBuf::from(GuestPath::new(path)))
        .collect();

    log_dbg!("directoryContentsAtPath {}: {:?}", path, paths);
    let path_strings = paths
        .iter()
        .map(|name| ns_string::from_rust_string(env, name.as_str().to_string()))
        .collect();

    let res = ns_array::from_vec(env, path_strings);
    autorelease(env, res)
}

- (id)enumeratorAtPath:(id)path {
    if path.is_null() {
        return nil;
    }
    
    let path_str = ns_string::to_rust_string(env, path);
    let guest_path = GuestPath::new(&path_str);
    let Ok(paths) = env.fs.enumerate_recursive(guest_path) else {
        return nil;
    };
    
    let host_object = Box::new(NSDirectoryEnumeratorHostObject {
        iterator: paths.into_iter(),
        base_path: GuestPathBuf::from(guest_path),
    });

    let class = env.objc.get_known_class("NSDirectoryEnumerator", &mut env.mem);
    let enumerator = env.objc.alloc_object(class, host_object, &mut env.mem);
    autorelease(env, enumerator)
}

- (id)subpathsOfDirectoryAtPath:(id)path
                          error:(MutPtr<id>)error {
    if path.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return nil;
    }
    
    let path_str = ns_string::to_rust_string(env, path);
    let guest_path = GuestPath::new(&path_str);
    let Ok(paths) = env.fs.enumerate_recursive(guest_path) else {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return nil;
    };

    let path_strings: Vec<id> = paths
        .iter()
        .map(|p| ns_string::from_rust_string(env, p.as_str().to_string()))
        .collect();

    let res = ns_array::from_vec(env, path_strings);
    autorelease(env, res)
}

- (id)subpathsAtPath:(id)path {
    let error: MutPtr<id> = Ptr::null();
    msg![env; this subpathsOfDirectoryAtPath:path error:error]
}

// MARK: - Creating and Deleting Items

- (bool)createFileAtPath:(id)path
                contents:(id)data
              attributes:(id)attributes {
    if path.is_null() {
        return false;
    }
    
    let _ = attributes; // Ignore for now
    
    if data.is_null() {
        let empty: id = msg_class![env; NSData new];
        let res: bool = msg![env; empty writeToFile:path atomically:false];
        release(env, empty);
        res
    } else {
        msg![env; data writeToFile:path atomically:false]
    }
}

- (bool)createDirectoryAtPath:(id)path
                   attributes:(id)attributes {
    let error: MutPtr<id> = Ptr::null();
    msg![env; this createDirectoryAtPath:path
             withIntermediateDirectories:false
                              attributes:attributes
                                   error:error]
}

- (bool)createDirectoryAtPath:(id)path
  withIntermediateDirectories:(bool)with_intermediates
                   attributes:(id)_attributes
                        error:(MutPtr<id>)error {
    if path.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return false;
    }
    
    let path_str = ns_string::to_rust_string(env, path);

    let res = if with_intermediates {
        env.fs.create_dir_all(GuestPath::new(&path_str))
    } else {
        env.fs.create_dir(GuestPath::new(&path_str))
    };

    match res {
        Ok(()) => {
            log_dbg!("createDirectoryAtPath {} => true", path_str);
            true
        }
        Err(err) => {
            log!(
                "Warning: createDirectoryAtPath {} failed with {:?}, returning false",
                path_str,
                err,
            );
            if !error.is_null() {
                let domain = get_static_str(env, NSCocoaErrorDomain);
                let ns_error = msg_class![env; NSError alloc];
                let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
                env.mem.write(error, ns_error);
            }
            false
        }
    }
}

- (bool)createSymbolicLinkAtPath:(id)path
             withDestinationPath:(id)dest_path
                           error:(MutPtr<id>)error {
    // Symbolic links not fully supported - log and return false
    let _ = (path, dest_path);
    log!("Warning: createSymbolicLinkAtPath:withDestinationPath:error: not fully implemented");
    if !error.is_null() {
        let domain = get_static_str(env, NSCocoaErrorDomain);
        let ns_error = msg_class![env; NSError alloc];
        let ns_error = msg![env; ns_error initWithDomain:domain code:1 userInfo:nil];
        env.mem.write(error, ns_error);
    }
    false
}

- (bool)removeItemAtPath:(id)path
                   error:(MutPtr<id>)out_error {
    if path.is_null() {
        if !out_error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let error = msg_class![env; NSError alloc];
            let error = msg![env; error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(out_error, error);
        }
        return false;
    }
    
    let path_str = ns_string::to_rust_string(env, path);
    match env.fs.remove(GuestPath::new(&path_str)) {
        Ok(()) => true,
        Err(err) => {
            if !out_error.is_null() {
                match err {
                    FsError::DoesNotExist => {
                        let domain = get_static_str(env, NSCocoaErrorDomain);
                        let error = msg_class![env; NSError alloc];
                        let error = msg![env; error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
                        env.mem.write(out_error, error);
                    }
                    _ => {
                        let domain = get_static_str(env, NSCocoaErrorDomain);
                        let error = msg_class![env; NSError alloc];
                        let error = msg![env; error initWithDomain:domain code:1 userInfo:nil];
                        env.mem.write(out_error, error);
                    }
                }
            }
            false
        }
    }
}

// MARK: - Moving and Copying Items

- (bool)copyItemAtPath:(id)src
                toPath:(id)dst
                 error:(MutPtr<id>)error {
    if src.is_null() || dst.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return false;
    }
    
    let src_str = ns_string::to_rust_string(env, src);
    let dst_str = ns_string::to_rust_string(env, dst);
    let data = match env.fs.read(GuestPath::new(src_str.as_ref())) {
        Ok(d) => d,
        Err(_) => {
            if !error.is_null() {
                let domain = get_static_str(env, NSCocoaErrorDomain);
                let ns_error = msg_class![env; NSError alloc];
                let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
                env.mem.write(error, ns_error);
            }
            return false;
        }
    };

    if env.fs.write(GuestPath::new(dst_str.as_ref()), &data).is_err() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return false;
    }
    true
}

- (bool)moveItemAtPath:(id)path
                toPath:(id)toPath
                 error:(MutPtr<id>)error {
    if path.is_null() || toPath.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return false;
    }
    
    let path_str = ns_string::to_rust_string(env, path);
    let to_path_str = ns_string::to_rust_string(env, toPath);
    match env.fs.rename(GuestPath::new(&path_str), GuestPath::new(&to_path_str)) {
        Ok(()) => true,
        Err(()) => {
            if !error.is_null() {
               let domain = get_static_str(env, NSCocoaErrorDomain);
               let ns_error = msg_class![env; NSError alloc];
               let ns_error = msg![env; ns_error initWithDomain:domain code:1 userInfo:nil];
               env.mem.write(error, ns_error);
            }
            false
        }
    }
}

- (bool)linkItemAtPath:(id)src_path
                toPath:(id)dst_path
                 error:(MutPtr<id>)error {
    // Hard links not supported - just copy instead
    msg![env; this copyItemAtPath:src_path toPath:dst_path error:error]
}

// MARK: - Managing iCloud-Based Items (Stubs)

- (id)URLForUbiquityContainerIdentifier:(id)_container_id {
    // iCloud not supported
    nil
}

- (bool)isUbiquitousItemAtURL:(id)_url {
    // iCloud not supported
    false
}

- (bool)setUbiquitous:(bool)_flag
        itemAtURL:(id)_url
   destinationURL:(id)_dest_url
            error:(MutPtr<id>)error {
    // iCloud not supported
    if !error.is_null() {
        let domain = get_static_str(env, NSCocoaErrorDomain);
        let ns_error = msg_class![env; NSError alloc];
        let ns_error = msg![env; ns_error initWithDomain:domain code:1 userInfo:nil];
        env.mem.write(error, ns_error);
    }
    false
}

// MARK: - Determining Access to Files

- (bool)fileExistsAtPath:(id)path {
    if path.is_null() {
        return false;
    }
    
    let path_str = ns_string::to_rust_string(env, path);
    let res_exists = env.fs.exists(GuestPath::new(&path_str));
    log_dbg!("[(NSFileManager*) {:?} fileExistsAtPath:{:?}] => {}", this, path, res_exists);
    res_exists
}

- (bool)fileExistsAtPath:(id)path
             isDirectory:(MutPtr<bool>)is_dir {
    if path.is_null() {
        if !is_dir.is_null() {
            env.mem.write(is_dir, false);
        }
        return false;
    }
    
    let path_str = ns_string::to_rust_string(env, path);
    let guest_path = GuestPath::new(&path_str);
    let res_exists = env.fs.exists(guest_path);
    let res_is_dir = if res_exists {
        !env.fs.is_file(guest_path)
    } else {
        false
    };

    if !is_dir.is_null() {
        env.mem.write(is_dir, res_is_dir);
    }

    log_dbg!("[(NSFileManager*) {:?} fileExistsAtPath:{:?} isDirectory:{:?}] => {}", this, path, res_is_dir, res_exists);
    res_exists
}

- (bool)isReadableFileAtPath:(id)path {
    if path.is_null() {
        return false;
    }
    
    let path_str = ns_string::to_rust_string(env, path);
    let (_, readable, _, _) = env.fs.access(GuestPath::new(&path_str));
    readable
}

- (bool)isWritableFileAtPath:(id)path {
    if path.is_null() {
        return false;
    }
    
    let path_str = ns_string::to_rust_string(env, path);
    let (_, _, writable, _) = env.fs.access(GuestPath::new(&path_str));
    writable
}

- (bool)isExecutableFileAtPath:(id)path {
    if path.is_null() {
        return false;
    }
    
    let path_str = ns_string::to_rust_string(env, path);
    let (_, _, _, executable) = env.fs.access(GuestPath::new(&path_str));
    executable
}

- (bool)isDeletableFileAtPath:(id)path {
    if path.is_null() {
        return false;
    }
    
    let path_str = ns_string::to_rust_string(env, path);
    let guest_path = GuestPath::new(&path_str);
    let is_file = env.fs.is_file(guest_path);

    if is_file {
        return msg![env; this isWritableFileAtPath:path];
    }

    let directory_enumerator: id = msg![env; this enumeratorAtPath:path];

    let mut is_deletable = true;
    loop {
        let sub_path: id = msg![env; directory_enumerator nextObject];
        if sub_path == nil {
            break;
        }
        let is_path_deletable: bool = msg![env; this isDeletableFileAtPath:sub_path];
        is_deletable &= is_path_deletable;
        if !is_deletable {
            break;
        }
    }
    is_deletable
}

// MARK: - Getting and Setting Attributes

- (id)attributesOfItemAtPath:(id)path
                       error:(MutPtr<id>)_error {
    if path.is_null() {
        if !_error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(_error, ns_error);
        }
        return nil;
    }
    
    log_once!("Warning: NSFileManager attributesOfItemAtPath:error: returns limited attributes!");
    let path_str = ns_string::to_rust_string(env, path);
    log_dbg!("[(NSFileManager *){:?} attributesOfItemAtPath:{} error:{:?}]", this, path_str, _error);

    let guest_path = GuestPath::new(&path_str);
    file_attributes_common(env, guest_path)
}

- (id)fileAttributesAtPath:(id)path
              traverseLink:(bool)_traverse {
    if path.is_null() {
        return nil;
    }
    
    log_once!("Warning: NSFileManager fileAttributesAtPath:traverseLink: returns limited attributes!");
    let path_str = ns_string::to_rust_string(env, path);
    log_dbg!("[(NSFileManager *){:?} fileAttributesAtPath:{} traverse:{}]", this, path_str, _traverse);

    let guest_path = GuestPath::new(&path_str);
    file_attributes_common(env, guest_path)
}

- (id)attributesOfFileSystemForPath:(id)_path
                              error:(MutPtr<id>)_error {
    log_once!("Warning: NSFileManager attributesOfFileSystemForPath:error: returns only NSFileSystemFreeSize attribute!");
    let dict = msg_class![env; NSMutableDictionary new];

    // Reporting 1 GB of free space
    let size: u64 = 1024 * 1024 * 1024;
    let size_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:size];

    let fs_free_size_key = get_static_str(env, NSFileSystemFreeSize);
    () = msg![env; dict setObject:size_num forKey:fs_free_size_key];

    let dict_imm = msg![env; dict copy];
    release(env, dict);
    autorelease(env, dict_imm)
}

- (bool)setAttributes:(id)_attributes
         ofItemAtPath:(id)_path
                error:(MutPtr<id>)error {
    // Attribute setting not implemented
    log!("Warning: setAttributes:ofItemAtPath:error: not implemented");
    if !error.is_null() {
        let domain = get_static_str(env, NSCocoaErrorDomain);
        let ns_error = msg_class![env; NSError alloc];
        let ns_error = msg![env; ns_error initWithDomain:domain code:1 userInfo:nil];
        env.mem.write(error, ns_error);
    }
    false
}

// MARK: - Getting and Comparing File Contents

- (id)contentsAtPath:(id)path {
    if path.is_null() {
        return nil;
    }
    
    // TODO: return nil if path is directory
    // TODO: handle non-absolute paths?
    let is_absolute: bool = msg![env; path isAbsolutePath];
    if !is_absolute {
        log!("Warning: contentsAtPath called with non-absolute path");
    }
    
    msg_class![env; NSData dataWithContentsOfFile:path]
}

- (bool)contentsEqualAtPath:(id)path1
                    andPath:(id)path2 {
    if path1.is_null() || path2.is_null() {
        return false;
    }
    
    let data1: id = msg![env; this contentsAtPath:path1];
    let data2: id = msg![env; this contentsAtPath:path2];
    
    if data1.is_null() || data2.is_null() {
        return data1 == data2; // Both nil = equal
    }
    
    msg![env; data1 isEqual:data2]
}

// MARK: - Getting the Relationship Between Items

- (id)displayNameAtPath:(id)path {
    if path.is_null() {
        return nil;
    }
    msg![env; path lastPathComponent]
}

- (id)componentsToDisplayForPath:(id)path {
    if path.is_null() {
        return nil;
    }
    msg![env; path pathComponents]
}

// MARK: - Converting File Paths to Strings

- (ConstPtr<u8>)fileSystemRepresentationWithPath:(id)path {
    if path.is_null() {
        return ConstPtr::null();
    }
    
    let length: NSUInteger = msg![env; path length];
    if length == 0 {
        return ConstPtr::null();
    }
    
    msg![env; path UTF8String]
}

- (id)stringWithFileSystemRepresentation:(ConstPtr<u8>)str
                                  length:(NSUInteger)len {
    if str.is_null() {
        return nil;
    }
    
    let string: id = msg_class![env; NSString alloc];
    msg![env; string initWithBytes:str length:len encoding:4] // UTF8
}

// MARK: - Deprecated Methods (for compatibility)

- (bool)changeFileAttributes:(id)_attrs
                      atPath:(id)_path {
    log!("Warning: changeFileAttributes:atPath: is deprecated and not implemented");
    false
}

- (id)fileSystemAttributesAtPath:(id)path {
    let error: MutPtr<id> = Ptr::null();
    msg![env; this attributesOfFileSystemForPath:path error:error]
}

- (id)pathContentOfSymbolicLinkAtPath:(id)_path {
    // Symbolic links not supported
    nil
}

- (bool)createSymbolicLinkAtPath:(id)path
                     pathContent:(id)_content {
    log!("Warning: createSymbolicLinkAtPath:pathContent: not implemented");
    let _ = path;
    false
}

@end

@implementation NSDirectoryEnumerator: NSEnumerator

- (id)nextObject {
    let host_obj = env.objc.borrow_mut::<NSDirectoryEnumeratorHostObject>(this);
    host_obj.iterator.next().map_or(nil, |s| ns_string::from_rust_string(env, String::from(s)))
}

- (id)fileAttributes {
    // Return attributes of current file
    // Not fully implemented
    nil
}

- (id)directoryAttributes {
    // Return attributes of current directory
    // Not fully implemented
    nil
}

- (())skipDescendants {
    // Skip subdirectories
    log!("Warning: NSDirectoryEnumerator skipDescendants not implemented");
}

- (())skipDescendents {
    // Deprecated spelling
    msg![env; this skipDescendants]
}

- (NSUInteger)level {
    // Return nesting level
    // Not fully implemented
    0
}

@end

};

fn file_attributes_common(env: &mut Environment, guest_path: &GuestPath) -> id {
    if !env.fs.exists(guest_path) {
        log!(
            "file_attributes_common() called with file that does not exist: {:?}, Returning nil",
            guest_path
        );
        return nil;
    }

    let dict = msg_class![env; NSMutableDictionary new];

    // Modification date
    if let Ok(unix_timestamp) = env.fs.modified(guest_path) {
        let unix_timestamp_f64 = unix_timestamp as f64;
        let unix_ref_date: id = msg_class![env; NSDate dateWithTimeIntervalSince1970:0f64];
        let unix_date: id =
            msg_class![env; NSDate dateWithTimeInterval:unix_timestamp_f64 sinceDate:unix_ref_date];

        let modif_date_key = get_static_str(env, NSFileModificationDate);
        () = msg![env; dict setObject:unix_date forKey:modif_date_key];
        
        // Use same for creation date
        let creation_date_key = get_static_str(env, NSFileCreationDate);
        () = msg![env; dict setObject:unix_date forKey:creation_date_key];
    }

    // File size
    if let Ok(size) = env.fs.size(guest_path) {
        let size_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:size];
        let size_key = get_static_str(env, NSFileSize);
        () = msg![env; dict setObject:size_num forKey:size_key];
    }

    // File type
    let file_type_key = get_static_str(env, NSFileType);
    if env.fs.is_file(guest_path) {
        let file_type_regular = get_static_str(env, NSFileTypeRegular);
        () = msg![env; dict setObject:file_type_regular forKey:file_type_key];
    } else if env.fs.is_dir(guest_path) {
        let file_type_directory = get_static_str(env, NSFileTypeDirectory);
        () = msg![env; dict setObject:file_type_directory forKey:file_type_key];
    } else {
        let file_type_unknown = get_static_str(env, NSFileTypeUnknown);
        () = msg![env; dict setObject:file_type_unknown forKey:file_type_key];
    }

    // POSIX permissions (stub - default to 0644 for files, 0755 for dirs)
    let perms: u32 = if env.fs.is_dir(guest_path) { 0o755 } else { 0o644 };
    let perms_num: id = msg_class![env; NSNumber numberWithUnsignedInt:perms];
    let perms_key = get_static_str(env, NSFilePosixPermissions);
    () = msg![env; dict setObject:perms_num forKey:perms_key];

    let dict_imm = msg![env; dict copy];
    release(env, dict);
    autorelease(env, dict_imm)
}

// Helper functions for path manipulation
pub fn path_exists(env: &mut Environment, path: id) -> bool {
    if path.is_null() {
        return false;
    }
    let manager: id = msg_class![env; NSFileManager defaultManager];
    msg![env; manager fileExistsAtPath:path]
}

pub fn is_directory(env: &mut Environment, path: id) -> bool {
    if path.is_null() {
        return false;
    }
    let manager: id = msg_class![env; NSFileManager defaultManager];
    let mut is_dir: bool = false;
    let is_dir_ptr = &mut is_dir as *mut bool;
    let exists: bool = msg![env; manager fileExistsAtPath:path isDirectory:is_dir_ptr];
    exists && is_dir
}

pub fn create_directory_if_needed(env: &mut Environment, path: id) -> bool {
    if path.is_null() {
        return false;
    }
    
    if is_directory(env, path) {
        return true; // Already exists
    }
    
    let manager: id = msg_class![env; NSFileManager defaultManager];
    let error: MutPtr<id> = Ptr::null();
    msg![env; manager createDirectoryAtPath:path
               withIntermediateDirectories:true
                                attributes:nil
                                     error:error]
}

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGDataProvider.h`

use super::cg_image::{self, CGImageRef, CGImageRelease, CGImageRetain};
use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::core_foundation::cf_allocator::kCFAllocatorDefault;
use crate::frameworks::core_foundation::cf_data::{
    CFDataCreate, CFDataGetBytePtr, CFDataGetLength, CFDataRef,
};
use crate::frameworks::core_foundation::cf_url::CFURLRef;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::frameworks::foundation::NSUInteger;
use crate::fs::GuestPath;
use crate::mem::{ConstVoidPtr, GuestUSize, MutVoidPtr};
use crate::objc::{id, msg, msg_class, nil, objc_classes, ClassExports, HostObject};
use crate::Environment;

pub type CGDataProviderRef = CFTypeRef;

/// `(*void)(void *info, const void *data, size_t size)`
type CGDataProviderReleaseDataCallback = GuestFunction;

// A CGDataProvider is supposed to be a collection of callbacks used for
// accessing data, but at least for now, we instead only support some specific
// use-cases.

enum CGDataProviderHostObject {
    DataWithSize {
        data: ConstVoidPtr,
        size: GuestUSize,
        /// User-provided pointer passed to release callback.
        info: MutVoidPtr,
        release_callback: CGDataProviderReleaseDataCallback,
    },
    // TODO: Maybe we should store image data in guest memory so we don't
    // need a special variant for this.
    CGImage(CGImageRef),
    CFData(CFDataRef),
    Sequential {
        data: Vec<u8>,
        info: MutVoidPtr,
        release_info: GuestFunction,
    },
}
impl HostObject for CGDataProviderHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// CGDataProvider is a CFType-based type, but in our implementation those
// are just Objective-C types, so we need a class for it, but its name is not
// visible anywhere.
@implementation _touchHLE_CGDataProvider: NSObject

- (())dealloc {
    match *env.objc.borrow(this) {
        CGDataProviderHostObject::DataWithSize {
            info,
            data,
            size,
            release_callback,
        } => {
            if !release_callback.to_ptr().is_null() {
                let args: (MutVoidPtr, ConstVoidPtr, GuestUSize) = (info, data, size);
                log_dbg!(
                    "Freeing {:?}, calling release callback {:?} with {:?}",
                    this,
                    release_callback,
                    args,
                );
                () = release_callback.call_from_host(env, args);
            }
        },
        CGDataProviderHostObject::CGImage(cg_image) => CGImageRelease(env, cg_image),
        CGDataProviderHostObject::CFData(cf_data) => CFRelease(env, cf_data),

        CGDataProviderHostObject::Sequential { info, release_info, ref data } => {
            if !release_info.to_ptr().is_null() {
                // Сообщаем игре, что источник данных можно закрывать/освобождать
                let _: () = release_info.call_from_host(env, (info,));
            }
        },
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

pub fn CGDataProviderRelease(env: &mut Environment, c: CGDataProviderRef) {
    if !c.is_null() {
        CFRelease(env, c);
    }
}
pub fn CGDataProviderRetain(env: &mut Environment, c: CGDataProviderRef) -> CGDataProviderRef {
    if !c.is_null() {
        CFRetain(env, c)
    } else {
        c
    }
}

fn CGDataProviderCreateWithData(
    env: &mut Environment,
    info: MutVoidPtr,
    data: ConstVoidPtr,
    size: GuestUSize,
    release_callback: CGDataProviderReleaseDataCallback,
) -> CGDataProviderRef {
    let class = env
        .objc
        .get_known_class("_touchHLE_CGDataProvider", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGDataProviderHostObject::DataWithSize {
            info,
            data,
            size,
            release_callback,
        }),
        &mut env.mem,
    )
}

#[allow(rustdoc::broken_intra_doc_links)] // https://github.com/rust-lang/rust/issues/83049
/// This is for use by [super::cg_image::CGImageGetDataProvider].
pub(super) fn from_cg_image(env: &mut Environment, cg_image: CGImageRef) -> CGDataProviderRef {
    CGImageRetain(env, cg_image);
    let class = env
        .objc
        .get_known_class("_touchHLE_CGDataProvider", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGDataProviderHostObject::CGImage(cg_image)),
        &mut env.mem,
    )
}

/// Generic interface for host code.
pub(super) fn borrow_bytes(env: &mut Environment, provider: CGDataProviderRef) -> &[u8] {
    match *env.objc.borrow(provider) {
        CGDataProviderHostObject::DataWithSize { data, size, .. } => {
            env.mem.bytes_at(data.cast(), size)
        }
        CGDataProviderHostObject::CGImage(cg_image) => {
            cg_image::borrow_image(&env.objc, cg_image).pixels()
        }
        CGDataProviderHostObject::CFData(cf_data) => {
            let data = CFDataGetBytePtr(env, cf_data);
            let size = CFDataGetLength(env, cf_data);
            env.mem.bytes_at(data, size.try_into().unwrap())
        }
        CGDataProviderHostObject::Sequential { ref data, .. } => data.as_slice(),
    }
}

fn CGDataProviderCopyData(env: &mut Environment, provider: CGDataProviderRef) -> CFDataRef {
    match *env.objc.borrow(provider) {
        CGDataProviderHostObject::DataWithSize { data, size, .. } => CFDataCreate(
            env,
            kCFAllocatorDefault,
            data.cast(),
            size.try_into().unwrap(),
        ),
        CGDataProviderHostObject::CGImage(cg_image) => {
            let bytes = cg_image::borrow_image(&env.objc, cg_image).pixels();

            let len: NSUInteger = bytes.len().try_into().unwrap();
            let alloc = env.mem.alloc(len);
            env.mem
                .bytes_at_mut(alloc.cast(), len)
                .copy_from_slice(bytes);

            // TODO: it would be cleaner to use CFDataCreateWithBytesNoCopy, but
            // that's a bit more tricky.
            let ns_data: id = msg_class![env; NSData alloc];
            msg![env; ns_data initWithBytesNoCopy:alloc length:len]
        }
        CGDataProviderHostObject::CFData(cf_data) => {
            let data = CFDataGetBytePtr(env, cf_data);
            let size = CFDataGetLength(env, cf_data);
            CFDataCreate(env, kCFAllocatorDefault, data.cast(), size)
        }
        CGDataProviderHostObject::Sequential { ref data, .. } => {
            let len: NSUInteger = data.len().try_into().unwrap();
            let alloc = env.mem.alloc(len);
            env.mem.bytes_at_mut(alloc.cast(), len).copy_from_slice(data);
            
            let ns_data: id = msg_class![env; NSData alloc];
            msg![env; ns_data initWithBytesNoCopy:alloc length:len]
        }
    }
}

fn CGDataProviderCreateWithURL(env: &mut Environment, url: CFURLRef) -> CGDataProviderRef {
    assert!(msg![env; url isFileURL]); // TODO
    let path: id = msg![env; url path];
    log_dbg!(
        "CGDataProviderCreateWithURL url path {}",
        to_rust_string(env, path)
    );
    let data: id = msg_class![env; NSData dataWithContentsOfFile:path];
    CGDataProviderCreateWithCFData(env, data)
}

fn CGDataProviderCreateWithCFData(env: &mut Environment, data: CFDataRef) -> CGDataProviderRef {
    CFRetain(env, data);
    let class = env
        .objc
        .get_known_class("_touchHLE_CGDataProvider", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGDataProviderHostObject::CFData(data)),
        &mut env.mem,
    )
}

fn CGDataProviderCreateWithFilename(
    env: &mut Environment,
    filename: crate::mem::ConstPtr<u8>,
) -> CGDataProviderRef {
    let path_str = env.mem.cstr_at_utf8(filename).unwrap_or("").to_string();
    log_dbg!("CGDataProviderCreateWithFilename: {}", path_str);
    let Ok(bytes) = env.fs.read(GuestPath::new(&path_str)) else {
        log!("Warning: CGDataProviderCreateWithFilename: couldn't read {:?}", path_str);
        return nil; // <- was std::ptr::null()
    };
    let len: GuestUSize = bytes.len().try_into().unwrap();
    let buf = env.mem.alloc(len);
    env.mem.bytes_at_mut(buf.cast(), len).copy_from_slice(&bytes);

    CGDataProviderCreateWithData(
        env,
        MutVoidPtr::null(),
        buf.cast_const().cast(),
        len,
        GuestFunction::null_ptr(), // <- was GuestFunction::from_ptr(...)
    )
}

fn CGDataProviderGetInfo(
    _env: &mut Environment,
    _provider: CGDataProviderRef,
) -> MutVoidPtr {
    // Real API returns the `info` pointer passed at creation time.
    // We don't expose it publicly; return null as a safe stub.
    MutVoidPtr::null()
}

fn CGDataProviderGetSize(
    env: &mut Environment,
    provider: CGDataProviderRef,
) -> u64 {
    match *env.objc.borrow(provider) {
        CGDataProviderHostObject::DataWithSize { size, .. } => size as u64,
        CGDataProviderHostObject::CGImage(cg_image) => {
            cg_image::borrow_image(&env.objc, cg_image).pixels().len() as u64
        }
        CGDataProviderHostObject::CFData(cf_data) => {
            CFDataGetLength(env, cf_data) as u64
        }
        CGDataProviderHostObject::Sequential { ref data, .. } => data.len() as u64,
    }
}

fn CGDataProviderCreateSequential(
    env: &mut Environment,
    info: MutVoidPtr,
    callbacks: ConstVoidPtr,
) -> CGDataProviderRef {
    if callbacks.is_null() {
        return nil;
    }

    // В iOS структура CGDataProviderSequentialCallbacks выглядит так:
    // uint32 version; (должна быть 0)
    // void* getBytes;
    // void* skipForward;
    // void* rewind;
    // void* releaseInfo;
    
    let version: u32 = env.mem.read(callbacks.cast::<u32>());
    if version != 0 {
        log!("Warning: CGDataProviderCreateSequential unsupported version {}", version);
        return nil;
    }

    // Читаем указатели на функции из гостевой памяти
    let ptr_get_bytes: MutVoidPtr = env.mem.read(callbacks.cast::<MutVoidPtr>() + 1);
    let ptr_release_info: MutVoidPtr = env.mem.read(callbacks.cast::<MutVoidPtr>() + 4);

    let get_bytes = GuestFunction::from_ptr(ptr_get_bytes);
    let release_info = GuestFunction::from_ptr(ptr_release_info);

    // Вытягиваем данные напрямую в вектор хоста
    let mut data = Vec::new();
    if !get_bytes.to_ptr().is_null() {
        let chunk_size: u32 = 4096; // Читаем по 4 КБ
        let buf = env.mem.alloc(chunk_size);
        loop {
            // Вызываем гостевую функцию getBytes(info, buffer, count)
            let bytes_read: u32 = get_bytes.call_from_host(env, (info, buf, chunk_size));
            if bytes_read == 0 {
                break; // Конец потока
            }
            data.extend_from_slice(env.mem.bytes_at(buf.cast(), bytes_read));
        }
        env.mem.free(buf);
    }

    // Упаковываем результат в наш новый вариант
    let class = env
        .objc
        .get_known_class("_touchHLE_CGDataProvider", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGDataProviderHostObject::Sequential {
            data,
            info,
            release_info,
        }),
        &mut env.mem,
    )
}

fn CGDataProviderCreateDirect(
    _env: &mut Environment,
    _info: MutVoidPtr,
    _size: i64,
    _callbacks: ConstVoidPtr,
) -> CGDataProviderRef {
    log!("Warning: CGDataProviderCreateDirect is not supported, returning null");
    nil // <- was std::ptr::null()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGDataProviderRetain(_)),
    export_c_func!(CGDataProviderRelease(_)),
    export_c_func!(CGDataProviderCreateWithData(_, _, _, _)),
    export_c_func!(CGDataProviderCopyData(_)),
    export_c_func!(CGDataProviderCreateWithURL(_)),
    export_c_func!(CGDataProviderCreateWithCFData(_)),
    export_c_func!(CGDataProviderCreateWithFilename(_)),
    export_c_func!(CGDataProviderGetInfo(_)),
    export_c_func!(CGDataProviderGetSize(_)),
    export_c_func!(CGDataProviderCreateSequential(_, _)),
    export_c_func!(CGDataProviderCreateDirect(_, _, _)),
];

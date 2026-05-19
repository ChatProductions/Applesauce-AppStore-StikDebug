/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `Mach-O` related functions.

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, MutPtr, Ptr, SafeRead};
use crate::Environment;

// --- ДОБАВЛЯЕМ СТРУКТУРЫ И КОНСТАНТЫ ДЛЯ host_info ---
const HOST_BASIC_INFO: i32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct host_basic_info {
    max_cpus: i32,
    avail_cpus: i32,
    memory_size: u32,
    cpu_type: i32,
    cpu_subtype: i32,
    cpu_threadtype: i32,
    physical_cpu: i32,
    physical_cpu_max: i32,
    logical_cpu: i32,
    logical_cpu_max: i32,
    max_mem: u64,
}
unsafe impl SafeRead for host_basic_info {}

fn host_info(
    env: &mut Environment,
    _host: i32,
    flavor: i32,
    info_out: MutPtr<i32>,
    info_cnt: MutPtr<u32>,
) -> i32 {
    match flavor {
        HOST_BASIC_INFO => {
            let mut info = host_basic_info::default();

            // Эмулируем железо iPad/iPhone
            info.max_cpus = 1;
            info.avail_cpus = 1;
            info.physical_cpu = 1;
            info.logical_cpu = 1;

            // CPU_TYPE_ARM = 12, CPU_SUBTYPE_ARM_V7 = 9
            info.cpu_type = 12;
            info.cpu_subtype = 9;

            // 256MB RAM (безопасное значение для старых игр)
            let mem_bytes = 256 * 1024 * 1024;
            info.memory_size = mem_bytes as u32;
            info.max_mem = mem_bytes as u64;

            let struct_size = (std::mem::size_of::<host_basic_info>() / 4) as u32;
            let provided_cnt = env.mem.read(info_cnt);

            if provided_cnt < struct_size {
                return 4; // KERN_INVALID_ARGUMENT
            }

            env.mem.write(info_out.cast(), info);
            env.mem.write(info_cnt, struct_size);

            0 // KERN_SUCCESS
        }
        _ => {
            // Если запрашивают другой flavor, просто рапортуем об успехе без
            // паники
            0
        }
    }
}
// -----------------------------------------------------

fn get_end(env: &mut Environment) -> u32 {
    // Assume app binary is the first.
    // From https://www.manpagez.com/man/3/get_end/
    // `In a Mach-O file <...> get_end returns the first address after
    // the last segment in the executable`
    // It was confirmed on a real device with the TestApp binary.
    env.bins[0].last_segment_end
}

fn get_etext(env: &mut Environment) -> u32 {
    // Assume app binary is the first.
    let app_sections = &env.bins[0].sections;
    assert_eq!(
        app_sections
            .iter()
            .filter(|s| s.name.to_uppercase() == "__TEXT")
            .count(),
        1
    );
    let text_section = app_sections
        .iter()
        .find(|s| s.name.to_uppercase() == "__TEXT")
        .unwrap();
    text_section.next_section_addr()
}

// --- dyld API functions ---

/// `uint32_t _dyld_image_count(void)` — returns the number of images
/// (Mach-O binaries) currently loaded in the process address space.
/// See: https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/dyld.3.html
fn _dyld_image_count(env: &mut Environment) -> u32 {
    env.bins.len() as u32
}

/// `const struct mach_header* _dyld_get_image_header(uint32_t image_index)`
/// Returns the mach_header pointer for the image at `image_index`.
/// The mach_header is located at the start of the __TEXT segment.
fn _dyld_get_image_header(env: &mut Environment, image_index: u32) -> u32 {
    let idx = image_index as usize;
    if idx >= env.bins.len() {
        log!(
            "Warning: _dyld_get_image_header({}) out of range (only {} images loaded), returning 0",
            image_index,
            env.bins.len()
        );
        return 0;
    }
    env.bins[idx].text_base
}

/// `const char* _dyld_get_image_name(uint32_t image_index)`
/// Returns a C-string pointer with the path of the image at `image_index`.
/// We allocate the string in guest memory the first time it's requested.
fn _dyld_get_image_name(env: &mut Environment, image_index: u32) -> ConstPtr<u8> {
    let idx = image_index as usize;
    if idx >= env.bins.len() {
        log!(
            "Warning: _dyld_get_image_name({}) out of range, returning NULL",
            image_index
        );
        return Ptr::null();
    }
    let name = env.bins[idx].name.clone();
    let len = name.len() as u32 + 1;
    let ptr: MutPtr<u8> = env.mem.alloc(len).cast();
    let dst = env.mem.bytes_at_mut(ptr, len);
    dst[..name.len()].copy_from_slice(name.as_bytes());
    dst[name.len()] = 0;
    ptr.cast_const()
}

/// `intptr_t _dyld_get_image_vmaddr_slide(uint32_t image_index)`
/// Returns the virtual memory address slide for the image. Since touchHLE
/// loads binaries at their preferred addresses (slide=0 for the main app),
/// we return 0. Dylibs might have a slide but in practice iOS games don't
/// query this for anything critical.
fn _dyld_get_image_vmaddr_slide(env: &mut Environment, image_index: u32) -> u32 {
    let idx = image_index as usize;
    if idx >= env.bins.len() {
        return 0;
    }
    // For the main binary (index 0) the slide is always 0 in touchHLE.
    // For dylibs we don't track the slide separately, return 0.
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(get_end()),
    export_c_func!(get_etext()),
    export_c_func!(host_info(_, _, _, _)),
    export_c_func!(_dyld_image_count()),
    export_c_func!(_dyld_get_image_header(_)),
    export_c_func!(_dyld_get_image_name(_)),
    export_c_func!(_dyld_get_image_vmaddr_slide(_)),
];

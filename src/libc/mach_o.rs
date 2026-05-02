/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `Mach-O` related functions.

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutPtr, SafeRead};
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
            // Если запрашивают другой flavor, просто рапортуем об успехе без паники
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

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(get_end()),
    export_c_func!(get_etext()),
    export_c_func!(host_info(_, _, _, _)),
];

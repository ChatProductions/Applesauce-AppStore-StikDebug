/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioServices.h` (Audio Services)

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::fourcc;
use crate::mem::{MutPtr, MutVoidPtr};
use crate::objc::id;
use crate::Environment;

/// Usually a FourCC.
type AudioServicesPropertyID = u32;
type SystemSoundID = u32;

const kAudioServicesUnsupportedPropertyError: OSStatus = fourcc(b"pty?") as _;
const kSystemSoundID_Vibrate: SystemSoundID = 0x00000FFF;

fn AudioServicesGetProperty(
    _env: &mut Environment,
    in_property_id: AudioServicesPropertyID,
    _in_specifier_size: u32,
    _in_specifier: crate::mem::ConstVoidPtr,
    _io_property_data_size: MutPtr<u32>,
    _out_property_data: MutVoidPtr,
) -> OSStatus {
    if in_property_id == 0xfff {
        kAudioServicesUnsupportedPropertyError
    } else {
        log!("AudioServicesGetProperty: property {} is unimplemented", in_property_id);
        0 // Возвращаем Success, чтобы не падать
    }
}

fn AudioServicesCreateSystemSoundID(
    env: &mut Environment,
    _in_file_url: id,
    out_system_sound_id: MutPtr<SystemSoundID>,
) -> OSStatus {
    log!("AudioToolbox: AudioServicesCreateSystemSoundID stubbed");
    if !out_system_sound_id.is_null() {
        // Записываем фейковый ID (например, 1001)
        env.mem.write(out_system_sound_id, 1001);
    }
    0 // noErr
}

fn AudioServicesDisposeSystemSoundID(
    _env: &mut Environment,
    _in_system_sound_id: SystemSoundID,
) -> OSStatus {
    0 // noErr
}

fn AudioServicesPlaySystemSound(_env: &mut Environment, in_system_sound_id: SystemSoundID) {
    if in_system_sound_id == kSystemSoundID_Vibrate {
        log!("TODO: vibration (AudioServicesPlaySystemSound)");
    } else {
        log!("AudioToolbox: Playing system sound ID: {}", in_system_sound_id);
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioServicesGetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioServicesCreateSystemSoundID(_, _, _)),
    export_c_func!(AudioServicesDisposeSystemSoundID(_, _)),
    export_c_func!(AudioServicesPlaySystemSound(_, _)),
];

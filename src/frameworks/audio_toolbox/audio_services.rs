/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(dead_code)]
//! `AudioServices.h` (Audio Services)

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::fourcc;
use crate::mem::{ConstVoidPtr, MutPtr, MutVoidPtr};
use crate::objc::id;
use crate::Environment;
use std::collections::HashMap;

type AudioServicesPropertyID = u32;
type SystemSoundID = u32;
type AudioServicesSystemSoundCompletionProc = u32;

const kAudioServicesUnsupportedPropertyError: OSStatus = fourcc(b"pty?") as _;
const kAudioServicesBadSystemSoundIDError: OSStatus = fourcc(b"!ids") as _;
const kSystemSoundID_Vibrate: SystemSoundID = 0x00000FFF;
const kSystemSoundID_UserPreferredAlert: SystemSoundID = 0x00001000;

// ДОБАВЛЕНО: Состояние для хранения звуков
#[derive(Default)]
pub struct State {
    next_sound_id: u32,
    sounds: HashMap<SystemSoundID, u32>, // В будущем u32 заменится на реальный ALuint буфер OpenAL
}

fn AudioServicesGetProperty(
    _env: &mut Environment,
    in_property_id: AudioServicesPropertyID,
    _in_specifier_size: u32,
    _in_specifier: ConstVoidPtr,
    _io_property_data_size: MutPtr<u32>,
    _out_property_data: MutVoidPtr,
) -> OSStatus {
    if in_property_id == 0xfff {
        kAudioServicesUnsupportedPropertyError
    } else {
        log!("AudioServicesGetProperty: property {} is unimplemented", in_property_id);
        0
    }
}

fn AudioServicesSetProperty(
    _env: &mut Environment,
    in_property_id: AudioServicesPropertyID,
    _in_specifier_size: u32,
    _in_specifier: ConstVoidPtr,
    _in_property_data_size: u32,
    _in_property_data: ConstVoidPtr,
) -> OSStatus {
    log!("AudioServicesSetProperty: property {} is unimplemented", in_property_id);
    0
}

fn AudioServicesCreateSystemSoundID(
    env: &mut Environment,
    _in_file_url: id,
    out_system_sound_id: MutPtr<SystemSoundID>,
) -> OSStatus {
    let state = &mut env.framework_state.audio_toolbox.audio_services;
    
    // Генерируем новый уникальный ID для каждого звука
    let new_id = state.next_sound_id + 1000;
    state.next_sound_id += 1;
    
    // Регистрируем его
    state.sounds.insert(new_id, 0);

    if !out_system_sound_id.is_null() {
        env.mem.write(out_system_sound_id, new_id);
    }
    
    log!("AudioToolbox: AudioServicesCreateSystemSoundID created ID {}", new_id);
    0
}

fn AudioServicesDisposeSystemSoundID(
    env: &mut Environment,
    in_system_sound_id: SystemSoundID,
) -> OSStatus {
    let state = &mut env.framework_state.audio_toolbox.audio_services;
    state.sounds.remove(&in_system_sound_id);
    0
}

fn AudioServicesPlaySystemSound(_env: &mut Environment, in_system_sound_id: SystemSoundID) {
    if in_system_sound_id == kSystemSoundID_Vibrate {
        log!("TODO: vibration (AudioServicesPlaySystemSound)");
    } else if in_system_sound_id == kSystemSoundID_UserPreferredAlert {
        log!("TODO: alert sound (AudioServicesPlaySystemSound)");
    } else {
        log!("AudioToolbox: Playing system sound ID: {}", in_system_sound_id);
        // В будущем здесь будет вызов OpenAL: alSourcePlay(buffer)
    }
}

fn AudioServicesPlayAlertSound(env: &mut Environment, in_system_sound_id: SystemSoundID) {
    AudioServicesPlaySystemSound(env, in_system_sound_id);
}

fn AudioServicesAddSystemSoundCompletion(
    _env: &mut Environment,
    _in_system_sound_id: SystemSoundID,
    _in_run_loop: MutVoidPtr,
    _in_run_loop_mode: MutVoidPtr,
    _in_completion_routine: AudioServicesSystemSoundCompletionProc,
    _in_client_data: MutVoidPtr,
) -> OSStatus {
    0
}

fn AudioServicesRemoveSystemSoundCompletion(
    _env: &mut Environment,
    _in_system_sound_id: SystemSoundID,
) {
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioServicesGetProperty(_, _, _, _, _)),
    export_c_func!(AudioServicesSetProperty(_, _, _, _, _)),
    export_c_func!(AudioServicesCreateSystemSoundID(_, _)),
    export_c_func!(AudioServicesDisposeSystemSoundID(_)),
    export_c_func!(AudioServicesPlaySystemSound(_)),
    export_c_func!(AudioServicesPlayAlertSound(_)),
    export_c_func!(AudioServicesAddSystemSoundCompletion(_, _, _, _, _)),
    export_c_func!(AudioServicesRemoveSystemSoundCompletion(_)),
];


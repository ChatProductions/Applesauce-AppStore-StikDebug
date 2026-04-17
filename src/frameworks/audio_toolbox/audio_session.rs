/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::{debug_fourcc, fourcc};
use crate::frameworks::core_foundation::cf_run_loop::{CFRunLoopMode, CFRunLoopRef};
use crate::mem::{guest_size_of, ConstVoidPtr, MutPtr, MutVoidPtr};
use crate::Environment;

type AudioSessionInterruptionListener = GuestFunction;
type AudioSessionPropertyListener = GuestFunction;

const kAudioSessionBadPropertySizeError: OSStatus = fourcc(b"!siz") as _;
const kAudioSessionNoErr: OSStatus = 0;

type AudioSessionPropertyID = u32;
const kAudioSessionProperty_OtherAudioIsPlaying: AudioSessionPropertyID = fourcc(b"othr");
const kAudioSessionProperty_AudioCategory: AudioSessionPropertyID = fourcc(b"acat");
const kAudioSessionProperty_CurrentHardwareSampleRate: AudioSessionPropertyID = fourcc(b"chsr");
const kAudioSessionProperty_CurrentHardwareOutputNumberChannels: AudioSessionPropertyID = fourcc(b"choc");
const kAudioSessionProperty_CurrentHardwareOutputVolume: AudioSessionPropertyID = fourcc(b"chov");
const kAudioSessionProperty_PreferredHardwareIOBufferDuration: AudioSessionPropertyID = fourcc(b"pbuf");
const kAudioSessionProperty_CurrentHardwareIOBufferDuration: AudioSessionPropertyID = fourcc(b"cbuf");
const kAudioSessionProperty_AudioInputAvailable: AudioSessionPropertyID = fourcc(b"aiav");
const kAudioSessionProperty_AudioRoute: AudioSessionPropertyID = fourcc(b"rout");

// Корректная структура состояния, которая нужна для audio_unit.rs
pub struct State {
    pub active: bool,
    pub category: u32,
    pub current_hardware_sample_rate: f64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active: false,
            category: fourcc(b"ambi"),
            current_hardware_sample_rate: 44100.0,
        }
    }
}

#[allow(dead_code)]
fn get_audio_session_property_size(in_id: AudioSessionPropertyID) -> u32 {
    match in_id {
        kAudioSessionProperty_OtherAudioIsPlaying => guest_size_of::<u32>(),
        kAudioSessionProperty_AudioCategory => guest_size_of::<u32>(),
        kAudioSessionProperty_CurrentHardwareSampleRate => guest_size_of::<f64>(),
        kAudioSessionProperty_CurrentHardwareOutputNumberChannels => guest_size_of::<u32>(),
        kAudioSessionProperty_CurrentHardwareOutputVolume => guest_size_of::<f32>(),
        kAudioSessionProperty_CurrentHardwareIOBufferDuration => guest_size_of::<f32>(),
        kAudioSessionProperty_PreferredHardwareIOBufferDuration => guest_size_of::<f32>(),
        kAudioSessionProperty_AudioInputAvailable => guest_size_of::<u32>(),
        kAudioSessionProperty_AudioRoute => guest_size_of::<u32>(),
        _ => {
            log!(
                "TODO: get_audio_session_property_size unknown property: {} -> 4",
                debug_fourcc(in_id)
            );
            guest_size_of::<u32>()
        }
    }
}

pub fn AudioSessionInitialize(
    _env: &mut Environment,
    _in_run_loop: CFRunLoopRef,
    _in_run_loop_mode: CFRunLoopMode,
    _in_interruption_listener: AudioSessionInterruptionListener,
    _in_client_data: ConstVoidPtr,
) -> OSStatus {
    log_dbg!("AudioSessionInitialize");
    kAudioSessionNoErr
}

pub fn AudioSessionSetActive(env: &mut Environment, active: u32) -> OSStatus {
    log_dbg!("AudioSessionSetActive({})", active != 0);
    // Теперь обращаемся по правильному пути: env.framework_state...
    env.framework_state.audio_toolbox.audio_session.active = active != 0;
    kAudioSessionNoErr
}

pub fn AudioSessionGetPropertySize(
    _env: &mut Environment,
    in_id: AudioSessionPropertyID,
    _out_data_size: MutPtr<u32>,
) -> OSStatus {
    log_dbg!("TODO: AudioSessionGetPropertySize for {}", debug_fourcc(in_id));
    kAudioSessionNoErr
}

pub fn AudioSessionGetProperty(
    _env: &mut Environment,
    in_id: AudioSessionPropertyID,
    _io_data_size: MutPtr<u32>,
    _out_data: MutVoidPtr,
) -> OSStatus {
    log!("TODO: AudioSessionGetProperty {}", debug_fourcc(in_id));
    kAudioSessionNoErr
}

pub fn AudioSessionSetProperty(
    _env: &mut Environment,
    in_id: AudioSessionPropertyID,
    _in_data_size: u32,
    _in_data: ConstVoidPtr,
) -> OSStatus {
    log!("TODO: AudioSessionSetProperty {}", debug_fourcc(in_id));
    kAudioSessionNoErr
}

pub fn AudioSessionAddPropertyListener(
    _env: &mut Environment,
    in_id: AudioSessionPropertyID,
    _in_proc: AudioSessionPropertyListener,
    _in_client_data: ConstVoidPtr,
) -> OSStatus {
    log_dbg!("TODO: AudioSessionAddPropertyListener {}", debug_fourcc(in_id));
    kAudioSessionNoErr
}

pub fn AudioSessionRemovePropertyListener(
    _env: &mut Environment,
    in_id: AudioSessionPropertyID,
) -> OSStatus {
    log_dbg!("TODO: AudioSessionRemovePropertyListener {}", debug_fourcc(in_id));
    kAudioSessionNoErr
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioSessionInitialize(_, _, _, _)),
    export_c_func!(AudioSessionGetProperty(_, _, _)),
    export_c_func!(AudioSessionGetPropertySize(_, _)),
    export_c_func!(AudioSessionSetProperty(_, _, _)),
    export_c_func!(AudioSessionSetActive(_)),
    export_c_func!(AudioSessionAddPropertyListener(_, _, _)),
    export_c_func!(AudioSessionRemovePropertyListener(_)),
];

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `AudioSession.h` (Audio Session)

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::{debug_fourcc, fourcc};
use crate::frameworks::core_foundation::cf_run_loop::{CFRunLoopMode, CFRunLoopRef};
use crate::mem::{guest_size_of, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr};
use crate::Environment;

type AudioSessionInterruptionListener = GuestFunction;
type AudioSessionPropertyListener = GuestFunction;

const kAudioSessionBadPropertySizeError: OSStatus = fourcc(b"!siz") as _;

type AudioSessionPropertyID = u32;
const kAudioSessionProperty_OtherAudioIsPlaying: AudioSessionPropertyID = fourcc(b"othr");
const kAudioSessionProperty_AudioCategory: AudioSessionPropertyID = fourcc(b"acat");
const kAudioSessionProperty_CurrentHardwareSampleRate: AudioSessionPropertyID = fourcc(b"chsr");
const kAudioSessionProperty_CurrentHardwareOutputNumberChannels: AudioSessionPropertyID = fourcc(b"choc");
const kAudioSessionProperty_CurrentHardwareOutputVolume: AudioSessionPropertyID = fourcc(b"chov");
const kAudioSessionProperty_PreferredHardwareIOBufferDuration: AudioSessionPropertyID = fourcc(b"iobd");
const kAudioSessionProperty_PreferredHardwareSampleRate: AudioSessionPropertyID = fourcc(b"hwsr");
const kAudioSessionProperty_AudioInputAvailable: AudioSessionPropertyID = fourcc(b"aiav");
const kAudioSessionProperty_AudioRoute: AudioSessionPropertyID = fourcc(b"rout");

const kAudioSessionCategory_SoloAmbientSound: u32 = fourcc(b"solo");
const kAudioSessionProperty_CurrentHardwareIOBufferDuration: u32 = fourcc(b"chbd");

pub struct State {
    audio_session_category: u32,
    pub current_hardware_sample_rate: f64,
    pub current_hardware_output_number_channels: u32,
    current_hardware_output_volume: f32,
    current_hardware_io_buffer_duration: f32,
    pub interruption_listener: AudioSessionInterruptionListener,
    pub client_data: MutVoidPtr,
    pub is_initialized: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            audio_session_category: kAudioSessionCategory_SoloAmbientSound,
            current_hardware_sample_rate: 44100.0,
            current_hardware_output_number_channels: 2,
            current_hardware_output_volume: 1.0,
            current_hardware_io_buffer_duration: 0.023220,
            interruption_listener: GuestFunction::null(),
            client_data: MutVoidPtr::null(),
            is_initialized: false,
        }
    }
}

fn AudioSessionInitialize(
    env: &mut Environment,
    in_run_loop: CFRunLoopRef,
    in_run_loop_mode: CFRunLoopMode,
    in_interruption_listener: AudioSessionInterruptionListener,
    in_client_data: MutVoidPtr,
) -> OSStatus {
    let state = &mut env.framework_state.audio_toolbox.audio_session;
    state.interruption_listener = in_interruption_listener;
    state.client_data = in_client_data;
    state.is_initialized = true;

    log!(
        "AudioSessionInitialize({:?}, {:?}, {:?}, {:?}) -> 0",
        in_run_loop,
        in_run_loop_mode,
        in_interruption_listener,
        in_client_data
    );
    0
}

fn AudioSessionGetPropertySize(
    env: &mut Environment,
    in_ID: AudioSessionPropertyID,
    out_data_size: MutPtr<u32>,
) -> OSStatus {
    let size = get_audio_session_property_size(in_ID);
    env.mem.write(out_data_size, size);
    0
}

fn AudioSessionGetProperty(
    env: &mut Environment,
    in_ID: AudioSessionPropertyID,
    io_data_size: MutPtr<u32>,
    out_data: MutVoidPtr,
) -> OSStatus {
    let required_size = get_audio_session_property_size(in_ID);
    let io_data_size_value = env.mem.read(io_data_size);

    if io_data_size_value != required_size {
        log!("Warning: AudioSessionGetProperty() failed");
        return kAudioSessionBadPropertySizeError;
    }

    let state = &env.framework_state.audio_toolbox.audio_session;

    match in_ID {
        kAudioSessionProperty_OtherAudioIsPlaying => env.mem.write(out_data.cast(), 0u32),
        kAudioSessionProperty_AudioCategory => env.mem.write(out_data.cast(), state.audio_session_category),
        kAudioSessionProperty_CurrentHardwareSampleRate => env.mem.write(out_data.cast(), state.current_hardware_sample_rate),
        kAudioSessionProperty_CurrentHardwareOutputNumberChannels => env.mem.write(out_data.cast(), state.current_hardware_output_number_channels),
        kAudioSessionProperty_CurrentHardwareOutputVolume => env.mem.write(out_data.cast(), state.current_hardware_output_volume),
        kAudioSessionProperty_CurrentHardwareIOBufferDuration => env.mem.write(out_data.cast(), state.current_hardware_io_buffer_duration),
        kAudioSessionProperty_AudioInputAvailable => env.mem.write(out_data.cast(), 1u32),
        kAudioSessionProperty_AudioRoute => env.mem.write(out_data.cast(), 0u32),
        _ => {
            log!("AudioSessionGetProperty() unimplemented property: {} -> returning 0", debug_fourcc(in_ID));
            env.mem.write(out_data.cast::<u32>(), 0u32);
        }
    }
    0
}

fn AudioSessionSetProperty(
    env: &mut Environment,
    in_ID: AudioSessionPropertyID,
    in_data_size: u32,
    in_data: ConstVoidPtr,
) -> OSStatus {
    let required_size: GuestUSize = match in_ID {
        kAudioSessionProperty_AudioCategory => guest_size_of::<u32>(),
        kAudioSessionProperty_PreferredHardwareIOBufferDuration => guest_size_of::<f32>(),
        kAudioSessionProperty_PreferredHardwareSampleRate => guest_size_of::<f64>(),
        _ => return 0,
    };

    if in_data_size != required_size {
        return kAudioSessionBadPropertySizeError;
    }

    if in_ID == kAudioSessionProperty_PreferredHardwareSampleRate {
        env.framework_state.audio_toolbox.audio_session.current_hardware_sample_rate = env.mem.read(in_data.cast::<f64>());
    }
    0
}

fn AudioSessionSetActive(_env: &mut Environment, _active: bool) -> OSStatus { 0 }
fn AudioSessionAddPropertyListener(_env: &mut Environment, _inID: AudioSessionPropertyID, _inProc: AudioSessionPropertyListener, _inClientData: MutVoidPtr) -> OSStatus { 0 }
fn AudioSessionRemovePropertyListenerWithUserData(_env: &mut Environment, _in_property_id: AudioSessionPropertyID, _in_listener: AudioSessionPropertyListener, _in_client_data: MutVoidPtr) -> OSStatus { 0 }

fn get_audio_session_property_size(in_ID: AudioSessionPropertyID) -> GuestUSize {
    match in_ID {
        kAudioSessionProperty_OtherAudioIsPlaying |
        kAudioSessionProperty_AudioCategory |
        kAudioSessionProperty_CurrentHardwareOutputNumberChannels |
        kAudioSessionProperty_AudioInputAvailable |
        kAudioSessionProperty_AudioRoute => guest_size_of::<u32>(),
        kAudioSessionProperty_CurrentHardwareSampleRate => guest_size_of::<f64>(),
        kAudioSessionProperty_CurrentHardwareOutputVolume |
        kAudioSessionProperty_CurrentHardwareIOBufferDuration => guest_size_of::<f32>(),
        _ => 4, // БЕЗОПАСНЫЙ ФОЛЛБЕК: возвращаем 4 байта вместо паники эмулятора
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioSessionInitialize(_, _, _, _)),
    export_c_func!(AudioSessionGetProperty(_, _, _)),
    export_c_func!(AudioSessionGetPropertySize(_, _)),
    export_c_func!(AudioSessionSetProperty(_, _, _)),
    export_c_func!(AudioSessionSetActive(_)),
    export_c_func!(AudioSessionAddPropertyListener(_, _, _)),
    export_c_func!(AudioSessionRemovePropertyListenerWithUserData(_, _, _)),
];

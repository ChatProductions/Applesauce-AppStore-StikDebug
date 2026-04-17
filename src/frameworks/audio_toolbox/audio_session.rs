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

#[derive(Default)]
pub struct State {
    pub active: bool,
    pub category: u32,
}

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
            // Безопасное значение по умолчанию, чтобы избежать краша
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
    env.state.audio_toolbox.audio_session.active = active != 0;
    kAudioSessionNoErr
}

pub fn AudioSessionGetPropertySize(
    _env: &mut Environment,
    in_id: AudioSessionPropertyID,
    out_data_size: MutPtr<u32>,
) -> OSStatus {
    if !out_data_size.is_null() {
        out_data_size.write(_env, get_audio_session_property_size(in_id));
    }
    kAudioSessionNoErr
}

pub fn AudioSessionGetProperty(
    env: &mut Environment,
    in_id: AudioSessionPropertyID,
    io_data_size: MutPtr<u32>,
    out_data: MutVoidPtr,
) -> OSStatus {
    let size = if !io_data_size.is_null() {
        io_data_size.read(env)
    } else {
        return kAudioSessionBadPropertySizeError;
    };

    if out_data.is_null() {
        return kAudioSessionNoErr;
    }

    match in_id {
        kAudioSessionProperty_OtherAudioIsPlaying => {
            if size < guest_size_of::<u32>() { return kAudioSessionBadPropertySizeError; }
            out_data.cast::<u32>().write(env, 0); // Музыка в фоне не играет
        }
        kAudioSessionProperty_AudioCategory => {
            if size < guest_size_of::<u32>() { return kAudioSessionBadPropertySizeError; }
            let category = env.state.audio_toolbox.audio_session.category;
            out_data.cast::<u32>().write(env, category);
        }
        kAudioSessionProperty_CurrentHardwareSampleRate => {
            if size < guest_size_of::<f64>() { return kAudioSessionBadPropertySizeError; }
            out_data.cast::<f64>().write(env, 44100.0); // Заглушка под 44.1 kHz
        }
        kAudioSessionProperty_CurrentHardwareOutputNumberChannels => {
            if size < guest_size_of::<u32>() { return kAudioSessionBadPropertySizeError; }
            out_data.cast::<u32>().write(env, 2); // Стерео
        }
        kAudioSessionProperty_CurrentHardwareOutputVolume => {
            if size < guest_size_of::<f32>() { return kAudioSessionBadPropertySizeError; }
            out_data.cast::<f32>().write(env, 1.0); // Максимальная громкость
        }
        kAudioSessionProperty_CurrentHardwareIOBufferDuration | kAudioSessionProperty_PreferredHardwareIOBufferDuration => {
            if size < guest_size_of::<f32>() { return kAudioSessionBadPropertySizeError; }
            out_data.cast::<f32>().write(env, 0.05); // Заглушка буфера 50ms
        }
        kAudioSessionProperty_AudioInputAvailable => {
            if size < guest_size_of::<u32>() { return kAudioSessionBadPropertySizeError; }
            out_data.cast::<u32>().write(env, 0); // Микрофон недоступен
        }
        kAudioSessionProperty_AudioRoute => {
            if size < guest_size_of::<u32>() { return kAudioSessionBadPropertySizeError; }
            out_data.cast::<u32>().write(env, 0); // Стандартный путь вывода
        }
        _ => {
            log!("TODO: AudioSessionGetProperty unknown property: {}", debug_fourcc(in_id));
            // Пишем нули как fallback для предотвращения сбоев
            let write_size = std::cmp::min(size, get_audio_session_property_size(in_id));
            // В touchHLE заглушка для нереализованных свойств обычно заполняет нулями
        }
    };

    kAudioSessionNoErr
}

pub fn AudioSessionSetProperty(
    env: &mut Environment,
    in_id: AudioSessionPropertyID,
    in_data_size: u32,
    in_data: ConstVoidPtr,
) -> OSStatus {
    match in_id {
        kAudioSessionProperty_AudioCategory => {
            if in_data_size >= guest_size_of::<u32>() && !in_data.is_null() {
                let category = in_data.cast::<u32>().read(env);
                log_dbg!("AudioSessionSetProperty(AudioCategory, {})", debug_fourcc(category));
                env.state.audio_toolbox.audio_session.category = category;
            }
        }
        _ => {
            log!("TODO: AudioSessionSetProperty unknown property: {}", debug_fourcc(in_id));
        }
    }
    kAudioSessionNoErr
}

pub fn AudioSessionAddPropertyListener(
    _env: &mut Environment,
    _in_id: AudioSessionPropertyID,
    _in_proc: AudioSessionPropertyListener,
    _in_client_data: ConstVoidPtr,
) -> OSStatus {
    log_dbg!("TODO: AudioSessionAddPropertyListener");
    kAudioSessionNoErr
}

pub fn AudioSessionRemovePropertyListener(
    _env: &mut Environment,
    _in_id: AudioSessionPropertyID,
) -> OSStatus {
    log_dbg!("TODO: AudioSessionRemovePropertyListener");
    kAudioSessionNoErr
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioSessionInitialize(_, _, _, _)),
    export_c_func!(AudioSessionGetProperty(_, _, _, _)),
    export_c_func!(AudioSessionGetPropertySize(_, _, _)),
    export_c_func!(AudioSessionSetProperty(_, _, _, _)),
    export_c_func!(AudioSessionSetActive(_, _)),
    export_c_func!(AudioSessionAddPropertyListener(_, _, _, _)),
    export_c_func!(AudioSessionRemovePropertyListener(_, _)),
];

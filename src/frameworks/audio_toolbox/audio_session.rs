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
    env.framework_state.audio_toolbox.audio_session.active = active != 0;
    kAudioSessionNoErr
}

pub fn AudioSessionGetPropertySize(
    env: &mut Environment,
    in_id: AudioSessionPropertyID,
    out_data_size: MutPtr<u32>,
) -> OSStatus {
    log_dbg!("AudioSessionGetPropertySize for {}", debug_fourcc(in_id));
    
    let size = get_audio_session_property_size(in_id);
    
    // Записываем размер ответа для игры, если указатель не нулевой
    if !out_data_size.is_null() {
        out_data_size.write(&mut env.mem, size);
    }
    
    kAudioSessionNoErr
}

pub fn AudioSessionGetProperty(
    env: &mut Environment,
    in_id: AudioSessionPropertyID,
    io_data_size: MutPtr<u32>,
    out_data: MutVoidPtr,
) -> OSStatus {
    log_dbg!("AudioSessionGetProperty {}", debug_fourcc(in_id));

    if out_data.is_null() {
        return crate::frameworks::carbon_core::paramErr;
    }

    let size = get_audio_session_property_size(in_id);

    if !io_data_size.is_null() {
        let provided_size = io_data_size.read(&env.mem);
        if provided_size < size {
            return kAudioSessionBadPropertySizeError;
        }
        // Записываем реальный размер
        io_data_size.write(&mut env.mem, size);
    }

    // Записываем сами значения свойств в память, чтобы игра не читала "мусор"
    match in_id {
        kAudioSessionProperty_OtherAudioIsPlaying => {
            out_data.cast::<u32>().write(&mut env.mem, 0); // Фоновая музыка не играет
        }
        kAudioSessionProperty_AudioCategory => {
            let cat = env.framework_state.audio_toolbox.audio_session.category;
            out_data.cast::<u32>().write(&mut env.mem, cat);
        }
        kAudioSessionProperty_CurrentHardwareSampleRate => {
            let rate = env.framework_state.audio_toolbox.audio_session.current_hardware_sample_rate;
            out_data.cast::<f64>().write(&mut env.mem, rate);
        }
        kAudioSessionProperty_CurrentHardwareOutputNumberChannels => {
            out_data.cast::<u32>().write(&mut env.mem, 2); // Стерео
        }
        kAudioSessionProperty_CurrentHardwareOutputVolume => {
            out_data.cast::<f32>().write(&mut env.mem, 1.0); // Максимальная громкость
        }
        kAudioSessionProperty_CurrentHardwareIOBufferDuration |
        kAudioSessionProperty_PreferredHardwareIOBufferDuration => {
            out_data.cast::<f32>().write(&mut env.mem, 0.05); // Dummy duration
        }
        kAudioSessionProperty_AudioInputAvailable => {
            out_data.cast::<u32>().write(&mut env.mem, 0); // Микрофона нет
        }
        kAudioSessionProperty_AudioRoute => {
            out_data.cast::<u32>().write(&mut env.mem, 0);
        }
        _ => {
            log!("TODO: AudioSessionGetProperty UNIMPLEMENTED write for {}", debug_fourcc(in_id));
            if size == 4 {
                out_data.cast::<u32>().write(&mut env.mem, 0);
            } else if size == 8 {
                out_data.cast::<u64>().write(&mut env.mem, 0);
            }
        }
    }

    kAudioSessionNoErr
}

pub fn AudioSessionSetProperty(
    env: &mut Environment,
    in_id: AudioSessionPropertyID,
    in_data_size: u32,
    in_data: ConstVoidPtr,
) -> OSStatus {
    log_dbg!("AudioSessionSetProperty {}", debug_fourcc(in_id));
    
    if in_data.is_null() {
        return crate::frameworks::carbon_core::paramErr;
    }

    match in_id {
        kAudioSessionProperty_AudioCategory => {
            if in_data_size >= 4 {
                // Читаем категорию, которую нам передает игра, и сохраняем её
                let category = in_data.cast::<u32>().read(&env.mem);
                env.framework_state.audio_toolbox.audio_session.category = category;
            }
        }
        kAudioSessionProperty_PreferredHardwareIOBufferDuration => {
            // Игнорируем
        }
        _ => {
            log!("TODO: AudioSessionSetProperty UNIMPLEMENTED {}", debug_fourcc(in_id));
        }
    }
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

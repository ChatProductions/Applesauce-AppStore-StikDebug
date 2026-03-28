/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioUnit.h` (Audio Unit Services)

// Позволяем компилятору игнорировать мелкие предупреждения, чтобы сборка не прерывалась
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use std::time::Instant;

use crate::audio::openal::al_types::{ALuint, ALvoid};
use crate::audio::openal::{AL_BUFFERS_PROCESSED, AL_PLAYING, AL_SOURCE_STATE};

use crate::abi::CallFromHost;
use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::audio_toolbox::audio_components;
use crate::frameworks::audio_toolbox::audio_queue::{
    is_supported_audio_format, log_if_broken_audio_format,
};
use crate::frameworks::carbon_core::{paramErr, OSStatus};
use crate::frameworks::core_audio_types::AudioStreamBasicDescription;
use crate::frameworks::core_foundation::cf_run_loop::CFRunLoopGetMain;
use crate::frameworks::foundation::ns_run_loop;
use crate::mem::{guest_size_of, ConstVoidPtr, MutPtr, MutVoidPtr, SafeRead};
use crate::objc::nil;

use super::audio_components::{AURenderCallbackStruct, AudioComponentInstance};
use super::audio_queue::decode_buffer;
use super::audio_session;

pub type AudioUnit = AudioComponentInstance;
type AudioUnitPropertyID = u32;
type AudioUnitScope = u32;
type AudioUnitElement = u32;

// --- СТРУКТУРЫ ---

#[repr(C, packed)]
pub struct AudioBufferList<const COUNT: usize> {
    pub number_buffers: u32,
    pub buffers: [AudioBuffer; COUNT],
}
unsafe impl SafeRead for AudioBufferList<1> {}
unsafe impl SafeRead for AudioBufferList<2> {}

#[repr(C, packed)]
pub struct AudioBuffer {
    pub number_channels: u32,
    pub data_byte_size: u32,
    pub data: MutVoidPtr,
}

// -----------------------------------------------------

const kAudioUnitScope_Global: AudioUnitScope = 0;
const kAudioUnitScope_Input: AudioUnitScope = 1;
const kAudioUnitScope_Output: AudioUnitScope = 2;

const kAudioUnitProperty_SampleRate: AudioUnitPropertyID = 2;
const kAudioUnitProperty_SetRenderCallback: AudioUnitPropertyID = 23;
const kAudioUnitProperty_MaximumFramesPerSlice: AudioUnitPropertyID = 14;
const kAudioUnitProperty_StreamFormat: AudioUnitPropertyID = 8;

const kAudioOutputUnitProperty_EnableIO: AudioUnitPropertyID = 2003;

fn AudioUnitInitialize(env: &mut Environment, in_unit: AudioUnit) -> OSStatus {
    let run_loop = CFRunLoopGetMain(env);
    ns_run_loop::add_audio_unit(env, run_loop, in_unit);
    0 
}

fn AudioUnitUninitialize(env: &mut Environment, in_unit: AudioUnit) -> OSStatus {
    let run_loop = CFRunLoopGetMain(env);
    match ns_run_loop::remove_audio_unit(env, run_loop, in_unit) {
        Ok(_) => 0,
        Err(_) => paramErr, 
    }
}

fn AudioUnitSetProperty(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    in_data: ConstVoidPtr,
    in_data_size: u32,
) -> OSStatus {
    assert!(in_element == 0);

    let host_object = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
        .unwrap();

    let result;
    match in_id {
        kAudioUnitProperty_SetRenderCallback => {
            assert_eq!(in_scope, kAudioUnitScope_Global);
            assert_eq!(in_data_size, guest_size_of::<AURenderCallbackStruct>());
            let render_callback = env.mem.read(in_data.cast::<AURenderCallbackStruct>());
            host_object.render_callback = Some(render_callback);
            result = 0;
            log_dbg!("AudioUnitSetProperty({:?}, kAudioUnitProperty_SetRenderCallback, ...)", in_unit);
        }
        kAudioUnitProperty_StreamFormat => {
            assert_eq!(in_data_size, guest_size_of::<AudioStreamBasicDescription>());
            let stream_format = env.mem.read(in_data.cast::<AudioStreamBasicDescription>());
            log_if_broken_audio_format(&stream_format);
            match in_scope {
                kAudioUnitScope_Global => host_object.global_stream_format = stream_format,
                kAudioUnitScope_Output => host_object.output_stream_format = Some(stream_format),
                kAudioUnitScope_Input => host_object.input_stream_format = Some(stream_format),
                _ => unimplemented!("in_scope {}", in_scope),
            };
            result = 0;
        }
        kAudioOutputUnitProperty_EnableIO => {
            result = 0;
        }
        _ => {
            log_dbg!("AudioUnitSetProperty: unknown property {}", in_id);
            result = 0;
        }
    };

    result
}

fn AudioUnitGetProperty(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    out_data: MutVoidPtr,
    io_data_size: MutPtr<u32>,
) -> OSStatus {
    assert!(in_element == 0);

    let host_object = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
        .unwrap();

    match in_id {
        kAudioUnitProperty_MaximumFramesPerSlice => {
            assert_eq!(env.mem.read(io_data_size), guest_size_of::<u32>());
            let max_frames: u32 = host_object.maximum_frames_per_slice;
            env.mem.write(out_data.cast(), max_frames);
            env.mem.write(io_data_size.cast(), guest_size_of::<u32>());
        }
        kAudioUnitProperty_StreamFormat => {
            let stream_format = match in_scope {
                kAudioUnitScope_Global => host_object.global_stream_format,
                kAudioUnitScope_Output => host_object.output_stream_format.unwrap(),
                kAudioUnitScope_Input => host_object.input_stream_format.unwrap(),
                _ => unimplemented!(),
            };
            env.mem.write(out_data.cast(), stream_format);
            env.mem.write(io_data_size.cast(), guest_size_of::<AudioStreamBasicDescription>());
        }
        kAudioUnitProperty_SampleRate => {
            let sample_rate = match in_scope {
                kAudioUnitScope_Global => host_object.global_stream_format.sample_rate,
                _ => host_object.global_stream_format.sample_rate,
            };
            env.mem.write(out_data.cast(), sample_rate);
            env.mem.write(io_data_size.cast(), guest_size_of::<f64>());
        }
        _ => return -1,
    };
    0 
}

fn AudioOutputUnitStart(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    let context = env.framework_state.audio_toolbox.make_al_context_current(&mut env.openal_manager);
    let mut source: ALuint = 0;
    unsafe {
        context.GenSources(1, &mut source);
        context.SourcePlay(source);
    }

    let audio_components_state = audio_components::State::get(&mut env.framework_state);
    let audio_unit_state = audio_components_state.audio_component_instances.get_mut(&ci).unwrap();
    audio_unit_state.al_source = Some(source);
    audio_unit_state.last_render_time = Some(Instant::now());
    audio_unit_state.started = true;
    0
}

fn AudioOutputUnitStop(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    let at_state = &mut env.framework_state.audio_toolbox;
    let context = at_state.al_context.make_al_context_current(&mut env.openal_manager);

    if let Some(audio_unit_state) = at_state.audio_components.audio_component_instances.get_mut(&ci) {
        audio_unit_state.started = false;
        if let Some(al_source) = audio_unit_state.al_source {
            unsafe { context.DeleteSources(1, &al_source); }
        }
        audio_unit_state.al_source = None;
        0
    } else {
        -1
    }
}

// --- ТА САМАЯ ЗАГЛУШКА ---
fn AudioUnitAddRenderNotify(
    _env: &mut Environment,
    in_unit: AudioUnit,
    in_proc: ConstVoidPtr,
    in_proc_ref_con: ConstVoidPtr,
) -> OSStatus {
    log_dbg!("STUB: AudioUnitAddRenderNotify called");
    0 
}

pub fn render_audio_unit(env: &mut Environment, audio_unit: AudioUnit) {
    if env.bundle.bundle_identifier().starts_with("com.ea.simcity") {
        return;
    }

    let at_state = &mut env.framework_state.audio_toolbox;
    let context = at_state.al_context.make_al_context_current(&mut env.openal_manager);
    let current_hardware_sample_rate = at_state.audio_session.current_hardware_sample_rate;

    let audio_unit_host_object = at_state.audio_components.audio_component_instances.get_mut(&audio_unit).unwrap();

    if !audio_unit_host_object.started || audio_unit_host_object.is_running_handler {
        return;
    }

    audio_unit_host_object.is_running_handler = true;

    let input_stream_format = audio_unit_host_object.input_stream_format;
    let output_stream_format = audio_unit_host_object.output_stream_format;
    let stream_format = input_stream_format.unwrap_or(output_stream_format.unwrap_or(audio_unit_host_object.global_stream_format));
    let sample_rate = input_stream_format.map(|f| f.sample_rate).unwrap_or(current_hardware_sample_rate);

    let al_source = audio_unit_host_object.al_source.unwrap();
    let mut al_buffers = Vec::new();
    unsafe {
        let mut buffers_processed = 0;
        context.GetSourcei(al_source, AL_BUFFERS_PROCESSED, &mut buffers_processed);
        while buffers_processed > 0 {
            let mut al_buffer = 0;
            context.SourceUnqueueBuffers(al_source, 1, &mut al_buffer);
            al_buffers.push(al_buffer);
            context.GetSourcei(al_source, AL_BUFFERS_PROCESSED, &mut buffers_processed);
        }
    }

    let now = Instant::now();
    let elapsed_time = now.duration_since(audio_unit_host_object.last_render_time.unwrap());
    let number_frames = ((elapsed_time.as_secs_f64() * sample_rate) as u32).min(2048);
    let buffer_size = number_frames * stream_format.channels_per_frame * (stream_format.bits_per_channel / 8);

    let action_flags = env.mem.alloc_and_write(0u32);
    let buffer_data = env.mem.alloc(buffer_size);
    let audio_buffer_list = AudioBufferList {
        number_buffers: 1,
        buffers: [AudioBuffer {
            number_channels: stream_format.channels_per_frame,
            data_byte_size: buffer_size,
            data: buffer_data,
        }],
    };
    let abl_ptr = env.mem.alloc_and_write(audio_buffer_list);

    let callback = audio_unit_host_object.render_callback.unwrap();
    callback.input_proc.call_from_host(env, (callback.input_proc_ref_con, action_flags, nil.cast_void().cast_const(), 0u32, number_frames, abl_ptr.cast()));

    let (al_format, _, processed_data) = decode_buffer(&env.mem, &stream_format, buffer_data.cast(), buffer_size);

    unsafe {
        let al_buffer = al_buffers.pop().unwrap_or_else(|| {
            let mut b = 0;
            context.GenBuffers(1, &mut b);
            b
        });
        context.BufferData(al_buffer, al_format, processed_data.as_ptr() as *const ALvoid, processed_data.len() as i32, sample_rate as i32);
        context.SourceQueueBuffers(al_source, 1, &al_buffer);
        
        let mut state = 0;
        context.GetSourcei(al_source, AL_SOURCE_STATE, &mut state);
        if state != AL_PLAYING { context.SourcePlay(al_source); }
        if !al_buffers.is_empty() { context.DeleteBuffers(al_buffers.len() as i32, al_buffers.as_ptr()); }
    }

    env.mem.free(action_flags.cast_void());
    env.mem.free(buffer_data.cast_void());
    env.mem.free(abl_ptr.cast_void());

    let audio_unit_host_object = audio_components::State::get(&mut env.framework_state).audio_component_instances.get_mut(&audio_unit).unwrap();
    audio_unit_host_object.last_render_time = Some(now);
    audio_unit_host_object.is_running_handler = false;
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioUnitInitialize(_)),
    export_c_func!(AudioUnitUninitialize(_)),
    export_c_func!(AudioUnitSetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioOutputUnitStart(_)),
    export_c_func!(AudioOutputUnitStop(_)),
    export_c_func!(AudioUnitAddRenderNotify(_, _, _)), // ИСПРАВЛЕНО: 3 аргумента после env
];


/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioUnit.h` (Audio Unit Services)

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
type AudioUnitParameterID = u32;
type AudioUnitParameterValue = f32;

// =========================================================================
// MARK: - Structures
// =========================================================================

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

/// `AudioUnitConnection` — used by kAudioUnitProperty_MakeConnection.
#[repr(C, packed)]
#[derive(Copy, Clone)]
struct AudioUnitConnection {
    source_audio_unit:  AudioUnit,
    source_output_number: u32,
    dest_input_number:    u32,
}
unsafe impl SafeRead for AudioUnitConnection {}

// =========================================================================
// MARK: - Scope / element constants
// =========================================================================

const kAudioUnitScope_Global: AudioUnitScope = 0;
const kAudioUnitScope_Input:  AudioUnitScope = 1;
const kAudioUnitScope_Output: AudioUnitScope = 2;
const kAudioUnitScope_Group:  AudioUnitScope = 3;
const kAudioUnitScope_Part:   AudioUnitScope = 4;
const kAudioUnitScope_Note:   AudioUnitScope = 5;

// =========================================================================
// MARK: - Property ID constants
// =========================================================================

const kAudioUnitProperty_ClassInfo:              AudioUnitPropertyID = 0;
const kAudioUnitProperty_MakeConnection:         AudioUnitPropertyID = 1;
const kAudioUnitProperty_SampleRate:             AudioUnitPropertyID = 2;
const kAudioUnitProperty_ParameterList:          AudioUnitPropertyID = 3;
const kAudioUnitProperty_ParameterInfo:          AudioUnitPropertyID = 4;
const kAudioUnitProperty_CPULoad:                AudioUnitPropertyID = 6;
const kAudioUnitProperty_StreamFormat:           AudioUnitPropertyID = 8;
const kAudioUnitProperty_ElementCount:           AudioUnitPropertyID = 11;
const kAudioUnitProperty_Latency:                AudioUnitPropertyID = 12;
const kAudioUnitProperty_SupportedNumChannels:   AudioUnitPropertyID = 13;
const kAudioUnitProperty_MaximumFramesPerSlice:  AudioUnitPropertyID = 14;
const kAudioUnitProperty_ParameterValueStrings:  AudioUnitPropertyID = 16;
const kAudioUnitProperty_AudioChannelLayout:     AudioUnitPropertyID = 19;
const kAudioUnitProperty_TailTime:               AudioUnitPropertyID = 20;
const kAudioUnitProperty_BypassEffect:           AudioUnitPropertyID = 21;
const kAudioUnitProperty_LastRenderError:        AudioUnitPropertyID = 22;
const kAudioUnitProperty_SetRenderCallback:      AudioUnitPropertyID = 23;
const kAudioUnitProperty_FactoryPresets:         AudioUnitPropertyID = 24;
const kAudioUnitProperty_RenderQuality:          AudioUnitPropertyID = 26;
const kAudioUnitProperty_HostCallbacks:          AudioUnitPropertyID = 27;
const kAudioUnitProperty_InPlaceProcessing:      AudioUnitPropertyID = 29;
const kAudioUnitProperty_ElementName:            AudioUnitPropertyID = 30;
const kAudioUnitProperty_SupportedChannelLayoutTags: AudioUnitPropertyID = 32;
const kAudioUnitProperty_PresentPreset:          AudioUnitPropertyID = 36;
const kAudioUnitProperty_DependentParameters:    AudioUnitPropertyID = 45;
const kAudioUnitProperty_InputSamplesInOutput:   AudioUnitPropertyID = 49;
const kAudioUnitProperty_ShouldAllocateBuffer:   AudioUnitPropertyID = 51;
const kAudioUnitProperty_FrequencyResponse:      AudioUnitPropertyID = 52;
const kAudioUnitProperty_ParameterHistoryInfo:   AudioUnitPropertyID = 53;
const kAudioUnitProperty_NickName:               AudioUnitPropertyID = 54;
const kAudioUnitProperty_OfflineRender:          AudioUnitPropertyID = 37;
const kAudioUnitProperty_ParameterIDName:        AudioUnitPropertyID = 34;
const kAudioOutputUnitProperty_EnableIO:         AudioUnitPropertyID = 2003;
const kAudioOutputUnitProperty_HasIO:            AudioUnitPropertyID = 2006;
const kAudioOutputUnitProperty_StartTime:        AudioUnitPropertyID = 2004;
const kAudioOutputUnitProperty_SetInputCallback: AudioUnitPropertyID = 2005;
const kAudioOutputUnitProperty_IsRunning:        AudioUnitPropertyID = 2001;
const kAudioMixerProperty_Volume:                AudioUnitPropertyID = 7;
const kAudioMixerProperty_Metering:              AudioUnitPropertyID = 1003;
const kAudioUnitProperty_MeteringMode:           AudioUnitPropertyID = 1003;

// =========================================================================
// MARK: - AudioUnitInitialize / Uninitialize
// =========================================================================

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

// =========================================================================
// MARK: - AudioUnitSetProperty
// =========================================================================

fn AudioUnitSetProperty(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    in_data: ConstVoidPtr,
    in_data_size: u32,
) -> OSStatus {
    if in_element != 0 {
        log_dbg!(
            "AudioUnitSetProperty: ignoring non-zero element {}",
            in_element
        );
        // Don't return error — many apps set properties on bus 1 etc.
        return 0;
    }

    let Some(host_object) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
    else {
        log_dbg!(
            "AudioUnitSetProperty: unknown audio unit {:?}, returning paramErr",
            in_unit
        );
        return paramErr;
    };

    match in_id {
        kAudioUnitProperty_SetRenderCallback => {
            let render_callback = env.mem.read(in_data.cast::<AURenderCallbackStruct>());
            host_object.render_callback = Some(render_callback);
        }
        kAudioOutputUnitProperty_SetInputCallback => {
            let cb = env.mem.read(in_data.cast::<AURenderCallbackStruct>());
            host_object.render_callback = Some(cb);
        }
        kAudioUnitProperty_StreamFormat => {
            let stream_format = env.mem.read(in_data.cast::<AudioStreamBasicDescription>());
            log_if_broken_audio_format(&stream_format);
            match in_scope {
                kAudioUnitScope_Global => host_object.global_stream_format = stream_format,
                kAudioUnitScope_Output => host_object.output_stream_format = Some(stream_format),
                kAudioUnitScope_Input  => host_object.input_stream_format  = Some(stream_format),
                _ => log_dbg!("AudioUnitSetProperty StreamFormat: unsupported scope {}", in_scope),
            }
        }
        kAudioUnitProperty_SampleRate => {
            let rate: f64 = env.mem.read(in_data.cast());
            host_object.global_stream_format.sample_rate = rate;
            log_dbg!("AudioUnitSetProperty: sample rate set to {}", rate);
        }
        kAudioUnitProperty_MaximumFramesPerSlice => {
            let frames: u32 = env.mem.read(in_data.cast());
            host_object.maximum_frames_per_slice = frames;
            log_dbg!("AudioUnitSetProperty: maximum frames per slice = {}", frames);
        }
        kAudioUnitProperty_BypassEffect => {
            let bypass: u32 = env.mem.read(in_data.cast());
            log_dbg!("AudioUnitSetProperty: bypass effect = {}", bypass);
        }
        kAudioUnitProperty_InPlaceProcessing => {
            let v: u32 = env.mem.read(in_data.cast());
            log_dbg!("AudioUnitSetProperty: in-place processing = {}", v);
        }
        kAudioUnitProperty_ShouldAllocateBuffer => {
            let v: u32 = env.mem.read(in_data.cast());
            log_dbg!("AudioUnitSetProperty: should allocate buffer = {}", v);
        }
        kAudioUnitProperty_RenderQuality => {
            let q: u32 = env.mem.read(in_data.cast());
            log_dbg!("AudioUnitSetProperty: render quality = {}", q);
        }
        kAudioUnitProperty_HostCallbacks => {
            log_dbg!("AudioUnitSetProperty: host callbacks set (ignored)");
        }
        kAudioUnitProperty_MakeConnection => {
            let conn = env.mem.read(in_data.cast::<AudioUnitConnection>());
            let src = conn.source_audio_unit;
            let src_out = conn.source_output_number;
            let dest_in = conn.dest_input_number;
            log_dbg!(
                "AudioUnitSetProperty: MakeConnection src={:?} srcOut={} destIn={}",
                src, src_out, dest_in
            );
        }
        kAudioOutputUnitProperty_EnableIO => {
            log_dbg!("AudioUnitSetProperty: EnableIO (ignored)");
        }
        kAudioMixerProperty_Volume | kAudioMixerProperty_Metering => {
            log_dbg!("AudioUnitSetProperty: mixer property {} (ignored)", in_id);
        }
        kAudioUnitProperty_OfflineRender => {
            let v: u32 = env.mem.read(in_data.cast());
            log_dbg!("AudioUnitSetProperty: offline render = {}", v);
        }
        _ => {
            log_dbg!(
                "AudioUnitSetProperty: unknown property {} (scope={} element={}) — ignored",
                in_id, in_scope, in_element
            );
        }
    }
    0
}

// =========================================================================
// MARK: - AudioUnitGetProperty
// =========================================================================

fn AudioUnitGetProperty(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    out_data: MutVoidPtr,
    io_data_size: MutPtr<u32>,
) -> OSStatus {
    let Some(host_object) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
    else {
        log_dbg!(
            "AudioUnitGetProperty: unknown audio unit {:?}, returning paramErr",
            in_unit
        );
        return paramErr;
    };

    match in_id {
        kAudioUnitProperty_MaximumFramesPerSlice => {
            let v = host_object.maximum_frames_per_slice;
            env.mem.write(out_data.cast(), v);
            env.mem.write(io_data_size, guest_size_of::<u32>());
        }
        kAudioUnitProperty_StreamFormat => {
            let fmt = match in_scope {
                kAudioUnitScope_Output => host_object.output_stream_format
                    .unwrap_or(host_object.global_stream_format),
                kAudioUnitScope_Input  => host_object.input_stream_format
                    .unwrap_or(host_object.global_stream_format),
                _                      => host_object.global_stream_format,
            };
            env.mem.write(out_data.cast(), fmt);
            env.mem.write(io_data_size, guest_size_of::<AudioStreamBasicDescription>());
        }
        kAudioUnitProperty_SampleRate => {
            let rate = match in_scope {
                kAudioUnitScope_Output => host_object.output_stream_format
                    .map(|f| f.sample_rate)
                    .unwrap_or(host_object.global_stream_format.sample_rate),
                _ => host_object.global_stream_format.sample_rate,
            };
            env.mem.write(out_data.cast(), rate);
            env.mem.write(io_data_size, guest_size_of::<f64>());
        }
        kAudioUnitProperty_Latency => {
            let latency: f64 = 0.0;
            env.mem.write(out_data.cast(), latency);
            env.mem.write(io_data_size, guest_size_of::<f64>());
        }
        kAudioUnitProperty_TailTime => {
            let tail: f64 = 0.0;
            env.mem.write(out_data.cast(), tail);
            env.mem.write(io_data_size, guest_size_of::<f64>());
        }
        kAudioUnitProperty_ElementCount => {
            let count: u32 = 1;
            env.mem.write(out_data.cast(), count);
            env.mem.write(io_data_size, guest_size_of::<u32>());
        }
        kAudioUnitProperty_LastRenderError => {
            let err: u32 = 0;
            env.mem.write(out_data.cast(), err);
            env.mem.write(io_data_size, guest_size_of::<u32>());
        }
        kAudioUnitProperty_BypassEffect => {
            let v: u32 = 0;
            env.mem.write(out_data.cast(), v);
            env.mem.write(io_data_size, guest_size_of::<u32>());
        }
        kAudioUnitProperty_InPlaceProcessing => {
            let v: u32 = 1; // yes, in-place is supported
            env.mem.write(out_data.cast(), v);
            env.mem.write(io_data_size, guest_size_of::<u32>());
        }
        kAudioUnitProperty_ShouldAllocateBuffer => {
            let v: u32 = 1;
            env.mem.write(out_data.cast(), v);
            env.mem.write(io_data_size, guest_size_of::<u32>());
        }
        kAudioOutputUnitProperty_HasIO => {
            // Report that output is available, input is not.
            let has: u32 = if in_scope == kAudioUnitScope_Output { 1 } else { 0 };
            env.mem.write(out_data.cast(), has);
            env.mem.write(io_data_size, guest_size_of::<u32>());
        }
        kAudioOutputUnitProperty_IsRunning => {
            let running: u32 = if host_object.started { 1 } else { 0 };
            env.mem.write(out_data.cast(), running);
            env.mem.write(io_data_size, guest_size_of::<u32>());
        }
        kAudioUnitProperty_CPULoad => {
            let load: f32 = 0.0;
            env.mem.write(out_data.cast(), load);
            env.mem.write(io_data_size, guest_size_of::<f32>());
        }
        _ => {
            log_dbg!(
                "AudioUnitGetProperty: unknown property {} (scope={} element={}) — returning -1",
                in_id, in_scope, in_element
            );
            return -1;
        }
    }
    0
}

// =========================================================================
// MARK: - AudioUnitGetPropertyInfo
// =========================================================================

fn AudioUnitGetPropertyInfo(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    out_data_size: MutPtr<u32>,
    out_writable: MutPtr<bool>,
) -> OSStatus {
    // Map known properties to their sizes.
    let (size, writable) = match in_id {
        kAudioUnitProperty_StreamFormat           => (guest_size_of::<AudioStreamBasicDescription>(), true),
        kAudioUnitProperty_SampleRate             => (guest_size_of::<f64>(), true),
        kAudioUnitProperty_MaximumFramesPerSlice  => (guest_size_of::<u32>(), true),
        kAudioUnitProperty_Latency                => (guest_size_of::<f64>(), false),
        kAudioUnitProperty_TailTime               => (guest_size_of::<f64>(), false),
        kAudioUnitProperty_ElementCount           => (guest_size_of::<u32>(), false),
        kAudioUnitProperty_LastRenderError        => (guest_size_of::<u32>(), false),
        kAudioUnitProperty_BypassEffect           => (guest_size_of::<u32>(), true),
        kAudioUnitProperty_InPlaceProcessing      => (guest_size_of::<u32>(), true),
        kAudioUnitProperty_ShouldAllocateBuffer   => (guest_size_of::<u32>(), true),
        kAudioOutputUnitProperty_HasIO            => (guest_size_of::<u32>(), false),
        kAudioOutputUnitProperty_IsRunning        => (guest_size_of::<u32>(), false),
        kAudioUnitProperty_CPULoad                => (guest_size_of::<f32>(), false),
        kAudioUnitProperty_SetRenderCallback      => (guest_size_of::<AURenderCallbackStruct>(), true),
        _ => {
            log_dbg!(
                "AudioUnitGetPropertyInfo: unknown property {} (scope={} element={}) — returning -1",
                in_id, in_scope, in_element
            );
            return -1;
        }
    };

    if !out_data_size.is_null() {
        env.mem.write(out_data_size, size);
    }
    if !out_writable.is_null() {
        env.mem.write(out_writable, writable);
    }
    log_dbg!(
        "AudioUnitGetPropertyInfo: property {} size={} writable={}",
        in_id, size, writable
    );
    0
}

// =========================================================================
// MARK: - Parameter get/set
// =========================================================================

fn AudioUnitSetParameter(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitParameterID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    in_value: AudioUnitParameterValue,
    in_buffer_offset_in_frames: u32,
) -> OSStatus {
    log_dbg!(
        "AudioUnitSetParameter: unit={:?} id={} scope={} element={} value={} offset={}",
        in_unit, in_id, in_scope, in_element, in_value, in_buffer_offset_in_frames
    );
    // Volume parameter - log but ignore (field not in host object)
    if in_id == 0 /* kMultiChannelMixerParam_Volume */ || in_id == 7 {
        log_dbg!("AudioUnitSetParameter: volume={} (ignored)", in_value);
    }
    0
}

fn AudioUnitGetParameter(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitParameterID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    out_value: MutPtr<AudioUnitParameterValue>,
) -> OSStatus {
    log_dbg!(
        "AudioUnitGetParameter: unit={:?} id={} scope={} element={}",
        in_unit, in_id, in_scope, in_element
    );
    let value = if let Some(obj) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get(&in_unit)
    {
        let _ = obj; // volume not stored, return 1.0
        1.0
    } else {
        1.0
    };
    if !out_value.is_null() {
        env.mem.write(out_value, value);
    }
    0
}

fn AudioUnitScheduleParameters(
    _env: &mut Environment,
    _in_unit: AudioUnit,
    _in_parameter_event: ConstVoidPtr,
    _in_num_parameter_events: u32,
) -> OSStatus {
    log_dbg!("AudioUnitScheduleParameters — ignored");
    0
}

// =========================================================================
// MARK: - Reset
// =========================================================================

fn AudioUnitReset(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
) -> OSStatus {
    log_dbg!(
        "AudioUnitReset: unit={:?} scope={} element={}",
        in_unit, in_scope, in_element
    );
    if let Some(obj) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
    {
        obj.last_render_time = None;
    }
    0
}

// =========================================================================
// MARK: - AudioOutputUnitStart / Stop
// =========================================================================

fn AudioOutputUnitStart(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    let context = env.framework_state
        .audio_toolbox
        .make_al_context_current(&mut env.openal_manager);
    let mut source: ALuint = 0;
    unsafe {
        context.GenSources(1, &mut source);
        context.SourcePlay(source);
    }

    let audio_components_state = audio_components::State::get(&mut env.framework_state);
    let Some(audio_unit_state) = audio_components_state
        .audio_component_instances
        .get_mut(&ci)
    else {
        log_dbg!("AudioOutputUnitStart: unknown audio unit {:?}", ci);
        return paramErr;
    };
    audio_unit_state.al_source = Some(source);
    audio_unit_state.last_render_time = Some(Instant::now());
    audio_unit_state.started = true;
    0
}

fn AudioOutputUnitStop(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    let at_state = &mut env.framework_state.audio_toolbox;
    let context = at_state.al_context.make_al_context_current(&mut env.openal_manager);

    if let Some(audio_unit_state) = at_state
        .audio_components
        .audio_component_instances
        .get_mut(&ci)
    {
        audio_unit_state.started = false;
        if let Some(al_source) = audio_unit_state.al_source {
            unsafe { context.DeleteSources(1, &al_source); }
        }
        audio_unit_state.al_source = None;
        0
    } else {
        log_dbg!("AudioOutputUnitStop: unknown audio unit {:?}", ci);
        -1
    }
}

// =========================================================================
// MARK: - Render notify / process
// =========================================================================

fn AudioUnitAddRenderNotify(
    _env: &mut Environment,
    in_unit: AudioUnit,
    in_proc: ConstVoidPtr,
    in_proc_ref_con: ConstVoidPtr,
) -> OSStatus {
    log_dbg!(
        "AudioUnitAddRenderNotify: unit={:?} proc={:?} refcon={:?} — ignored",
        in_unit, in_proc, in_proc_ref_con
    );
    0
}

fn AudioUnitRemoveRenderNotify(
    _env: &mut Environment,
    in_unit: AudioUnit,
    in_proc: ConstVoidPtr,
    in_proc_ref_con: ConstVoidPtr,
) -> OSStatus {
    log_dbg!(
        "AudioUnitRemoveRenderNotify: unit={:?} — ignored",
        in_unit
    );
    0
}

fn AudioUnitRender(
    env: &mut Environment,
    in_unit: AudioUnit,
    io_action_flags: MutPtr<u32>,
    _in_time_stamp: ConstVoidPtr,
    in_output_bus_number: u32,
    in_number_frames: u32,
    io_data: MutVoidPtr,
) -> OSStatus {
    // Pull audio by calling render_audio_unit which invokes the app's callback.
    log_dbg!(
        "AudioUnitRender: unit={:?} bus={} frames={}",
        in_unit, in_output_bus_number, in_number_frames
    );
    render_audio_unit(env, in_unit);
    0
}

fn AudioUnitProcess(
    env: &mut Environment,
    in_unit: AudioUnit,
    io_action_flags: MutPtr<u32>,
    _in_time_stamp: ConstVoidPtr,
    in_number_frames: u32,
    io_data: MutVoidPtr,
) -> OSStatus {
    log_dbg!(
        "AudioUnitProcess: unit={:?} frames={} — delegating to render",
        in_unit, in_number_frames
    );
    render_audio_unit(env, in_unit);
    0
}

fn AudioUnitProcessMultiple(
    _env: &mut Environment,
    in_unit: AudioUnit,
    io_action_flags: MutPtr<u32>,
    _in_time_stamp: ConstVoidPtr,
    in_number_frames: u32,
    _in_num_input_bus_buffers: u32,
    _io_input_bus_buffer_list: ConstVoidPtr,
    _io_output_bus_buffer_list: MutVoidPtr,
) -> OSStatus {
    log_dbg!(
        "AudioUnitProcessMultiple: unit={:?} frames={} — ignored",
        in_unit, in_number_frames
    );
    0
}

fn AudioUnitComplexRender(
    _env: &mut Environment,
    _in_unit: AudioUnit,
    _io_action_flags: MutPtr<u32>,
    _in_time_stamp: ConstVoidPtr,
    _in_output_bus_number: u32,
    _in_number_frames: u32,
    _out_num_packets_per_slice: MutPtr<u32>,
    _out_packet_descriptions: MutVoidPtr,
    _io_data: MutVoidPtr,
) -> OSStatus {
    log_dbg!("AudioUnitComplexRender — ignored");
    0
}

// =========================================================================
// MARK: - render_audio_unit (internal)
// =========================================================================

pub fn render_audio_unit(env: &mut Environment, audio_unit: AudioUnit) {
    if env.bundle.bundle_identifier().starts_with("com.ea.simcity") {
        return;
    }

    let (
        current_hardware_sample_rate,
        started,
        is_running_handler,
        input_stream_format,
        output_stream_format,
        global_stream_format,
        al_source,
        last_render_time,
        render_callback,
    ) = {
        let at = &mut env.framework_state.audio_toolbox;
        let Some(obj) = at.audio_components
            .audio_component_instances
            .get_mut(&audio_unit)
        else {
            return;
        };
        (
            at.audio_session.current_hardware_sample_rate,
            obj.started,
            obj.is_running_handler,
            obj.input_stream_format,
            obj.output_stream_format,
            obj.global_stream_format,
            obj.al_source,
            obj.last_render_time,
            obj.render_callback,
        )
    };

    if !started || is_running_handler { return; }

    {
        let at = &mut env.framework_state.audio_toolbox;
        if let Some(obj) = at.audio_components
            .audio_component_instances
            .get_mut(&audio_unit)
        {
            obj.is_running_handler = true;
        } else {
            return;
        }
    }

    let stream_format = input_stream_format
        .unwrap_or(output_stream_format.unwrap_or(global_stream_format));
    let sample_rate = input_stream_format
        .map(|f| f.sample_rate)
        .unwrap_or(current_hardware_sample_rate);

    let Some(al_source)       = al_source       else { return; };
    let Some(last_render_time) = last_render_time else { return; };
    let Some(callback)         = render_callback  else { return; };

    let mut al_buffers = Vec::new();
    {
        let at = &mut env.framework_state.audio_toolbox;
        let context = at.al_context.make_al_context_current(&mut env.openal_manager);
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
    }

    let now = Instant::now();
    let elapsed_time = now.duration_since(last_render_time);
    let number_frames = ((elapsed_time.as_secs_f64() * sample_rate) as u32).min(2048);
    let bytes_per_chan = stream_format.bits_per_channel / 8;
    let buffer_size = number_frames * stream_format.channels_per_frame * bytes_per_chan;

    let action_flags = env.mem.alloc_and_write(0u32);
    let buffer_data  = env.mem.alloc(buffer_size);
    let audio_buffer_list = AudioBufferList {
        number_buffers: 1,
        buffers: [AudioBuffer {
            number_channels: stream_format.channels_per_frame,
            data_byte_size:  buffer_size,
            data:            buffer_data,
        }],
    };
    let abl_ptr = env.mem.alloc_and_write(audio_buffer_list);

    let input_proc         = callback.input_proc;
    let input_proc_ref_con = callback.input_proc_ref_con;

    let _: OSStatus = input_proc.call_from_host(
        env,
        (
            input_proc_ref_con,
            action_flags,
            nil.cast_void().cast_const(),
            0u32,
            number_frames,
            abl_ptr.cast::<AudioBufferList<1>>(),
        ),
    );

    let (al_format, _, processed_data) = decode_buffer(
        &env.mem, &stream_format, buffer_data.cast(), buffer_size,
    );

    {
        let at = &mut env.framework_state.audio_toolbox;
        let context = at.al_context.make_al_context_current(&mut env.openal_manager);
        unsafe {
            let al_buffer = al_buffers.pop().unwrap_or_else(|| {
                let mut b = 0;
                context.GenBuffers(1, &mut b);
                b
            });
            context.BufferData(
                al_buffer,
                al_format,
                processed_data.as_ptr() as *const ALvoid,
                processed_data.len() as i32,
                sample_rate as i32,
            );
            context.SourceQueueBuffers(al_source, 1, &al_buffer);
            let mut state = 0;
            context.GetSourcei(al_source, AL_SOURCE_STATE, &mut state);
            if state != AL_PLAYING {
                context.SourcePlay(al_source);
            }
            if !al_buffers.is_empty() {
                context.DeleteBuffers(al_buffers.len() as i32, al_buffers.as_ptr());
            }
        }
    }

    env.mem.free(action_flags.cast_void());
    env.mem.free(buffer_data.cast_void());
    env.mem.free(abl_ptr.cast_void());

    if let Some(obj) = env.framework_state
        .audio_toolbox
        .audio_components
        .audio_component_instances
        .get_mut(&audio_unit)
    {
        obj.last_render_time    = Some(now);
        obj.is_running_handler  = false;
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioUnitInitialize(_)),
    export_c_func!(AudioUnitUninitialize(_)),
    export_c_func!(AudioUnitSetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetPropertyInfo(_, _, _, _, _, _)),
    export_c_func!(AudioUnitSetParameter(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetParameter(_, _, _, _, _)),
    export_c_func!(AudioUnitScheduleParameters(_, _, _)),
    export_c_func!(AudioUnitReset(_, _, _)),
    export_c_func!(AudioOutputUnitStart(_)),
    export_c_func!(AudioOutputUnitStop(_)),
    export_c_func!(AudioUnitAddRenderNotify(_, _, _)),
    export_c_func!(AudioUnitRemoveRenderNotify(_, _, _)),
    export_c_func!(AudioUnitRender(_, _, _, _, _, _)),
    export_c_func!(AudioUnitProcess(_, _, _, _, _)),
    export_c_func!(AudioUnitProcessMultiple(_, _, _, _, _, _, _)),
];

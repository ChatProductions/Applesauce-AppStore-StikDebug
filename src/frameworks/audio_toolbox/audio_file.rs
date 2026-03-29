/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioFile.h` (Audio File Services)

#![allow(dead_code)]

use crate::abi::{CallFromHost, GuestFunction};
use crate::audio;
use crate::audio::AudioDescription;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::{eofErr, OSStatus};
use crate::frameworks::core_audio_types::{
    debug_fourcc, fourcc, kAudioFormatAppleIMA4, kAudioFormatFlagIsBigEndian,
    kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked,
    kAudioFormatFlagIsSignedInteger, kAudioFormatLinearPCM,
    AudioStreamBasicDescription,
};
use crate::frameworks::audio_toolbox::audio_unit::AudioBufferList;
use crate::frameworks::core_foundation::cf_url::CFURLRef;
use crate::mem::{guest_size_of, MutPtr, MutVoidPtr, SafeRead};
use crate::Environment;
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    pub audio_files: HashMap<AudioFileID, AudioFileHostObject>,
}

impl State {
    pub fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.audio_file
    }
}

pub struct AudioFileHostObject {
    pub audio_file: audio::AudioFile,
    pub client_data_format: Option<AudioStreamBasicDescription>,
    pub read_pos_bytes: u64,
}

#[repr(C, packed)]
pub struct OpaqueAudioFileID {
    _filler: u8,
}
unsafe impl SafeRead for OpaqueAudioFileID {}

pub type AudioFileID = MutPtr<OpaqueAudioFileID>;

const kAudioFileBadPropertySizeError: OSStatus = fourcc(b"!siz") as _;
const kAudioFileUnsupportedProperty: OSStatus = fourcc(b"pty?") as _;
const kAudioFileUnsupportedFileTypeError: OSStatus = fourcc(b"typ?") as _;
const kAudioFileUnspecifiedError: OSStatus = fourcc(b"wht?") as _;

type AudioFilePermissions = i8;
pub const kAudioFileReadPermission: AudioFilePermissions = 1;

type AudioFileTypeID = u32;
const kAudioFileCAFType: AudioFileTypeID = fourcc(b"caff");

type AudioFilePropertyID = u32;
pub const kAudioFilePropertyDataFormat: AudioFilePropertyID = fourcc(b"dfmt");
const kAudioFilePropertyAudioDataByteCount: AudioFilePropertyID = fourcc(b"bcnt");
const kAudioFilePropertyAudioDataPacketCount: AudioFilePropertyID =
    fourcc(b"pcnt");
pub const kAudioFilePropertyPacketSizeUpperBound: AudioFilePropertyID =
    fourcc(b"pkub");
const kAudioFilePropertyMagicCookieData: AudioFilePropertyID = fourcc(b"mgic");
const kAudioFilePropertyChannelLayout: AudioFilePropertyID = fourcc(b"cmap");
const kAudioFilePropertyEstimatedDuration: AudioFilePropertyID =
    fourcc(b"edur");

const kExtAudioFilePropertyFileDataFormat: u32 = fourcc(b"ffmt");
const kExtAudioFilePropertyClientDataFormat: u32 = fourcc(b"cfmt");
const kExtAudioFilePropertyFileLengthFrames: u32 = fourcc(b"#frm");

fn get_asbd(host_object: &AudioFileHostObject) -> AudioStreamBasicDescription {
    let AudioDescription {
        sample_rate,
        format,
        bytes_per_packet,
        frames_per_packet,
        channels_per_frame,
        bits_per_channel,
    } = host_object.audio_file.audio_description();

    match format {
        audio::AudioFormat::LinearPcm {
            is_float,
            is_little_endian,
        } => {
            let is_packed = (bits_per_channel
                * channels_per_frame
                * frames_per_packet)
                == (bytes_per_packet * 8);
            let format_flags = (u32::from(is_float) * kAudioFormatFlagIsFloat)
                | (u32::from((!is_float) && matches!(bits_per_channel, 16 | 24))
                    * kAudioFormatFlagIsSignedInteger)
                | (u32::from(is_packed) * kAudioFormatFlagIsPacked)
                | (u32::from(!is_little_endian) * kAudioFormatFlagIsBigEndian);
            AudioStreamBasicDescription {
                sample_rate,
                format_id: kAudioFormatLinearPCM,
                format_flags,
                bytes_per_packet,
                frames_per_packet,
                bytes_per_frame: bytes_per_packet / frames_per_packet,
                channels_per_frame,
                bits_per_channel,
                _reserved: 0,
            }
        }
        audio::AudioFormat::AppleIma4 => AudioStreamBasicDescription {
            sample_rate,
            format_id: kAudioFormatAppleIMA4,
            format_flags: 0,
            bytes_per_packet,
            frames_per_packet,
            bytes_per_frame: 0,
            channels_per_frame,
            bits_per_channel,
            _reserved: 0,
        },
    }
}

fn effective_client_asbd(
    host_object: &AudioFileHostObject,
) -> AudioStreamBasicDescription {
    host_object
        .client_data_format
        .unwrap_or_else(|| get_asbd(host_object))
}

pub fn AudioFileOpenURL(
    _env: &mut Environment,
    _in_url: CFURLRef,
    _in_permissions: AudioFilePermissions,
    _in_file_type_hint: AudioFileTypeID,
    _out_audio_file: MutPtr<AudioFileID>,
) -> OSStatus {
    log_dbg!("AudioFileOpenURL stub");
    kAudioFileUnspecifiedError
}

pub fn AudioFileGetProperty(
    _env: &mut Environment,
    _in_audio_file: AudioFileID,
    _in_property_id: AudioFilePropertyID,
    _io_data_size: MutPtr<u32>,
    _out_property_data: MutVoidPtr,
) -> OSStatus {
    log_dbg!(
        "AudioFileGetProperty stub for {}",
        debug_fourcc(_in_property_id)
    );
    0
}

pub fn AudioFileGetPropertyInfo(
    _env: &mut Environment,
    _in_audio_file: AudioFileID,
    _in_property_id: AudioFilePropertyID,
    _out_data_size: MutPtr<u32>,
    _out_writable: MutPtr<u32>,
) -> OSStatus {
    0
}

pub fn AudioFileReadBytes(
    _env: &mut Environment,
    _in_audio_file: AudioFileID,
    _in_use_cache: u32,
    _in_starting_byte: i64,
    _io_num_bytes: MutPtr<u32>,
    _out_buffer: MutVoidPtr,
) -> OSStatus {
    0
}

pub fn AudioFileReadPackets(
    _env: &mut Environment,
    _in_audio_file: AudioFileID,
    _in_use_cache: u32,
    _out_num_bytes: MutPtr<u32>,
    _out_packet_descriptions: MutPtr<u8>,
    _in_starting_packet: i64,
    _io_num_packets: MutPtr<u32>,
    _out_buffer: MutVoidPtr,
) -> OSStatus {
    0
}

pub fn AudioFileReadPacketData(
    _env: &mut Environment,
    _in_audio_file: AudioFileID,
    _in_use_cache: u32,
    _io_num_bytes: MutPtr<u32>,
    _out_packet_descriptions: MutPtr<u8>,
    _in_starting_packet: i64,
    _io_num_packets: MutPtr<u32>,
    _out_buffer: MutVoidPtr,
) -> OSStatus {
    0
}

pub fn ExtAudioFileOpenURL(
    env: &mut Environment,
    in_url: CFURLRef,
    out_ext_audio_file: MutPtr<AudioFileID>,
) -> OSStatus {
    AudioFileOpenURL(
        env,
        in_url,
        kAudioFileReadPermission,
        0,
        out_ext_audio_file,
    )
}

pub fn ExtAudioFileGetProperty(
    env: &mut Environment,
    in_ext_audio_file: AudioFileID,
    in_property_id: u32,
    io_data_size: MutPtr<u32>,
    out_property_data: MutVoidPtr,
) -> OSStatus {
    return_if_null!(in_ext_audio_file);
    let host_object = State::get(&mut env.framework_state)
        .audio_files
        .get_mut(&in_ext_audio_file)
        .unwrap();
    match in_property_id {
        x if x == kExtAudioFilePropertyFileDataFormat => {
            let req_size = guest_size_of::<AudioStreamBasicDescription>() as u32;
            if env.mem.read(io_data_size) != req_size {
                return kAudioFileBadPropertySizeError;
            }
            env.mem
                .write(out_property_data.cast(), get_asbd(host_object));
            0
        }
        x if x == kExtAudioFilePropertyClientDataFormat => {
            let req_size = guest_size_of::<AudioStreamBasicDescription>() as u32;
            if env.mem.read(io_data_size) != req_size {
                return kAudioFileBadPropertySizeError;
            }
            env.mem.write(
                out_property_data.cast(),
                effective_client_asbd(host_object),
            );
            0
        }
        kExtAudioFilePropertyFileLengthFrames => {
            if !io_data_size.is_null() {
                env.mem.write(io_data_size, guest_size_of::<i64>() as u32);
            }
            if !out_property_data.is_null() {
                let desc = host_object.audio_file.audio_description();
                let frames = host_object.audio_file.packet_count() as i64
                    * desc.frames_per_packet as i64;
                env.mem.write(out_property_data.cast(), frames);
            }
            0
        }
        _ => AudioFileGetProperty(
            env,
            in_ext_audio_file,
            in_property_id,
            io_data_size,
            out_property_data,
        ),
    }
}

pub fn ExtAudioFileSetProperty(
    env: &mut Environment,
    in_ext_audio_file: AudioFileID,
    in_property_id: u32,
    in_property_data_size: u32,
    in_property_data: MutVoidPtr,
) -> OSStatus {
    return_if_null!(in_ext_audio_file);
    let host_object = State::get(&mut env.framework_state)
        .audio_files
        .get_mut(&in_ext_audio_file)
        .unwrap();
    match in_property_id {
        x if x == kExtAudioFilePropertyClientDataFormat => {
            let req_size = guest_size_of::<AudioStreamBasicDescription>() as u32;
            if in_property_data_size != req_size {
                return kAudioFileBadPropertySizeError;
            }
            if in_property_data.is_null() {
                return kAudioFileUnspecifiedError;
            }
            let asbd: AudioStreamBasicDescription =
                env.mem.read(in_property_data.cast());
            host_object.client_data_format = Some(asbd);
            0
        }
        _ => {
            log_dbg!(
                "ExtAudioFileSetProperty stub for {}",
                debug_fourcc(in_property_id)
            );
            0
        }
    }
}

pub fn ExtAudioFileRead(
    env: &mut Environment,
    in_ext_audio_file: AudioFileID,
    io_number_frames: MutPtr<u32>,
    io_data: MutPtr<AudioBufferList<1>>,
) -> OSStatus {
    return_if_null!(in_ext_audio_file);
    if io_number_frames.is_null() || io_data.is_null() {
        return kAudioFileUnspecifiedError;
    }
    let frames_requested = env.mem.read(io_number_frames);
    if frames_requested == 0 {
        return 0;
    }
    let host_object = State::get(&mut env.framework_state)
        .audio_files
        .get_mut(&in_ext_audio_file)
        .unwrap();
    let asbd = effective_client_asbd(host_object);
    let bytes_per_frame = asbd.bytes_per_frame;
    if bytes_per_frame == 0 {
        return kAudioFileUnspecifiedError;
    }
    let mut buffer_list: AudioBufferList<1> = env.mem.read(io_data);
    if buffer_list.number_buffers == 0 {
        env.mem.write(io_number_frames, 0);
        return 0;
    }
    let first_buffer = &mut buffer_list.buffers[0];
    let max_bytes = first_buffer
        .data_byte_size
        .min(frames_requested.saturating_mul(bytes_per_frame));
    if max_bytes == 0 {
        env.mem.write(io_number_frames, 0);
        return 0;
    }
    let start = host_object.read_pos_bytes;
    let out_slice = env.mem.bytes_at_mut(first_buffer.data.cast(), max_bytes);
    let read = match host_object.audio_file.read_bytes(start, out_slice) {
        Ok(n) => n,
        Err(_) => return kAudioFileUnspecifiedError,
    };
    host_object.read_pos_bytes += read as u64;
    first_buffer.data_byte_size = read as u32;
    env.mem.write(io_data, buffer_list);
    env.mem.write(io_number_frames, (read as u32) / bytes_per_frame);
    if read == 0 {
        eofErr
    } else {
        0
    }
}

pub fn AudioFileOpenWithCallbacks(
    env: &mut Environment,
    client_data: MutVoidPtr,
    read_callback: GuestFunction,
    _write_callback: GuestFunction,
    getsize_callback: GuestFunction,
    _setsize_callback: GuestFunction,
    _in_file_type_hint: AudioFileTypeID,
    out_audio_file: MutPtr<AudioFileID>,
) -> OSStatus {
    let size: i64 = getsize_callback.call_from_host(env, (client_data,));
    let size_u32 = size as u32;
    let data_ptr: MutPtr<u8> = env.mem.alloc(size_u32).cast();
    let bytes_read_ptr: MutPtr<u32> = env.mem.alloc_and_write(0u32).cast();
    let status: OSStatus = read_callback.call_from_host(
        env,
        (client_data, 0_i64, size_u32, data_ptr, bytes_read_ptr),
    );
    if status != 0 {
        return status;
    }
    let actual_size = env.mem.read(bytes_read_ptr);
    let data_vec = env.mem.bytes_at(data_ptr, actual_size).to_vec();
    let audio_file = match audio::AudioFile::read_from_vec(data_vec) {
        Ok(af) => af,
        Err(_) => return kAudioFileUnsupportedFileTypeError,
    };
    let guest_af = env.mem.alloc_and_write(OpaqueAudioFileID { _filler: 0 });
    State::get(&mut env.framework_state).audio_files.insert(
        guest_af,
        AudioFileHostObject {
            audio_file,
            client_data_format: None,
            read_pos_bytes: 0,
        },
    );
    env.mem.write(out_audio_file, guest_af);
    0
}

pub fn AudioFileClose(env: &mut Environment, in_af: AudioFileID) -> OSStatus {
    if State::get(&mut env.framework_state)
        .audio_files
        .remove(&in_af)
        .is_some()
    {
        env.mem.free(in_af.cast());
        0
    } else {
        kAudioFileUnspecifiedError
    }
}

pub fn ExtAudioFileDispose(
    env: &mut Environment,
    in_ext_audio_file: AudioFileID,
) -> OSStatus {
    AudioFileClose(env, in_ext_audio_file)
}

fn AudioFileStreamOpen(
    _: &mut Environment,
    _: MutVoidPtr,
    _: MutVoidPtr,
    _: MutVoidPtr,
    _: u32,
    _: MutVoidPtr,
) -> OSStatus {
    kAudioFileUnspecifiedError
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioFileOpenURL(_, _, _, _)),
    export_c_func!(ExtAudioFileOpenURL(_, _)),
    export_c_func!(ExtAudioFileGetProperty(_, _, _, _)),
    export_c_func!(ExtAudioFileSetProperty(_, _, _, _)),
    export_c_func!(ExtAudioFileRead(_, _, _)),
    export_c_func!(ExtAudioFileDispose(_)),
    export_c_func!(AudioFileGetPropertyInfo(_, _, _, _)),
    export_c_func!(AudioFileGetProperty(_, _, _, _)),
    export_c_func!(AudioFileReadBytes(_, _, _, _, _)),
    export_c_func!(AudioFileReadPackets(_, _, _, _, _, _, _)),
    export_c_func!(AudioFileReadPacketData(_, _, _, _, _, _, _)),
    export_c_func!(AudioFileOpenWithCallbacks(_, _, _, _, _, _, _)),
    export_c_func!(AudioFileClose(_)),
    export_c_func!(AudioFileStreamOpen(_, _, _, _, _)),
];

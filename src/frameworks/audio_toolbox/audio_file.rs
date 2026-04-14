/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioFile.h` (Audio File Services)

use crate::abi::{CallFromHost, GuestFunction};
use crate::audio; // Keep this module namespaced to avoid confusion
use crate::audio::AudioDescription;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::{eofErr, OSStatus};
use crate::frameworks::core_audio_types::{
    debug_fourcc, fourcc, kAudioFormatAppleIMA4, kAudioFormatFlagIsBigEndian,
    kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked, kAudioFormatFlagIsSignedInteger,
    kAudioFormatLinearPCM, AudioStreamBasicDescription,
};
use crate::frameworks::core_foundation::cf_url::CFURLRef;
use crate::frameworks::foundation::ns_url::to_rust_path;
use crate::mem::{guest_size_of, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead};
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
}

#[repr(C, packed)]
pub struct OpaqueAudioFileID {
    _filler: u8,
}
unsafe impl SafeRead for OpaqueAudioFileID {}

pub type AudioFileID = MutPtr<OpaqueAudioFileID>;

#[allow(dead_code)]
const kAudioFileFileNotFoundError: OSStatus = -43;
const kAudioFileNotOpenError: OSStatus = -38;
const kAudioFileSuccess: OSStatus = 1;

const kAudioFileBadPropertySizeError: OSStatus = fourcc(b"!siz") as _;
const kAudioFileUnsupportedProperty: OSStatus = fourcc(b"pty?") as _;

const kAudioFileUnsupportedFileTypeError: OSStatus = fourcc(b"typ?") as _;
const kAudioFileUnspecifiedError: OSStatus = fourcc(b"wht?") as _;

type AudioFilePermissions = i8;

pub const kAudioFileReadPermission: AudioFilePermissions = 1;

/// Usually a FourCC.
type AudioFileTypeID = u32;
const kAudioFileCAFType: AudioFileTypeID = fourcc(b"caff");

/// Usually a FourCC.
type AudioFilePropertyID = u32;
pub const kAudioFilePropertyDataFormat: AudioFilePropertyID = fourcc(b"dfmt");
const kAudioFilePropertyAudioDataByteCount: AudioFilePropertyID = fourcc(b"bcnt");

const kAudioFilePropertyAudioDataPacketCount: AudioFilePropertyID = fourcc(b"pcnt");
pub const kAudioFilePropertyPacketSizeUpperBound: AudioFilePropertyID = fourcc(b"pkub");
const kAudioFilePropertyMagicCookieData: AudioFilePropertyID = fourcc(b"mgic");

const kAudioFilePropertyChannelLayout: AudioFilePropertyID = fourcc(b"cmap");
const kAudioFilePropertyEstimatedDuration: AudioFilePropertyID = fourcc(b"edur");
const kAudioFileProperty_PacketTable: AudioFilePropertyID = fourcc(b"pnfo");
const kAudioFilePropertyPacketToFrame: AudioFilePropertyID = fourcc(b"flst");

pub fn AudioFileOpenURL(
    env: &mut Environment,
    in_file_ref: CFURLRef,
    in_permissions: AudioFilePermissions,
    in_file_type_hint: AudioFileTypeID,
    out_audio_file: MutPtr<AudioFileID>,
) -> OSStatus {
    return_if_null!(in_file_ref);

    if !out_audio_file.is_null() {
        env.mem.write(out_audio_file, Ptr::null());
    }

    assert!(in_permissions == kAudioFileReadPermission);

    match in_file_type_hint {
        0 | kAudioFileCAFType => {}
        _ => unimplemented!(),
    }

    let path = to_rust_path(env, in_file_ref);

    let audio_file = match audio::AudioFile::open_for_reading(path, &env.fs) {
        Ok(audio_file) => audio_file,
        Err(_) => {
            // ЧЕРНАЯ ДЫРА: Если файл не найден или сломан, мы не выдаем ошибку.
            // Мы оставляем ID = NULL и возвращаем 0 (УСПЕХ). Игра думает, что всё отлично!
            return 0; 
        }
    };

    let host_object = AudioFileHostObject { audio_file };
    let guest_audio_file = env.mem.alloc_and_write(OpaqueAudioFileID { _filler: 0 });

    State::get(&mut env.framework_state)
        .audio_files
        .insert(guest_audio_file, host_object);

    env.mem.write(out_audio_file, guest_audio_file);

    0 // success
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
    if !out_audio_file.is_null() {
        env.mem.write(out_audio_file, Ptr::null());
    }

    let size: i64 = getsize_callback.call_from_host(env, (client_data,));
    let size: u32 = size.try_into().unwrap();

    if size == 0 { return 0; } // ЧЕРНАЯ ДЫРА

    let data_ptr: MutPtr<u8> = env.mem.alloc(size).cast();
    let bytes_read_ptr: MutPtr<u32> = env.mem.alloc(guest_size_of::<u32>()).cast();

    env.mem.write(bytes_read_ptr, 0);

    let status: OSStatus = read_callback.call_from_host(env, (client_data, 0_i64, size, data_ptr, bytes_read_ptr));

    if status != 0 { return 0; } // ЧЕРНАЯ ДЫРА

    let data_vec = env.mem.bytes_at(data_ptr, env.mem.read(bytes_read_ptr)).to_vec();

    let Ok(audio_file) = audio::AudioFile::read_from_vec(data_vec) else {
        return 0; // ЧЕРНАЯ ДЫРА
    };

    let guest_audio_file = env.mem.alloc_and_write(OpaqueAudioFileID { _filler: 0 });
    let host_object = AudioFileHostObject { audio_file };

    State::get(&mut env.framework_state)
        .audio_files
        .insert(guest_audio_file, host_object);

    env.mem.write(out_audio_file, guest_audio_file);
    0 // success
}

fn property_size(property_id: AudioFilePropertyID) -> GuestUSize {
    match property_id {
        kAudioFilePropertyDataFormat => guest_size_of::<AudioStreamBasicDescription>(),
        kAudioFilePropertyAudioDataByteCount => guest_size_of::<u64>(),
        kAudioFilePropertyAudioDataPacketCount => guest_size_of::<u64>(),
        kAudioFilePropertyPacketSizeUpperBound => guest_size_of::<u32>(),
        kAudioFilePropertyEstimatedDuration => guest_size_of::<f64>(),
        kAudioFileProperty_PacketTable => guest_size_of::<f64>(),
        kAudioFilePropertyPacketToFrame => guest_size_of::<f64>(),
        _ => 8, // Безопасный фоллбэк вместо паники
     }
}

fn AudioFileGetPropertyInfo(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    in_property_id: AudioFilePropertyID,
    out_data_size: MutPtr<u32>,
    is_writable: MutPtr<u32>,
) -> OSStatus {
    // ЧЕРНАЯ ДЫРА: Обработка фейковых NULL-файлов
    if in_audio_file.is_null() || State::get(&mut env.framework_state).audio_files.get(&in_audio_file).is_none() {
        if !out_data_size.is_null() { env.mem.write(out_data_size, property_size(in_property_id)); }
        if !is_writable.is_null() { env.mem.write(is_writable, 0); }
        return 0; // SUCCESS!
    }

    if in_property_id == kAudioFilePropertyMagicCookieData || in_property_id == kAudioFilePropertyChannelLayout {
        if !out_data_size.is_null() { env.mem.write(out_data_size, 0); }
        if !is_writable.is_null() { env.mem.write(is_writable, 0); }
        return kAudioFileUnsupportedProperty;
    }
    
    if !out_data_size.is_null() { env.mem.write(out_data_size, property_size(in_property_id)); }
    if !is_writable.is_null() { env.mem.write(is_writable, 0); }
    0 // success
}

pub fn AudioFileGetProperty(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    in_property_id: AudioFilePropertyID,
    io_data_size: MutPtr<u32>,
    out_property_data: MutVoidPtr,
) -> OSStatus {
    // ЧЕРНАЯ ДЫРА: Кормим игру нулями для сломанных файлов
    if in_audio_file.is_null() || State::get(&mut env.framework_state).audio_files.get(&in_audio_file).is_none() {
        if !io_data_size.is_null() {
            let size = env.mem.read(io_data_size);
            if size > 0 && !out_property_data.is_null() {
                env.mem.bytes_at_mut(out_property_data.cast(), size).fill(0);
            }
        }
        return 0; // SUCCESS!
    }

    let required_size = property_size(in_property_id);
    if env.mem.read(io_data_size) != required_size {
        return kAudioFileBadPropertySizeError;
    }

    let host_object = State::get(&mut env.framework_state).audio_files.get_mut(&in_audio_file).unwrap();

    match in_property_id {
        kAudioFilePropertyDataFormat => {
            let audio::AudioDescription { sample_rate, format, bytes_per_packet, frames_per_packet, channels_per_frame, bits_per_channel } = host_object.audio_file.audio_description();
            let desc: AudioStreamBasicDescription = match format {
                audio::AudioFormat::LinearPcm { is_float, is_little_endian } => {
                 let is_packed = (bits_per_channel * channels_per_frame * frames_per_packet) == (bytes_per_packet * 8);
                 let format_flags = (u32::from(is_float) * kAudioFormatFlagIsFloat)
                        | (u32::from((!is_float) && matches!(bits_per_channel, 16 | 24)) * kAudioFormatFlagIsSignedInteger)
                        | (u32::from(is_packed) * kAudioFormatFlagIsPacked)
                        | (u32::from(!is_little_endian) * kAudioFormatFlagIsBigEndian);
                    AudioStreamBasicDescription { sample_rate, format_id: kAudioFormatLinearPCM, format_flags, bytes_per_packet, frames_per_packet, bytes_per_frame: bytes_per_packet / frames_per_packet, channels_per_frame, bits_per_channel, _reserved: 0 }
                }
                audio::AudioFormat::AppleIma4 => {
                    AudioStreamBasicDescription { sample_rate, format_id: kAudioFormatAppleIMA4, format_flags: 0, bytes_per_packet, frames_per_packet, bytes_per_frame: 0, channels_per_frame, bits_per_channel, _reserved: 0 }
                }
                audio::AudioFormat::Mpeg4Aac => {
                     AudioStreamBasicDescription { sample_rate, format_id: fourcc(b"aac "), format_flags: 0, bytes_per_packet, frames_per_packet, bytes_per_frame: 0, channels_per_frame, bits_per_channel, _reserved: 0 }
                }
            };
            env.mem.write(out_property_data.cast(), desc);
        }
        kAudioFilePropertyAudioDataByteCount => { env.mem.write(out_property_data.cast(), host_object.audio_file.byte_count()); }
        kAudioFilePropertyAudioDataPacketCount => { env.mem.write(out_property_data.cast(), host_object.audio_file.packet_count()); }
        kAudioFilePropertyPacketSizeUpperBound => { env.mem.write(out_property_data.cast(), host_object.audio_file.packet_size_upper_bound()); }
        kAudioFilePropertyEstimatedDuration | kAudioFileProperty_PacketTable | kAudioFilePropertyPacketToFrame => {
            let AudioDescription { sample_rate, bytes_per_packet, frames_per_packet, .. } = host_object.audio_file.audio_description();
            let estimated_duration: f64 = host_object.audio_file.byte_count() as f64 * frames_per_packet as f64 / (bytes_per_packet as f64 * sample_rate);
            env.mem.write(out_property_data.cast(), estimated_duration);
        }
        _ => {}
    }
    0 // success
}

fn AudioFileReadBytes(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    _in_use_cache: bool,
    in_starting_byte: i64,
    io_num_bytes: MutPtr<u32>,
    out_buffer: MutVoidPtr,
) -> OSStatus {
    // ЧЕРНАЯ ДЫРА
    if in_audio_file.is_null() || State::get(&mut env.framework_state).audio_files.get(&in_audio_file).is_none() {
        if !io_num_bytes.is_null() { env.mem.write(io_num_bytes, 0); }
        return 0; // SUCCESS (read 0 bytes)
    }

    let host_object = State::get(&mut env.framework_state).audio_files.get_mut(&in_audio_file).unwrap();
    let bytes_to_read = env.mem.read(io_num_bytes);
    let buffer_slice = env.mem.bytes_at_mut(out_buffer.cast(), bytes_to_read);

    let bytes_read = host_object.audio_file.read_bytes(in_starting_byte.try_into().unwrap(), buffer_slice).unwrap();
    env.mem.write(io_num_bytes, bytes_read.try_into().unwrap());

    if bytes_read < bytes_to_read as usize { eofErr } else { 0 }
}

fn AudioFileReadPacketData(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    in_use_cache: bool,
    out_num_bytes: MutPtr<u32>,
    out_packet_descriptions: MutVoidPtr,
    in_starting_packet: i64,
    io_num_packets: MutPtr<u32>,
    out_buffer: MutVoidPtr,
) -> OSStatus {
    AudioFileReadPackets(env, in_audio_file, in_use_cache, out_num_bytes, out_packet_descriptions, in_starting_packet, io_num_packets, out_buffer)
}

pub fn AudioFileReadPackets(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    in_use_cache: bool,
    out_num_bytes: MutPtr<u32>,
    _out_packet_descriptions: MutVoidPtr,
    in_starting_packet: i64,
    io_num_packets: MutPtr<u32>,
    out_buffer: MutVoidPtr,
) -> OSStatus {
    // ЧЕРНАЯ ДЫРА: Симулируем идеальный, но пустой файл. Игра сразу пойдет дальше.
    if in_audio_file.is_null() || State::get(&mut env.framework_state).audio_files.get(&in_audio_file).is_none() {
        if !io_num_packets.is_null() { env.mem.write(io_num_packets, 0); }
        if !out_num_bytes.is_null() { env.mem.write(out_num_bytes, 0); }
        return 0; // SUCCESS! 
    }

    let host_object = State::get(&mut env.framework_state).audio_files.get_mut(&in_audio_file).unwrap();
    let packet_size = host_object.audio_file.packet_size_fixed();
    let packets_to_read = env.mem.read(io_num_packets);

    if packet_size == 0 || packets_to_read == 0 {
        env.mem.write(io_num_packets, 0);
        if !out_num_bytes.is_null() { env.mem.write(out_num_bytes, 0); }
        return kAudioFileSuccess;
    }

    let starting_byte = match i64::from(packet_size).checked_mul(in_starting_packet) {
        Some(v) => v, None => return kAudioFileBadPropertySizeError,
    };

    let bytes_to_read = match packets_to_read.checked_mul(packet_size) {
        Some(v) => v, None => return kAudioFileBadPropertySizeError,
    };

    if !out_num_bytes.is_null() { env.mem.write(out_num_bytes, bytes_to_read); }

    let res = AudioFileReadBytes(env, in_audio_file, in_use_cache, starting_byte, out_num_bytes, out_buffer);

    let bytes_read = if !out_num_bytes.is_null() { env.mem.read(out_num_bytes) } else { 0 };
    let packets_read = bytes_read / packet_size;
    env.mem.write(io_num_packets, packets_read);

    res
}

pub fn AudioFileClose(env: &mut Environment, in_audio_file: AudioFileID) -> OSStatus {
    // ЧЕРНАЯ ДЫРА
    if in_audio_file.is_null() || State::get(&mut env.framework_state).audio_files.get(&in_audio_file).is_none() {
        return 0; // SUCCESS!
    }

    State::get(&mut env.framework_state).audio_files.remove(&in_audio_file);
    env.mem.free(in_audio_file.cast());
    0 
}

fn AudioFileStreamOpen(
    _env: &mut Environment, _in_client_data: MutVoidPtr, _in_property_listener_proc: MutVoidPtr,
    _in_packets_proc: MutVoidPtr, _in_file_type_hint: AudioFileTypeID, _out_audio_file_stream: MutVoidPtr,
) -> OSStatus {
    kAudioFileUnspecifiedError
}

pub fn AudioFormatGetPropertyInfo(
    _env: &mut Environment, _property_id: AudioFilePropertyID, _specifier_size: u32,
    _specifier: crate::mem::ConstPtr<u8>, _out_property_data_size: MutPtr<u32>,
) -> OSStatus {
    -50
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioFileOpenURL(_, _, _, _)),
    export_c_func!(AudioFileGetPropertyInfo(_, _, _, _)),
    export_c_func!(AudioFileGetProperty(_, _, _, _)),
    export_c_func!(AudioFileReadBytes(_, _, _, _, _)),
    export_c_func!(AudioFileReadPackets(_, _, _, _, _, _, _)),
    export_c_func!(AudioFileReadPacketData(_, _, _, _, _, _, _)),
    export_c_func!(AudioFileOpenWithCallbacks(_, _, _, _, _, _, _)),
    export_c_func!(AudioFileClose(_)),
    export_c_func!(AudioFileStreamOpen(_, _, _, _, _)),
    export_c_func!(AudioFormatGetPropertyInfo(_, _, _, _)),
];


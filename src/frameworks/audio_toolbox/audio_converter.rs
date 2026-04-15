/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `AudioConverter.h` — Audio format conversion.
//!
//! PvZ 1.1 calls AudioConverterNew() when setting up its audio pipeline.
//! We provide a skeleton implementation that hooks into the game's callback
//! via AudioConverterFillComplexBuffer to prepare for Symphonia decoding.

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::{AudioStreamBasicDescription, debug_fourcc};
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr, SafeRead};
use crate::Environment;

// OSStatus error codes
const kAudioConverterErr_InvalidInputSize: OSStatus = -50;

// CoreAudio types needed for buffer filling
#[repr(C, packed)]
pub struct AudioBuffer {
    pub mNumberChannels: u32,
    pub mDataByteSize: u32,
    pub mData: MutVoidPtr,
}
unsafe impl SafeRead for AudioBuffer {}

#[repr(C, packed)]
pub struct AudioBufferList {
    pub mNumberBuffers: u32,
    pub mBuffers: [AudioBuffer; 1], // In reality, this is a variable-length array
}
unsafe impl SafeRead for AudioBufferList {}

#[repr(C, packed)]
pub struct AudioStreamPacketDescription {
    pub mStartOffset: i64,
    pub mVariableFramesInPacket: u32,
    pub mDataByteSize: u32,
}
unsafe impl SafeRead for AudioStreamPacketDescription {}

/// Callback type used by the game to feed compressed data to our converter
pub type AudioConverterComplexInputDataProc = GuestFunction;

// Обновленная структура, которая теперь хранит форматы
#[repr(C, packed)]
struct OpaqueAudioConverter {
    source_format: AudioStreamBasicDescription,
    dest_format: AudioStreamBasicDescription,
    // В будущем здесь будет указатель на состояние декодера Symphonia
    decoder_state_ptr: MutVoidPtr, 
}
unsafe impl SafeRead for OpaqueAudioConverter {}

type AudioConverterRef = MutPtr<OpaqueAudioConverter>;

fn AudioConverterNew(
    env: &mut Environment,
    in_source_format: ConstPtr<AudioStreamBasicDescription>,
    in_destination_format: ConstPtr<AudioStreamBasicDescription>,
    out_audio_converter: MutPtr<AudioConverterRef>,
) -> OSStatus {
    let source_format = env.mem.read(in_source_format);
    let dest_format = env.mem.read(in_destination_format);

    log!(
        "AudioConverterNew called! Converting from {} to {}",
        debug_fourcc(source_format.format_id),
        debug_fourcc(dest_format.format_id)
    );

    let converter_data = OpaqueAudioConverter {
        source_format,
        dest_format,
        decoder_state_ptr: MutPtr::null(),
    };

    let converter: AudioConverterRef = env.mem.alloc_and_write(converter_data);
    env.mem.write(out_audio_converter, converter);

    0 // noErr
}

fn AudioConverterDispose(env: &mut Environment, in_audio_converter: AudioConverterRef) -> OSStatus {
    if in_audio_converter.is_null() {
        log!("AudioConverterDispose: null converter, returning error");
        return kAudioConverterErr_InvalidInputSize;
    }
    
    // В будущем: здесь нужно будет освобождать память Box/Symphonia по указателю decoder_state_ptr

    env.mem.free(in_audio_converter.cast());
    log_dbg!("AudioConverterDispose({:?}) -> 0", in_audio_converter);
    0
}

fn AudioConverterReset(_env: &mut Environment, in_audio_converter: AudioConverterRef) -> OSStatus {
    log_dbg!("AudioConverterReset({:?}) -> 0 (stubbed)", in_audio_converter);
    0
}

fn AudioConverterGetProperty(
    _env: &mut Environment,
    in_audio_converter: AudioConverterRef,
    in_property_id: u32,
    _io_data_size: MutPtr<u32>,
    _out_property_data: MutPtr<u8>,
) -> OSStatus {
    log!(
        "TODO: AudioConverterGetProperty({:?}, {}) -> unimplemented (-1)",
        in_audio_converter,
        debug_fourcc(in_property_id),
    );
    -1
}

fn AudioConverterSetProperty(
    _env: &mut Environment,
    in_audio_converter: AudioConverterRef,
    in_property_id: u32,
    in_data_size: u32,
    _in_property_data: ConstPtr<u8>,
) -> OSStatus {
    log!(
        "TODO: AudioConverterSetProperty({:?}, {}, size={}) -> 0 (stubbed)",
        in_audio_converter,
        debug_fourcc(in_property_id),
        in_data_size,
    );
    0
}

/// Эта функция вызывается игрой, когда ей нужен декодированный звук.
fn AudioConverterFillComplexBuffer(
    env: &mut Environment,
    in_audio_converter: AudioConverterRef,
    in_input_data_proc: AudioConverterComplexInputDataProc,
    in_input_data_proc_user_data: MutVoidPtr,
    io_output_data_packet_size: MutPtr<u32>,
    out_output_data: MutPtr<AudioBufferList>,
    out_packet_description: MutPtr<MutPtr<AudioStreamPacketDescription>>,
) -> OSStatus {
    if in_audio_converter.is_null() {
        return kAudioConverterErr_InvalidInputSize;
    }
    
    let requested_packets = env.mem.read(io_output_data_packet_size);
    log_dbg!(
        "AudioConverterFillComplexBuffer requested {} packets. Converter: {:?}",
        requested_packets,
        in_audio_converter
    );

    // Выделяем временную память в госте для аргументов коллбэка
    let io_number_data_packets_ptr: MutPtr<u32> = env.mem.alloc_and_write(requested_packets);
    let io_data_ptr: MutPtr<AudioBufferList> = env.mem.alloc_and_write(AudioBufferList {
        mNumberBuffers: 0,
        mBuffers: [AudioBuffer { mNumberChannels: 0, mDataByteSize: 0, mData: MutPtr::null() }],
    });
    let out_data_packet_description_ptr = env.mem.alloc_and_write::<MutPtr<AudioStreamPacketDescription>>(MutPtr::null());

    // Делаем вызов обратно в игру, чтобы она отдала нам порцию сжатых аудиоданных
    let callback_status: OSStatus = in_input_data_proc.call_from_host(
        env,
        (
            in_audio_converter,
            io_number_data_packets_ptr,
            io_data_ptr,
            out_data_packet_description_ptr,
            in_input_data_proc_user_data,
        ),
    );

    // Читаем, сколько пакетов нам реально вернула игра
    let provided_packets = env.mem.read(io_number_data_packets_ptr);
    log_dbg!("Game callback returned status: {}, provided packets: {}", callback_status, provided_packets);

    // Освобождаем временную память
    env.mem.free(io_number_data_packets_ptr.cast());
    env.mem.free(io_data_ptr.cast());
    env.mem.free(out_data_packet_description_ptr.cast());

    // Здесь должен быть код инициализации Symphonia (MediaSource), передача
    // полученных от игры байтов в декодер и запись распакованного PCM в out_output_data.
    
    // Пока мы просто возвращаем 0 пакетов, чтобы игра не зависла
    env.mem.write(io_output_data_packet_size, 0);

    0 // Успешно возвращаем управление
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioConverterNew(_, _, _)),
    export_c_func!(AudioConverterDispose(_)),
    export_c_func!(AudioConverterReset(_)),
    export_c_func!(AudioConverterGetProperty(_, _, _, _)),
    export_c_func!(AudioConverterSetProperty(_, _, _, _)),
    export_c_func!(AudioConverterFillComplexBuffer(_, _, _, _, _, _)),
];


/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! AVAudioPlayer
//!
//! Implemented using Audio Queue Services based on [the PlayingAudio example](https://developer.apple.com/library/archive/documentation/MusicAudio/Conceptual/AudioQueueProgrammingGuide/AQPlayback/PlayingAudio.html)

use crate::dyld::HostFunction;
use crate::frameworks::audio_toolbox::audio_file::{
    self, kAudioFilePropertyDataFormat, kAudioFilePropertyPacketSizeUpperBound,
    kAudioFileReadPermission, AudioFileClose, AudioFileGetProperty, AudioFileID, AudioFileOpenURL,
    AudioFileReadPackets,
};
use crate::frameworks::audio_toolbox::audio_queue::{
    kAudioQueueParam_Volume, AudioQueueAllocateBuffer, AudioQueueBufferRef, AudioQueueDispose,
    AudioQueueEnqueueBuffer, AudioQueueGetParameter, AudioQueueNewOutput, AudioQueueOutputCallback,
    AudioQueuePause, AudioQueueRef, AudioQueueSetParameter, AudioQueueStart, AudioQueueStop,
};
use crate::frameworks::carbon_core::eofErr;
use crate::frameworks::core_audio_types::AudioStreamBasicDescription;
use crate::frameworks::core_foundation::cf_run_loop::kCFRunLoopCommonModes;
use crate::frameworks::foundation::ns_error::NSOSStatusErrorDomain;
use crate::frameworks::foundation::{ns_string, NSInteger, NSTimeInterval};
use crate::mem::{guest_size_of, GuestUSize, MutPtr, MutVoidPtr, Ptr, ConstVoidPtr};
use crate::objc::{
    id, msg, msg_class, nil, release, retain, todo_objc_setter, Class, ClassExports, HostObject,
    NSZonePtr,
};
use crate::objc_classes;
use crate::Environment;

// Подключаем Symphonia для прямой распаковки музыки в память
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::core::audio::SampleBuffer;

const kNumberBuffers: usize = 3;

struct AVAudioPlayerHostObject {
    audio_file_url: id,
    output_callback: AudioQueueOutputCallback,
    audio_file_id: Option<AudioFileID>,
    audio_desc: Option<AudioStreamBasicDescription>,
    audio_queue: Option<AudioQueueRef>,
    audio_queue_buffers: Option<MutPtr<AudioQueueBufferRef>>,
    num_packets_to_read: u32,
    current_packet: i64, // При использовании PCM это будет смещение в байтах
    set_current_time: NSTimeInterval,
    volume: f32,
    is_playing: bool,
    num_of_loops: NSInteger,
    
    // ДОБАВЛЕНО: Прямой PCM-буфер для хранения раскодированного звука
    pcm_data: Option<Vec<u8>>,
}
impl HostObject for AVAudioPlayerHostObject {}

// Вспомогательная функция для быстрой конвертации любых файлов в PCM через Symphonia
fn decode_to_pcm(data_slice: &[u8]) -> Option<(Vec<u8>, f64, u32)> {
    let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(data_slice.to_vec())), Default::default());
    let hint = Hint::new();
    
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .ok()?;
        
    let mut format_reader = probed.format;
    let track = format_reader.default_track()?;
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())
        .ok()?;

    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100) as f64;
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2) as u32;

    let mut pcm_out = Vec::new();
    let mut sample_buf = None;

    loop {
        let packet = match format_reader.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(_) => break,
        };

        if packet.track_id() != track_id { continue; }

        if let Ok(audio_buf) = decoder.decode(&packet) {
            if sample_buf.is_none() {
                let spec = *audio_buf.spec();
                let duration = audio_buf.capacity() as u64;
                sample_buf = Some(SampleBuffer::<i16>::new(duration, spec));
            }

            if let Some(buf) = &mut sample_buf {
                buf.copy_interleaved_ref(audio_buf);
                for &sample in buf.samples() {
                    pcm_out.extend_from_slice(&sample.to_le_bytes());
                }
            }
        }
    }
    Some((pcm_out, sample_rate, channels))
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation AVAudioPlayer: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let symb = "__touchHLE_AVAudioPlayerOutputBufferHelper";
    let hf: HostFunction = &(_touchHLE_AVAudioPlayerOutputBufferHelper as fn(&mut Environment, _, _, _) -> _);
    let callback = env.dyld.create_guest_function(&mut env.mem, symb, hf);
    
    let host_object = Box::new(AVAudioPlayerHostObject {
        audio_file_url: nil,
        output_callback: callback,
        audio_file_id: None,
        audio_desc: None,
        audio_queue: None,
        audio_queue_buffers: None,
        num_packets_to_read: 0,
        current_packet: 0,
        set_current_time: 0.0,
        volume: 1.0,
        is_playing: false,
        num_of_loops: 0,
        pcm_data: None, // Инициализация
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithContentsOfURL:(id)url error:(MutPtr<id>)outError {
    let path: id = msg![env; url path];
    let path_str = ns_string::to_rust_string(env, path);
    log_dbg!("[(AVAudioPlayer*){:?} initWithContentsOfURL:{:?} {} outError:{:?}]", this, url, path_str, outError);

    retain(env, url);
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_file_url = url;
    
    let tmp_afi_ptr: MutPtr<AudioFileID> = env.mem.alloc(guest_size_of::<AudioFileID>()).cast();
    let status = AudioFileOpenURL(env, url, kAudioFileReadPermission, 0, tmp_afi_ptr) as NSInteger;
    let audio_file_id = env.mem.read(tmp_afi_ptr);
    env.mem.free(tmp_afi_ptr.cast());
    
    if status != 0 {
        // Фоллбек: Если стандартный AudioFile не смог прочитать (например, это MP3), 
        // читаем файл напрямую и декодируем через Symphonia!
        log_dbg!("AudioFileOpenURL failed, falling back to Symphonia for {}", path_str);
        
        // ИСПРАВЛЕНИЕ E0277: Добавлено .as_ref()
        if let Ok(file_data) = std::fs::read(path_str.as_ref()) {
            if let Some((pcm_out, sample_rate, channels)) = decode_to_pcm(&file_data) {
                let audio_desc = AudioStreamBasicDescription {
                    sample_rate,
                    format_id: crate::frameworks::core_audio_types::kAudioFormatLinearPCM,
                    format_flags: 12,
                    bytes_per_packet: channels * 2,
                    frames_per_packet: 1,
                    bytes_per_frame: channels * 2,
                    channels_per_frame: channels,
                    bits_per_channel: 16,
                    _reserved: 0,
                };
                let mut host_obj = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
                host_obj.pcm_data = Some(pcm_out);
                host_obj.audio_desc = Some(audio_desc);
                return this; 
            }
        }

        // Если все сломалось - возвращаем ошибку
        if !outError.is_null() {
            let domain = ns_string::get_static_str(env, NSOSStatusErrorDomain);
            let error = msg_class![env; NSError alloc];
            let error_code: NSInteger = status; // Возвращаем реальный статус ошибки
            let error = msg![env; error initWithDomain:domain code:error_code userInfo:nil];
            env.mem.write(outError, error);
        }
        return nil;
    }

    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_file_id = Some(audio_file_id);
    this
}

- (id)initWithData:(id)data error:(MutPtr<id>)outError {
    log_dbg!("[(AVAudioPlayer*){:?} initWithData:{:?} outError:{:?}]", this, data, outError);
    
    let length: GuestUSize = msg![env; data length];
    let bytes: ConstVoidPtr = msg![env; data bytes];
    let data_slice = env.mem.bytes_at(bytes.cast(), length);
    
    // Распаковываем сырые данные (MP3/AAC) в PCM прямо из памяти
    if let Some((pcm_out, sample_rate, channels)) = decode_to_pcm(data_slice) {
        let audio_desc = AudioStreamBasicDescription {
            sample_rate,
            format_id: crate::frameworks::core_audio_types::kAudioFormatLinearPCM,
            format_flags: 12,
            bytes_per_packet: channels * 2,
            frames_per_packet: 1,
            bytes_per_frame: channels * 2,
            channels_per_frame: channels,
            bits_per_channel: 16,
            _reserved: 0,
        };
        let mut host_obj = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
        host_obj.pcm_data = Some(pcm_out);
        host_obj.audio_desc = Some(audio_desc);
        return this;
    }

    if !outError.is_null() {
        let domain = ns_string::get_static_str(env, NSOSStatusErrorDomain);
        let error = msg_class![env; NSError alloc];
        let error_code: NSInteger = -1; // Возвращаем универсальную ошибку
        let error = msg![env; error initWithDomain:domain code:error_code userInfo:nil];
        env.mem.write(outError, error);
    }
    nil
}

- (())setDelegate:(id)delegate {
    todo_objc_setter!(this, delegate);
}

- (())setMeteringEnabled:(bool)enabled {
    todo_objc_setter!(this, enabled);
}

- (f32)volume {
    let aq_ref = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue;
    if aq_ref.is_none() {
        return env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).volume;
    }

    let tmp: MutPtr<f32> = env.mem.alloc(guest_size_of::<f32>()).cast();
    let status = AudioQueueGetParameter(env, aq_ref.unwrap(), kAudioQueueParam_Volume, tmp);
    assert_eq!(status, 0);
    let res = env.mem.read(tmp);
    env.mem.free(tmp.cast());
    res
}

- (())setVolume:(f32)volume {
    let host_object = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
    host_object.volume = volume;
    if let Some(aq_ref) = host_object.audio_queue {
        let status = AudioQueueSetParameter(env, aq_ref, kAudioQueueParam_Volume, volume);
        assert_eq!(status, 0);
    }
}

- (())prepareToPlay {
    let audio_queue = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue;
    if audio_queue.is_some() { return; }

    let has_pcm = env.objc.borrow::<AVAudioPlayerHostObject>(this).pcm_data.is_some();
    
    // ЛОГИКА 1: Если мы успешно декодировали звук через Symphonia
    if has_pcm {
        let callback = env.objc.borrow::<AVAudioPlayerHostObject>(this).output_callback;
        let audio_desc = env.objc.borrow::<AVAudioPlayerHostObject>(this).audio_desc.unwrap();
        
        let size = guest_size_of::<AudioStreamBasicDescription>();
        let tmp_data_ptr: MutPtr<AudioStreamBasicDescription> = env.mem.alloc(size).cast();
        env.mem.write(tmp_data_ptr, audio_desc);

        let aq_ref_ptr: MutPtr<AudioQueueRef> = env.mem.alloc(guest_size_of::<AudioQueueRef>()).cast();
        let common_modes = ns_string::get_static_str(env, kCFRunLoopCommonModes);
        let status = AudioQueueNewOutput(
            env, tmp_data_ptr.cast_const(), callback, this.cast(),
            Ptr::null(), common_modes, 0, aq_ref_ptr
        );
        assert_eq!(status, 0);
        let aq_ref = env.mem.read(aq_ref_ptr);
        env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue = Some(aq_ref);

        let volume = env.objc.borrow::<AVAudioPlayerHostObject>(this).volume;
        () = msg![env; this setVolume:volume];
        let set_current_time = env.objc.borrow::<AVAudioPlayerHostObject>(this).set_current_time;
        () = msg![env; this setCurrentTime:set_current_time];

        let buffer_byte_size = 65536; // 64 KB буфер для чистого PCM
        let num_packets_to_read = buffer_byte_size / audio_desc.bytes_per_packet;
        env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).num_packets_to_read = num_packets_to_read;
        
        let buffers: MutPtr<AudioQueueBufferRef> = env.mem.alloc(kNumberBuffers as GuestUSize * guest_size_of::<AudioQueueBufferRef>()).cast();
        env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue_buffers = Some(buffers);

        env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).is_playing = true;
        for i in 0..kNumberBuffers {
            let status = AudioQueueAllocateBuffer(env, aq_ref, buffer_byte_size, buffers + i as u32);
            assert_eq!(status, 0);
            _touchHLE_AVAudioPlayerOutputBufferHelper(env, this.cast(), aq_ref, env.mem.read(buffers + i as u32));
        }
        env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).is_playing = false;

        env.mem.free(aq_ref_ptr.cast());
        env.mem.free(tmp_data_ptr.cast());
        return;
    }

    // ЛОГИКА 2: Оригинальное чтение через AudioFile (если это простой wav файл)
    let audio_file_id = env.objc.borrow::<AVAudioPlayerHostObject>(this).audio_file_id.unwrap();
    let callback = env.objc.borrow::<AVAudioPlayerHostObject>(this).output_callback;

    let size = guest_size_of::<AudioStreamBasicDescription>();
    let tmp_size_ptr: MutPtr<GuestUSize> = env.mem.alloc(guest_size_of::<GuestUSize>()).cast();
    env.mem.write(tmp_size_ptr, size);
    let tmp_data_ptr: MutPtr<AudioStreamBasicDescription> = env.mem.alloc(size).cast();
    let status = AudioFileGetProperty(env, audio_file_id, kAudioFilePropertyDataFormat, tmp_size_ptr, tmp_data_ptr.cast());
    assert_eq!(status, 0);
    assert_eq!(size, env.mem.read(tmp_size_ptr));
    let audio_desc = env.mem.read(tmp_data_ptr);
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_desc = Some(audio_desc);

    let aq_ref_ptr: MutPtr<AudioQueueRef> = env.mem.alloc(guest_size_of::<AudioQueueRef>()).cast();
    let common_modes = ns_string::get_static_str(env, kCFRunLoopCommonModes);
    let status = AudioQueueNewOutput(
        env, tmp_data_ptr.cast_const(), callback, this.cast(),
        Ptr::null(), common_modes, 0, aq_ref_ptr
    );
    assert_eq!(status, 0);
    let aq_ref = env.mem.read(aq_ref_ptr);
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue = Some(aq_ref);

    let volume = env.objc.borrow::<AVAudioPlayerHostObject>(this).volume;
    () = msg![env; this setVolume:volume];
    let set_current_time = env.objc.borrow::<AVAudioPlayerHostObject>(this).set_current_time;
    () = msg![env; this setCurrentTime:set_current_time];

    let size = guest_size_of::<u32>();
    env.mem.write(tmp_size_ptr, size);
    let prop_size_ptr: MutPtr<u32> = env.mem.alloc(size).cast();
    let status = AudioFileGetProperty(env, audio_file_id, kAudioFilePropertyPacketSizeUpperBound, tmp_size_ptr, prop_size_ptr.cast());
    assert_eq!(status, 0);
    assert_eq!(size, env.mem.read(tmp_size_ptr));
    let prop_size = env.mem.read(prop_size_ptr);

    let (buffer_byte_size, num_packets_to_read) = derive_buffer_size(audio_desc, prop_size, 0.5);
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).num_packets_to_read = num_packets_to_read;
    let buffers: MutPtr<AudioQueueBufferRef> = env.mem.alloc(kNumberBuffers as GuestUSize * guest_size_of::<AudioQueueBufferRef>()).cast();
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue_buffers = Some(buffers);

    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).is_playing = true;
    for i in 0..kNumberBuffers {
        let status = AudioQueueAllocateBuffer(env, aq_ref, buffer_byte_size, buffers + i as u32);
        assert_eq!(status, 0);
        _touchHLE_AVAudioPlayerOutputBufferHelper(env, this.cast(), aq_ref, env.mem.read(buffers + i as u32));
    }
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).is_playing = false;

    env.mem.free(tmp_size_ptr.cast());
    env.mem.free(aq_ref_ptr.cast());
    env.mem.free(tmp_data_ptr.cast());
}

- (bool)isPlaying {
    env.objc.borrow::<AVAudioPlayerHostObject>(this).is_playing
}

- (bool)play {
    () = msg![env; this prepareToPlay];
    let aq_ref = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue.unwrap();
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).is_playing = true;
    let status = AudioQueueStart(env, aq_ref, Ptr::null());
    assert_eq!(status, 0);
    true
}

- (())pause {
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).is_playing = false;
    if let Some(aq_ref) = env.objc.borrow::<AVAudioPlayerHostObject>(this).audio_queue {
        AudioQueuePause(env, aq_ref);
    }
}

- (())stop {
    let &mut AVAudioPlayerHostObject {
        audio_queue,
        audio_queue_buffers,
        ..
    } = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
    if audio_queue.is_none() {
        return;
    }
    AudioQueueDispose(env, audio_queue.unwrap(), true);
    env.mem.free(audio_queue_buffers.unwrap().cast());
    
    // Обязательно сохраняем pcm_data перед сбросом структуры!
    let mut pcm_backup = None;
    {
        let host_obj = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
        pcm_backup = host_obj.pcm_data.take();
    }
    
    let &AVAudioPlayerHostObject { audio_file_url, output_callback, num_of_loops, audio_file_id, .. } = env.objc.borrow(this);
    *env.objc.borrow_mut::<AVAudioPlayerHostObject>(this) = AVAudioPlayerHostObject {
        audio_file_url,
        output_callback,
        num_of_loops,
        audio_file_id,
        audio_desc: None,
        audio_queue: None,
        audio_queue_buffers: None,
        num_packets_to_read: 0,
        current_packet: 0,
        set_current_time: 0.0,
        volume: 1.0,
        is_playing: false,
        pcm_data: pcm_backup,
    };
}

- (())setNumberOfLoops:(NSInteger)numberOfLoops {
    log_dbg!("[(AVAudioPlayer *) {:?} setNumberOfLoops:{:?}]", this, numberOfLoops);
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).num_of_loops = numberOfLoops;
}

- (())dealloc {
    () = msg![env; this stop];
    let &AVAudioPlayerHostObject {audio_file_url, audio_file_id, ..} = env.objc.borrow(this);
    release(env, audio_file_url);
    if let Some(audio_file_id) = audio_file_id {
        AudioFileClose(env, audio_file_id);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

- (NSTimeInterval)currentTime {
    let host_object = env.objc.borrow::<AVAudioPlayerHostObject>(this);
    
    // Логика для кастомного PCM
    if host_object.pcm_data.is_some() {
        if let Some(audio_desc) = host_object.audio_desc {
            let bytes_per_frame = audio_desc.bytes_per_frame as f64;
            let current_frame = (host_object.current_packet as f64) / bytes_per_frame;
            return current_frame / audio_desc.sample_rate;
        }
        return 0.0;
    }
    
    // Оригинальная логика
    let current_time = if let Some(audio_desc) = host_object.audio_desc {
        let current_frame = (host_object.current_packet as f64) * (audio_desc.frames_per_packet as f64);
        current_frame / audio_desc.sample_rate
    } else {
        0.0
    };
    log_dbg!("[(AVAudioPlayer *) {:?} currentTime] -> {:?}", this, current_time);
    current_time
}

- (())setCurrentTime:(NSTimeInterval)currentTime {
    let host_object = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
    host_object.set_current_time = currentTime;
    
    // Логика для кастомного PCM
    if let Some(pcm) = &host_object.pcm_data {
        if let Some(audio_desc) = host_object.audio_desc {
            let bytes_per_frame = audio_desc.bytes_per_frame as f64;
            let target_frame = currentTime * audio_desc.sample_rate;
            let target_byte = (target_frame * bytes_per_frame) as usize;
            
            if target_byte > pcm.len() {
                host_object.current_packet = 0;
            } else {
                let alignment = audio_desc.bytes_per_frame as usize;
                host_object.current_packet = (target_byte - (target_byte % alignment)) as i64;
            }
        }
        return;
    }

    // Оригинальная логика
    if let (Some(audio_desc), Some(audio_file_id)) = (host_object.audio_desc, host_object.audio_file_id) {
        let total_packets = audio_file::State::get(&mut env.framework_state).audio_files.get(&audio_file_id).unwrap().audio_file.packet_count();
        let total_frames = total_packets * audio_desc.frames_per_packet as u64;
        let new_current_frame = audio_desc.sample_rate * currentTime;
        if new_current_frame < 0.0 || new_current_frame > total_frames as f64 {
            host_object.current_packet = 0;
        } else {
            host_object.current_packet = (new_current_frame / (audio_desc.frames_per_packet as f64)) as i64;
        }
    }
    log_dbg!("[(AVAudioPlayer *) {:?} setCurrentTime: {}]", this, currentTime);
}

@end

};

fn derive_buffer_size(
    audio_desc: AudioStreamBasicDescription,
    max_packet_size: u32,
    seconds: f64,
) -> (u32, u32) {
    let mut out_buffer_size;
    const max_buffer_size: u32 = 0x50000;
    const min_buffer_size: u32 = 0x4000;
    if audio_desc.frames_per_packet != 0 {
        let num_packets_to_time =
            audio_desc.sample_rate / audio_desc.frames_per_packet as f64 * seconds;
        out_buffer_size = num_packets_to_time as u32 * max_packet_size;
    } else {
        out_buffer_size = if max_buffer_size > max_packet_size {
            max_buffer_size
        } else {
            max_packet_size
        }
    }

    if out_buffer_size > max_buffer_size && out_buffer_size > max_packet_size {
        out_buffer_size = max_buffer_size
    } else if out_buffer_size < min_buffer_size {
        out_buffer_size = min_buffer_size
    }

    let out_num_packets_to_read = out_buffer_size / max_packet_size;
    (out_buffer_size, out_num_packets_to_read)
}

fn _touchHLE_AVAudioPlayerOutputBufferHelper(
    env: &mut Environment,
    in_user_data: MutVoidPtr,
    in_aq: AudioQueueRef,
    in_buf: AudioQueueBufferRef,
) {
    let av_audio_player: id = in_user_data.cast();
    let class: Class = msg![env; av_audio_player class];
    
    assert_eq!(
        class,
        env.objc.get_known_class("AVAudioPlayer", &mut env.mem)
    );
    
    let is_playing = env.objc.borrow::<AVAudioPlayerHostObject>(av_audio_player).is_playing;
    if !is_playing { return; }

    // ЛОГИКА 1: Загрузка прямо из памяти (Symphonia)
    // ИСПРАВЛЕНИЕ E0499: Изолируем чтения и запись в память от долгих блокировок (borrows)
    let has_pcm = env.objc.borrow::<AVAudioPlayerHostObject>(av_audio_player).pcm_data.is_some();
    if has_pcm {
        // Сначала собираем все нужные переменные и отпускаем Borrow!
        let (pcm_len, current_byte, bytes_to_read, aq, num_of_loops) = {
            let host_obj = env.objc.borrow::<AVAudioPlayerHostObject>(av_audio_player);
            let pcm_len = host_obj.pcm_data.as_ref().unwrap().len();
            let current_byte = host_obj.current_packet as usize;
            let bytes_to_read = host_obj.num_packets_to_read as usize * host_obj.audio_desc.unwrap().bytes_per_packet as usize;
            (pcm_len, current_byte, bytes_to_read, host_obj.audio_queue.unwrap(), host_obj.num_of_loops)
        };
        
        let mut audio_queue_buffer = env.mem.read(in_buf);
        
        if current_byte < pcm_len {
            let bytes_copied = std::cmp::min(bytes_to_read, pcm_len - current_byte);
            
            // Копируем срез в отдельный вектор, чтобы не держать Borrow
            let slice_data = {
                let host_obj = env.objc.borrow::<AVAudioPlayerHostObject>(av_audio_player);
                let pcm = host_obj.pcm_data.as_ref().unwrap();
                pcm[current_byte .. current_byte + bytes_copied].to_vec()
            };
            
            // Пишем данные из вектора в память эмулятора
            let dst = env.mem.bytes_at_mut(audio_queue_buffer.audio_data.cast(), bytes_copied as u32);
            dst.copy_from_slice(&slice_data);
            
            audio_queue_buffer.audio_data_byte_size = bytes_copied as u32;
            env.mem.write(in_buf, audio_queue_buffer);
            
            let status = AudioQueueEnqueueBuffer(env, aq, in_buf, 0, Ptr::null());
            assert_eq!(status, 0);
            
            // Снова берем Borrow только на время прибавления
            let mut host_obj = env.objc.borrow_mut::<AVAudioPlayerHostObject>(av_audio_player);
            host_obj.current_packet += bytes_copied as i64;
        } else {
            // Конец файла (EOF)
            if num_of_loops == 0 {
                AudioQueueStop(env, aq, false);
                let mut host_obj = env.objc.borrow_mut::<AVAudioPlayerHostObject>(av_audio_player);
                host_obj.is_playing = false;
            } else {
                {
                    let mut host_obj = env.objc.borrow_mut::<AVAudioPlayerHostObject>(av_audio_player);
                    if host_obj.num_of_loops > 0 {
                        host_obj.num_of_loops -= 1;
                    }
                    host_obj.current_packet = 0;
                }
                // Рекурсивный вызов делаем, когда нет Borrow
                _touchHLE_AVAudioPlayerOutputBufferHelper(env, in_user_data, in_aq, in_buf);
            }
        }
        return;
    }

    // ЛОГИКА 2: Оригинальная логика через AudioFile
    let &AVAudioPlayerHostObject { audio_file_id, audio_queue, num_packets_to_read, current_packet, .. } = env.objc.borrow(av_audio_player);
    let aq = audio_queue.unwrap();
    assert_eq!(aq, in_aq);

    let num_bytes_ptr: MutPtr<u32> = env.mem.alloc(guest_size_of::<u32>()).cast();
    let num_packets_ptr: MutPtr<u32> = env.mem.alloc(guest_size_of::<u32>()).cast();
    env.mem.write(num_packets_ptr, num_packets_to_read);
    let mut audio_queue_buffer = env.mem.read(in_buf);
    
    let status = AudioFileReadPackets(
        env,
        audio_file_id.unwrap(),
        0, 
        num_bytes_ptr,
        Ptr::null(),
        current_packet,
        num_packets_ptr,
        audio_queue_buffer.audio_data,
    );
    let num_packets = env.mem.read(num_packets_ptr);
    let num_bytes = env.mem.read(num_bytes_ptr);
    env.mem.free(num_packets_ptr.cast());
    env.mem.free(num_bytes_ptr.cast());
    
    if num_packets > 0 {
        assert!(status == 0 || status == eofErr);
        audio_queue_buffer.audio_data_byte_size = num_bytes;
        env.mem.write(in_buf, audio_queue_buffer);
        let status = AudioQueueEnqueueBuffer(env, aq, in_buf, 0, Ptr::null());
        assert_eq!(status, 0);
        env.objc
            .borrow_mut::<AVAudioPlayerHostObject>(av_audio_player)
            .current_packet = current_packet + num_packets as i64;
    } else {
        assert_eq!(status, eofErr);
        let number_of_loops = env.objc.borrow::<AVAudioPlayerHostObject>(av_audio_player).num_of_loops;
        if number_of_loops == 0 {
            let status = AudioQueueStop(env, aq, false);
            assert_eq!(status, 0);
            env.objc.borrow_mut::<AVAudioPlayerHostObject>(av_audio_player).is_playing = false;
        } else {
            if number_of_loops > 0 {
                env.objc.borrow_mut::<AVAudioPlayerHostObject>(av_audio_player).num_of_loops -= 1;
            }
            env.objc.borrow_mut::<AVAudioPlayerHostObject>(av_audio_player).current_packet = 0;
            _touchHLE_AVAudioPlayerOutputBufferHelper(env, in_user_data, in_aq, in_buf);
        }
    }
}


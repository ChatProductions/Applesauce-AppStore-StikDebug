/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Quick-and-dirty decoding of miscellaneous formats (MP3, AAC, CAF) to linear PCM.
//!
//! This should be the only module in touchHLE that makes use of [symphonia].

use std::io::Cursor;
use symphonia::core::audio::AudioSpec;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;

/// PCM data decoded from an miscellaneous format file.
pub struct SymphoniaDecodedToPcm {
    /// 16-bit little-endian PCM samples, grouped in frames (one sample per
    /// channel in each frame).
    pub bytes: Vec<u8>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u32,
}

pub fn decode_symphonia_to_pcm(mut file: Cursor<Vec<u8>>) -> Result<SymphoniaDecodedToPcm, ()> {
    // 1. ИСПРАВЛЕНИЕ: Жестко сбрасываем позицию курсора на начало.
    // Без этого MediaSourceStream может прочитать 0 байт и умереть с "no suitable format reader".
    file.set_position(0);

    // Читаем magic bytes (первые 4 байта), чтобы точно знать в логах, что мы пытаемся парсить
    let mut magic = [0u8; 4];
    if std::io::Read::read_exact(&mut file, &mut magic).is_ok() {
        let magic_str = std::str::from_utf8(&magic).unwrap_or("unknown");
        log!("Symphonia incoming file magic: {}", magic_str);
        if &magic == b"caff" {
            log!("WARNING: This is an Apple CAF file! Symphonia does not natively support CAF containers. This will fail unless handled by a custom CAF decoder.");
        }
    }
    // Возвращаем курсор в начало после чтения magic bytes
    file.set_position(0);

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // 2. ИСПРАВЛЕНИЕ: Используем правильный Hint вместо Default
    let hint = Hint::new();

    // Пробуем определить формат
    let mut probed = match symphonia::default::get_probe()
        .probe(&hint, mss, Default::default(), Default::default()) {
        Ok(p) => p,
        Err(e) => {
            log!("Symphonia fatal probe error: {:?}", e);
            log!("FIX: Ensure Cargo.toml has features enabled: symphonia = {{ version = \"...\", features = [\"all\"] }}");
            return Err(());
        }
    };

    // Настраиваем декодер
    let mut decoder = match symphonia::default::get_codecs()
        .make(&probed.format.default_track().unwrap().codec_params, &Default::default()) {
        Ok(d) => d,
        Err(e) => {
            log!("Symphonia codec creation error: {:?}", e);
            return Err(());
        }
    };

    let track_id = probed.format.default_track().unwrap().id;
    let mut out_pcm = Vec::new();
    let mut audio_spec: Option<AudioSpec> = None;
    let mut tmp_raw_s16_buf = None;

    // Основной цикл чтения и декодирования фреймов
    loop {
        let packet = match probed.format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break; // Нормальный конец файла
            }
            Err(e) => {
                log!("Symphonia packet read error: {:?} (Stopping read but keeping decoded audio)", e);
                break;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded_packet = match decoder.decode(&packet) {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::DecodeError(e)) => {
                // Ошибки декодирования (битый фрейм MP3/AAC) можно игнорировать и идти дальше
                log!("Symphonia decode error (recoverable): {:?}", e);
                continue;
            }
            Err(e) => {
                log!("Symphonia fatal decode error: {:?}", e);
                break;
            }
        };

        let audio_spec = audio_spec.get_or_insert_with(|| decoded_packet.spec().clone());

        let tmp_raw_s16_buf = tmp_raw_s16_buf
            .get_or_insert_with(|| Vec::with_capacity(decoded_packet.capacity()));

        tmp_raw_s16_buf.clear();
        decoded_packet.copy_bytes_to_vec_interleaved_as::<i16>(tmp_raw_s16_buf);

        out_pcm.extend_from_slice(tmp_raw_s16_buf);
    }

    let audio_spec = audio_spec.ok_or_else(|| {
        log!("Symphonia: File yielded no valid audio data");
    })?;

    Ok(SymphoniaDecodedToPcm {
        bytes: out_pcm,
        sample_rate: audio_spec.rate,
        channels: audio_spec.channels.count() as u32,
    })
}

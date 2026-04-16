/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Quick-and-dirty decoding of miscellaneous formats (MP3, AAC) to linear PCM.
//!
//! This should be the only module in touchHLE that makes use of [symphonia].
//!
//! For AAC, Only the LC profile and MPEG-4 container format are supported (see
//! feature list in Cargo.toml).

use std::io::Cursor;
use symphonia::core::audio::{SampleBuffer, Signal};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::codecs::audio::well_known::{
    CODEC_ID_AAC, CODEC_ID_ADPCM_IMA_QT, CODEC_ID_ADPCM_IMA_WAV, CODEC_ID_ALAC, CODEC_ID_MP3,
    CODEC_ID_PCM_S16LE,
};
use symphonia::core::formats::FormatReader;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::core::errors::Error as SymphoniaError;

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

/// Потоковый декодер, который можно использовать в AudioToolbox 
/// для чтения аудиофайлов "на лету" без полной загрузки в RAM.
pub struct SymphoniaStreamer {
    pub format_reader: Box<dyn FormatReader>,
    pub decoder: Box<dyn Decoder>,
    pub track_id: u32,
    pub sample_rate: u32,
    pub channels: u32,
}

impl SymphoniaStreamer {
    pub fn open(mss: MediaSourceStream) -> Result<Self, ()> {
        let mut probed = symphonia::default::get_probe()
            .probe(
                &Hint::new(),
                mss,
                &Default::default(),
                &Default::default(),
            )
            .map_err(|_| ())?;

        let track = probed.format
            .tracks()
            .iter()
            .find(|t| {
                if let Some(codec_params) = &t.codec_params {
                    // TouchHLE codec probing logic
                    codec_params.codec == CODEC_ID_AAC
                        || codec_params.codec == CODEC_ID_ADPCM_IMA_WAV
                        || codec_params.codec == CODEC_ID_ADPCM_IMA_QT
                        || codec_params.codec == CODEC_ID_ALAC
                        || codec_params.codec == CODEC_ID_MP3
                        || codec_params.codec == CODEC_ID_PCM_S16LE
                } else {
                    false
                }
            })
            .ok_or(())?;
        
        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        // Безопасное извлечение sample_rate и channels
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params.channels.map_or(2, |c| c.count() as u32);

        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&codec_params, &DecoderOptions::default())
            .map_err(|_| ())?;

        Ok(Self {
            format_reader: probed.format,
            decoder,
            track_id,
            sample_rate,
            channels,
        })
    }

    /// Читает и декодирует следующий аудиопакет. Возвращает Interleaved PCM (i16 в байтах).
    pub fn next_pcm_packet(&mut self) -> Result<Option<Vec<u8>>, SymphoniaError> {
        loop {
            let packet = match self.format_reader.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None); // Конец файла
                }
                Err(e) => return Err(e),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(audio_buf) => {
                    // Используем SampleBuffer для правильного interleaved микширования
                    let mut sample_buf = SampleBuffer::<i16>::new(
                        audio_buf.capacity() as u64,
                        *audio_buf.spec(),
                    );
                    sample_buf.copy_interleaved_ref(audio_buf);

                    let raw_bytes = sample_buf.samples();
                    
                    // Переводим массив i16 в массив u8 для отправки в память эмулятора
                    let byte_slice: &[u8] = unsafe {
                        std::slice::from_raw_parts(
                            raw_bytes.as_ptr() as *const u8,
                            raw_bytes.len() * 2, // 2 байта на 1 сэмпл (16-bit)
                        )
                    };
                    
                    return Ok(Some(byte_slice.to_vec()));
                }
                Err(SymphoniaError::DecodeError(_)) => {
                    // Пропускаем "битые" пакеты и пытаемся прочитать следующий
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

pub fn decode_symphonia_to_pcm(file: Cursor<Vec<u8>>) -> Result<SymphoniaDecodedToPcm, ()> {
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    
    // Используем наш новый стример для чтения файла до конца
    let mut streamer = SymphoniaStreamer::open(mss)?;
    let mut out_pcm = Vec::<u8>::new();

    // Читаем все пакеты по очереди (как было раньше)
    while let Ok(Some(pcm_bytes)) = streamer.next_pcm_packet() {
        out_pcm.extend_from_slice(&pcm_bytes);
    }

    if out_pcm.is_empty() {
        return Err(());
    }

    Ok(SymphoniaDecodedToPcm {
        bytes: out_pcm,
        sample_rate: streamer.sample_rate,
        channels: streamer.channels,
    })
}


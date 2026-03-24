/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, you can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Audio file decoding and OpenAL bindings.
//!
//! The audio file decoding support is an abstraction over various libraries
//! (currently [caf], [hound], and [symphonia]), usage of which should be
//! confined to this module.
//!
//! Resources:
//! - [Apple Core Audio Format Specification 1.0](https://developer.apple.com/library/archive/documentation/MusicAudio/Reference/CAFSpec/CAF_intro/CAF_intro.html)

mod ima4;
pub mod openal;
mod symphonia_formats;

pub use ima4::decode_ima4;

use crate::fs::{Fs, GuestPath};
use std::io::Cursor;

#[derive(Debug)]
pub enum AudioFileOpenError {
    FileReadError,
    FileDecodeError,
}

#[derive(Debug)]
pub enum AudioFormat {
    LinearPcm {
        is_float: bool,
        is_little_endian: bool,
    },
    AppleIma4,
}

/// Fields have the same meanings as in the Core Audio Format's
/// Audio Description chunk, which is in turn similar to Core Audio Types'
/// AudioStreamBasicDescription.
#[derive(Debug)]
pub struct AudioDescription {
    /// Hz
    pub sample_rate: f64,
    pub format: AudioFormat,
    pub bytes_per_packet: u32,
    pub frames_per_packet: u32,
    pub channels_per_frame: u32,
    pub bits_per_channel: u32,
}

pub struct AudioFile(AudioFileInner);

enum AudioFileInner {
    Wave(hound::WavReader<Cursor<Vec<u8>>>),
    Caf(caf::CafPacketReader<Cursor<Vec<u8>>>),
    Symphonia(symphonia_formats::SymphoniaDecodedToPcm),
}

impl AudioFile {
    pub fn open_for_reading<P: AsRef<GuestPath>>(
        path: P,
        fs: &Fs,
    ) -> Result<Self, AudioFileOpenError> {
        let Ok(bytes) = fs.read(path.as_ref()) else {
            return Err(AudioFileOpenError::FileReadError);
        };

        if let Ok(audio_file) = Self::read_from_vec(bytes) {
            Ok(audio_file)
        } else {
            log!(
                "Could not decode audio file at path {:?}, likely an \
                unimplemented file format.",
                path.as_ref()
            );
            Err(AudioFileOpenError::FileReadError)
        }
    }

    pub fn read_from_vec(bytes: Vec<u8>) -> Result<Self, AudioFileOpenError> {
        // Try hound (WAV) first, but only for supported 8/16-bit
        // integer formats
        if let Ok(reader) = hound::WavReader::new(Cursor::new(&bytes)) {
            let spec = reader.spec();
            if (spec.bits_per_sample == 8 || spec.bits_per_sample == 16)
                && spec.sample_format == hound::SampleFormat::Int
            {
                let reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
                return Ok(AudioFile(AudioFileInner::Wave(reader)));
            }
            // Fall through to Symphonia for 24-bit, 32-bit or float WAVs
        }

        if caf::CafPacketReader::new(Cursor::new(&bytes), vec![]).is_ok() {
            let reader = caf::CafPacketReader::new(Cursor::new(bytes), vec![]).unwrap();
            Ok(AudioFile(AudioFileInner::Caf(reader)))
        } else if let Ok(pcm) = symphonia_formats::decode_symphonia_to_pcm(Cursor::new(bytes)) {
            Ok(AudioFile(AudioFileInner::Symphonia

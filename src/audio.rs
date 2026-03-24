/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
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
/// `AudioStreamBasicDescription`.
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
                "Could not decode audio file at path {:?}, likely an unimplemented file format.",
                path.as_ref()
            );
            Err(AudioFileOpenError::FileReadError)
        }
    }

    pub fn read_from_vec(bytes: Vec<u8>) -> Result<Self, AudioFileOpenError> {
        // Try hound (WAV) first, but only for supported 8/16-bit integer formats
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
            Ok(AudioFile(AudioFileInner::Symphonia(pcm)))
        } else {
            Err(AudioFileOpenError::FileDecodeError)
        }
    }

    pub fn audio_description(&self) -> AudioDescription {
        match self.0 {
            AudioFileInner::Wave(ref wave_reader) => {
                let hound::WavSpec {
                    channels,
                    sample_rate,
                    bits_per_sample,
                    sample_format,
                } = wave_reader.spec();
                
                assert!(matches!(bits_per_sample, 8 | 16));
                assert!(sample_format == hound::SampleFormat::Int);

                AudioDescription {
                    sample_rate: sample_rate.into(),
                    format: AudioFormat::LinearPcm {
                        is_float: false,
                        is_little_endian: true,
                    },
                    bytes_per_packet: u32::from(channels) * u32::from(bits_per_sample) / 8,
                    frames_per_packet: 1,
                    channels_per_frame: channels.into(),
                    bits_per_channel: bits_per_sample as u32,
                }
            }
            AudioFileInner::Caf(ref caf_reader) => {
                let caf::chunks::AudioDescription {
                    sample_rate,
                    ref format_id,
                    format_flags,
                    bytes_per_packet,
                    frames_per_packet,
                    channels_per_frame,
                    bits_per_channel,
                } = caf_reader.audio_desc;

                AudioDescription {
                    sample_rate,
                    format: match format_id {
                        caf::FormatType::LinearPcm => {
                            assert!((format_flags & !3) == 0);
                            let is_float = (format_flags & 1) == 1;
                            let is_little_endian = (format_flags & 2) == 2;
                            AudioFormat::LinearPcm {
                                is_float,
                                is_little_endian,
                            }
                        }
                        caf::FormatType::AppleIma4 => {
                            assert!(format_flags == 0);
                            AudioFormat::AppleIma4
                        }
                        _ => panic!("{format_id:?} not supported yet"),
                    },
                    bytes_per_packet,
                    frames_per_packet,
                    channels_per_frame,
                    bits_per_channel,
                }
            }
            AudioFileInner::Symphonia(symphonia_formats::SymphoniaDecodedToPcm {
                sample_rate,
                channels,
                ..
            }) => AudioDescription {
                sample_rate: f64::from(sample_rate),
                format: AudioFormat::LinearPcm {
                    is_float: false,
                    is_little_endian: true,
                },
                bytes_per_packet: channels * 2,
                frames_per_packet: 1,
                channels_per_frame: channels,
                bits_per_channel: 16,
            },
        }
    }

    fn bytes_per_sample(&self) -> u64 {
        let AudioDescription {
            format,
            bytes_per_packet,
            frames_per_packet,
            channels_per_frame,
            ..
        } = self.audio_description();
        if !matches!(format, AudioFormat::LinearPcm { .. }) {
            panic!("{format:?} is a compressed format!");
        }
        ((bytes_per_packet / frames_per_packet) / channels_per_frame).into()
    }

    pub fn byte_count(&self) -> u64 {
        match self.0 {
            AudioFileInner::Wave(ref wave_reader) => {
                let sample_count = wave_reader.len();
                u64::from(sample_count) * self.bytes_per_sample()
            }
            AudioFileInner::Caf(_) => {
                u64::from(self.packet_size_fixed()) * self.packet_count()
            }
            AudioFileInner::Symphonia(symphonia_formats::SymphoniaDecodedToPcm {
                ref bytes,
                ..
            }) => bytes.len() as u64,
        }
    }

    pub fn packet_count(&self) -> u64 {
        match self.0 {
            AudioFileInner::Wave(_)
            | AudioFileInner::Symphonia(symphonia_formats::SymphoniaDecodedToPcm { .. }) => {
                self.byte_count() / u64::from(self.packet_size_fixed())
            }
            AudioFileInner::Caf(ref caf_reader) => {
                caf_reader.get_packet_count().unwrap().try_into().unwrap()
            }
        }
    }

    pub fn packet_size_fixed(&self) -> u32 {
        let AudioDescription {
            bytes_per_packet, ..
        } = self.audio_description();
        assert!(bytes_per_packet != 0);
        bytes_per_packet
    }

    pub fn packet_size_upper_bound(&self) -> u32 {
        self.packet_size_fixed()
    }

    pub fn read_bytes(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, ()> {
        match self.0 {
            AudioFileInner::Wave(_) => {
                let bytes_per_sample = self.bytes_per_sample();
                if bytes_per_sample == 0 { return Err(()); }
                
                assert!(offset % bytes_per_sample == 0);
                assert!(buffer.len() as u64 % bytes_per_sample == 0);

                let sample_count = buffer.len() as u64 / bytes_per_sample;
                let sample_count: usize = sample_count.try_into().unwrap();

                let AudioFileInner::Wave(ref mut wave_reader) = self.0 else {
                    unreachable!()
                };

                let channels: u64 = wave_reader.spec().channels.into();
                wave_reader
                    .seek((offset / (bytes_per_sample * channels)).try_into().unwrap())
                    .map_err(|_| ())?;

                let mut byte_offset = 0;
                for sample in wave_reader.samples().take(sample_count) {
                    let sample: i16 = sample.map_err(|_| ())?;
                    match bytes_per_sample {
                        1 => buffer[byte_offset] = (sample + 128) as u8,
                        2 => buffer[byte_offset..][..2].copy_from_slice(&sample.to_le_bytes()),
                        _ => todo!(),
                    }
                    byte_offset += bytes_per_sample as usize;
                }
                Ok(byte_offset)
            }
            AudioFileInner::Caf(_) => {
                let packet_size = self.packet_size_fixed();
                assert!(offset % u64::from(packet_size) == 0);
                assert!(buffer.len() as u64 % u64::from(packet_size) == 0);

                let packet_count = buffer.len() as u64 / u64::from(packet_size);

                let AudioFileInner::Caf(ref mut caf_reader) = self.0 else {
                    unreachable!()
                };

                caf_reader
                    .seek_to_packet(usize::try_from(offset / u64::from(packet_size)).unwrap())
                    .map_err(|_| ())?;

                let packet_size_usize = usize::try_from(packet_size).unwrap();
                let mut i = 0;
                let mut byte_offset = 0;
                while i < packet_count && caf_reader.next_packet_size().is_some() {
                    caf_reader
                        .read_packet_into(&mut buffer[byte_offset..][..packet_size_usize])
                        .map_err(|_| ())?;
                    byte_offset += packet_size_usize;
                    i += 1;
                }
                Ok(byte_offset)
            }
            AudioFileInner::Symphonia(symphonia_formats::SymphoniaDecodedToPcm {
                ref bytes,
                ..
            }) => {
                let bytes_slice = bytes.get(offset as usize..).ok_or(())?;
                let bytes_to_read = buffer.len().min(bytes_slice.len());
                buffer[..bytes_to_read].copy_from_slice(&bytes_slice[..bytes_to_read]);
                Ok(bytes_to_read)
            }
        }
    }
}


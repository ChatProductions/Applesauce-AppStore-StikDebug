/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Standalone CAF (Apple Core Audio Format) → 16-bit little-endian PCM decoder.
//!
//! This is used as a fallback when [`super::symphonia_formats`] cannot probe a
//! CAF file. The CAF demuxer in `symphonia-format-caf 0.6.0-alpha.1` does not
//! correctly handle CAF files where the Audio Data chunk's `mChunkSize` is set
//! to `-1` (which the CAF specification explicitly allows for the last chunk
//! to mean "extends to the end of the file"); on such files Symphonia walks
//! off the end of the audio data trying to read a chunk header and bails out
//! with `IoError(UnexpectedEof)`. Plants vs. Zombies (`com.popcap.PvZ`) ships
//! `sounds/*.caf` files in this layout, which is what was preventing its
//! sound effects from playing.
//!
//! References:
//! - Apple, *Apple Core Audio Format Specification 1.0*, "The Audio Data Chunk"
//!   <https://developer.apple.com/library/archive/documentation/MusicAudio/Reference/CAFSpec/CAF_chunks/CAF_chunks.html>
//! - Apple, *Apple Core Audio Format Specification 1.0*, "The Audio Description Chunk"
//!   <https://developer.apple.com/library/archive/documentation/MusicAudio/Reference/CAFSpec/CAF_chunks/CAF_chunks.html#//apple_ref/doc/uid/TP40001862-CH210-SW2>
//! - Apple, *AudioServicesCreateSystemSoundID*
//!   <https://developer.apple.com/documentation/audiotoolbox/audioservicescreatesystemsoundid(_:_:)>

use super::ima4::decode_ima4;
use super::symphonia_formats::SymphoniaDecodedToPcm;
use std::io::Cursor;

/// Decode the contents of a `.caf` file into 16-bit little-endian interleaved
/// PCM, returning the same in-memory shape that [`SymphoniaDecodedToPcm`]
/// uses so the rest of the audio pipeline can consume it uniformly.
///
/// Currently supported audio data formats inside the CAF container:
/// - `lpcm` — Linear PCM (8/16/24/32-bit signed integer, big- or little-endian).
///   Float PCM is rejected.
/// - `ima4` — Apple IMA 4:1 ADPCM (mono or stereo).
///
/// Anything else (MPEG-4 AAC, MP3, ALAC, …) is returned as `Err(())` so the
/// caller can fall through to a different decoder.
pub fn decode_caf_to_pcm(file: Cursor<Vec<u8>>) -> Result<SymphoniaDecodedToPcm, ()> {
    use caf::FormatType;

    let mut reader = caf::CafPacketReader::new(file, vec![]).map_err(|_| ())?;
    let desc = reader.audio_desc.clone();

    let sample_rate: u32 = desc.sample_rate.round() as u32;
    let channels: u32 = desc.channels_per_frame;
    if sample_rate == 0 || channels == 0 {
        return Err(());
    }

    let mut out_pcm: Vec<u8> = Vec::new();

    match desc.format_id {
        FormatType::AppleIma4 => {
            // CAF IMA4: each packet covers `frames_per_packet` (= 64) frames,
            // and one packet's worth of bytes is `34 * channels_per_frame`
            // (per Apple's spec). For stereo, the packet data is the left
            // channel's 34-byte sub-packet immediately followed by the right
            // channel's 34-byte sub-packet.
            //
            // We decode each 34-byte sub-packet through `decode_ima4` exactly
            // like `audio_queue::decode_buffer` does, but eagerly for the
            // whole file.
            let sub_packet_bytes: usize = 34;
            let expected_packet_bytes: usize = sub_packet_bytes * channels as usize;
            if desc.bytes_per_packet as usize != expected_packet_bytes && desc.bytes_per_packet != 0
            {
                // Not actually a 34-bytes-per-channel packet layout — refuse
                // rather than producing garbage.
                return Err(());
            }

            // Greedily collect every CAF packet, then chunk into 34-byte
            // sub-packets and decode in (channel, channel, …) order.
            let mut all_bytes: Vec<u8> = Vec::new();
            while let Ok(Some(pkt)) = reader.next_packet() {
                all_bytes.extend_from_slice(&pkt);
            }

            if !all_bytes.len().is_multiple_of(sub_packet_bytes) {
                return Err(());
            }

            let mut sub_packets = all_bytes.chunks_exact(sub_packet_bytes);
            match channels {
                1 => {
                    for sub in sub_packets.by_ref() {
                        let pcm: [i16; 64] = decode_ima4(sub.try_into().unwrap());
                        for s in &pcm {
                            out_pcm.extend_from_slice(&s.to_le_bytes());
                        }
                    }
                }
                2 => {
                    while let Some(left) = sub_packets.next() {
                        let Some(right) = sub_packets.next() else {
                            return Err(());
                        };
                        let l_pcm: [i16; 64] = decode_ima4(left.try_into().unwrap());
                        let r_pcm: [i16; 64] = decode_ima4(right.try_into().unwrap());
                        for (l, r) in l_pcm.iter().zip(r_pcm.iter()) {
                            out_pcm.extend_from_slice(&l.to_le_bytes());
                            out_pcm.extend_from_slice(&r.to_le_bytes());
                        }
                    }
                }
                _ => return Err(()),
            }
        }
        FormatType::LinearPcm => {
            // CAF audio-description format flags (Apple CAF spec):
            //   bit 0 — kCAFLinearPCMFormatFlagIsFloat
            //   bit 1 — kCAFLinearPCMFormatFlagIsLittleEndian
            // Float PCM is rejected here because the rest of the pipeline only
            // accepts 16-bit signed integer little-endian PCM.
            let is_float = (desc.format_flags & 0b01) != 0;
            let is_little_endian = (desc.format_flags & 0b10) != 0;
            if is_float {
                return Err(());
            }

            let bits = desc.bits_per_channel;
            if !matches!(bits, 8 | 16 | 24 | 32) {
                return Err(());
            }

            let bytes_per_sample = (bits / 8) as usize;
            while let Ok(Some(pkt)) = reader.next_packet() {
                for sample in pkt.chunks_exact(bytes_per_sample) {
                    let mut buf = [0u8; 8];
                    buf[..sample.len()].copy_from_slice(sample);
                    let s16 = match (bits, is_little_endian) {
                        (8, _) => {
                            // CAF 8-bit LPCM is signed (per spec); convert to
                            // signed 16-bit by sign-extending.
                            let v = buf[0] as i8;
                            (v as i16) << 8
                        }
                        (16, true) => i16::from_le_bytes([buf[0], buf[1]]),
                        (16, false) => i16::from_be_bytes([buf[0], buf[1]]),
                        (24, true) => {
                            let v = (buf[0] as i32)
                                | ((buf[1] as i32) << 8)
                                | (((buf[2] as i8) as i32) << 16);
                            (v >> 8) as i16
                        }
                        (24, false) => {
                            let v = (buf[2] as i32)
                                | ((buf[1] as i32) << 8)
                                | (((buf[0] as i8) as i32) << 16);
                            (v >> 8) as i16
                        }
                        (32, true) => {
                            let v = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
                            (v >> 16) as i16
                        }
                        (32, false) => {
                            let v = i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                            (v >> 16) as i16
                        }
                        _ => unreachable!(),
                    };
                    out_pcm.extend_from_slice(&s16.to_le_bytes());
                }
            }
        }
        // Compressed formats other than IMA4 (AAC, MP3, ALAC, …) — leave them
        // for Symphonia to handle when it eventually grows correct support.
        _ => return Err(()),
    }

    if out_pcm.is_empty() {
        return Err(());
    }

    Ok(SymphoniaDecodedToPcm {
        bytes: out_pcm,
        sample_rate,
        channels,
    })
}

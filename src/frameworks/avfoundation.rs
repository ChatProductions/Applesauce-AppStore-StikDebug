/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! The AVFoundation framework.

mod av_audio_player;
pub mod av_audio_session;
pub mod av_capture;

use crate::dyld::{ConstantExports, HostConstant};
use crate::objc::id;
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    pub av_audio_session: av_audio_session::State,
    pub av_capture: av_capture::State,
    /// Side-table for `AVCaptureVideoPreviewLayer` instances. The base
    /// `CALayerHostObject` is allocated by `+[CALayer allocWithZone:]`, so
    /// subclass-specific state (the AVCaptureSession the layer is bound to,
    /// the videoGravity string, etc.) lives here keyed by layer `id`.
    pub av_capture_preview_extras: HashMap<id, av_capture::AVCapturePreviewLayerExtra>,
}

/// Constants commonly referenced from iOS 5/6 binaries that don't yet have a
/// dedicated host implementation. Their actual string values are observable
/// only through identity comparisons / dictionary lookups, so spelling them
/// canonically (matching Apple's headers) is sufficient.
pub const STUB_CONSTANTS: ConstantExports = &[
    // AVPlayerItem notification names.
    (
        "_AVPlayerItemDidPlayToEndTimeNotification",
        HostConstant::NSString("AVPlayerItemDidPlayToEndTimeNotification"),
    ),
    (
        "_AVPlayerItemFailedToPlayToEndTimeNotification",
        HostConstant::NSString("AVPlayerItemFailedToPlayToEndTimeNotification"),
    ),
    (
        "_AVPlayerItemPlaybackStalledNotification",
        HostConstant::NSString("AVPlayerItemPlaybackStalledNotification"),
    ),
    (
        "_AVPlayerItemNewErrorLogEntryNotification",
        HostConstant::NSString("AVPlayerItemNewErrorLogEntryNotification"),
    ),
    (
        "_AVPlayerItemNewAccessLogEntryNotification",
        HostConstant::NSString("AVPlayerItemNewAccessLogEntryNotification"),
    ),
    // AVAudioRecorder settings dictionary keys (apps reach them through
    // Mach-O symbol lookup, so we expose them as NSString constants whose
    // identity matches Apple's value).
    (
        "_AVFormatIDKey",
        HostConstant::NSString("AVFormatIDKey"),
    ),
    (
        "_AVSampleRateKey",
        HostConstant::NSString("AVSampleRateKey"),
    ),
    (
        "_AVNumberOfChannelsKey",
        HostConstant::NSString("AVNumberOfChannelsKey"),
    ),
    (
        "_AVEncoderAudioQualityKey",
        HostConstant::NSString("AVEncoderAudioQualityKey"),
    ),
    (
        "_AVEncoderBitRateKey",
        HostConstant::NSString("AVEncoderBitRateKey"),
    ),
    (
        "_AVEncoderBitDepthHintKey",
        HostConstant::NSString("AVEncoderBitDepthHintKey"),
    ),
    (
        "_AVLinearPCMBitDepthKey",
        HostConstant::NSString("AVLinearPCMBitDepthKey"),
    ),
    (
        "_AVLinearPCMIsBigEndianKey",
        HostConstant::NSString("AVLinearPCMIsBigEndianKey"),
    ),
    (
        "_AVLinearPCMIsFloatKey",
        HostConstant::NSString("AVLinearPCMIsFloatKey"),
    ),
    (
        "_AVLinearPCMIsNonInterleaved",
        HostConstant::NSString("AVLinearPCMIsNonInterleaved"),
    ),
];

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/AVFoundation.framework/AVFoundation",
    aliases: &[],
    class_exports: &[
        av_audio_player::CLASSES,
        av_audio_session::CLASSES,
        av_capture::CLASSES,
    ],
    constant_exports: &[
        av_audio_session::CONSTANTS,
        av_capture::CONSTANTS,
        STUB_CONSTANTS,
    ],
    function_exports: &[],
};

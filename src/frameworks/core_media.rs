/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `CoreMedia.framework/CoreMedia`.
//!
//! On iOS, CoreMedia provides time-related types (`CMTime`, `CMTimeRange`),
//! sample buffer plumbing (`CMSampleBufferRef`), and format descriptions used
//! mostly by AVFoundation. Apps that link against CoreMedia (directly or
//! transitively, e.g. via AVFoundation cutscene playback) put the path
//! `/System/Library/Frameworks/CoreMedia.framework/CoreMedia` in their Mach-O
//! load commands.
//!
//! Without a [crate::dyld::HostDylib] entry for that path, touchHLE prints a
//! `Warning: app binary depends on unimplemented or missing dylib
//! "/System/Library/Frameworks/CoreMedia.framework/CoreMedia"` at startup,
//! which can spook users into reporting otherwise-fine apps as broken (e.g.
//! HyperHLE appdb report #22, GhostToasters).
//!
//! This stub exists so that the dependency is recognized and the warning is
//! suppressed. The few CoreMedia functions touchHLE currently implements
//! (`CMSampleBufferGetImageBuffer`, `CMSampleBufferDataIsReady`, …) are
//! registered with [crate::frameworks::core_video] for historical reasons;
//! `dyld` searches all framework `function_exports` regardless of which
//! dylib they were declared under, so the binding still resolves correctly
//! whether the app links CoreMedia or CoreVideo.

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};
use crate::mem::{ConstVoidPtr, MutPtr};
use crate::Environment;

/// `CMTime` is a 24-byte struct (`int64_t value`, `int32_t timescale`,
/// `uint32_t flags`, `int64_t epoch`). `kCMTimeZero` is the all-zero
/// constant — `{value:0, timescale:0, flags:0, epoch:0}`. Apps reach the
/// symbol by Mach-O lookup and either compare against it for equality
/// or copy it as a starting point. Allocate the layout once per access
/// site; CFTime's other "well-known" constants (kCMTimeInvalid,
/// kCMTimePositiveInfinity, …) fit the same template, so we expose
/// each as a separate slot.
fn cm_time_zero(env: &mut Environment) -> ConstVoidPtr {
    let p: MutPtr<u8> = env.mem.alloc(24).cast();
    for i in 0..24 {
        env.mem.write(p + i, 0);
    }
    p.cast().cast_const()
}

/// `kCMTimeInvalid` has flags=0 (kCMTimeFlags_Valid bit clear). Same
/// 24-byte layout as kCMTimeZero, every byte zero.
fn cm_time_invalid(env: &mut Environment) -> ConstVoidPtr {
    cm_time_zero(env)
}

/// `kCMTimeIndefinite`: value=0, timescale=0, flags=kCMTimeFlags_Valid|
/// kCMTimeFlags_Indefinite (=0x3), epoch=0. Apps usually only check for
/// (flags & kCMTimeFlags_Valid) and ignore the rest, so a flags-only
/// difference from kCMTimeZero is enough.
fn cm_time_indefinite(env: &mut Environment) -> ConstVoidPtr {
    let p: MutPtr<u8> = env.mem.alloc(24).cast();
    for i in 0..24 {
        env.mem.write(p + i, 0);
    }
    // Bytes 12..16 are the `flags` field (after value:i64, timescale:i32).
    env.mem.write(p + 12u32, 0x03);
    p.cast().cast_const()
}

/// `kCMTimePositiveInfinity` and `kCMTimeNegativeInfinity` use flags
/// `kCMTimeFlags_Valid|kCMTimeFlags_PositiveInfinity` (0x5) and
/// `kCMTimeFlags_Valid|kCMTimeFlags_NegativeInfinity` (0x9) respectively.
fn cm_time_positive_infinity(env: &mut Environment) -> ConstVoidPtr {
    let p: MutPtr<u8> = env.mem.alloc(24).cast();
    for i in 0..24 {
        env.mem.write(p + i, 0);
    }
    env.mem.write(p + 12u32, 0x05);
    p.cast().cast_const()
}

fn cm_time_negative_infinity(env: &mut Environment) -> ConstVoidPtr {
    let p: MutPtr<u8> = env.mem.alloc(24).cast();
    for i in 0..24 {
        env.mem.write(p + i, 0);
    }
    env.mem.write(p + 12u32, 0x09);
    p.cast().cast_const()
}

pub const CONSTANTS: ConstantExports = &[
    ("_kCMTimeZero", HostConstant::Custom(cm_time_zero)),
    ("_kCMTimeInvalid", HostConstant::Custom(cm_time_invalid)),
    (
        "_kCMTimeIndefinite",
        HostConstant::Custom(cm_time_indefinite),
    ),
    (
        "_kCMTimePositiveInfinity",
        HostConstant::Custom(cm_time_positive_infinity),
    ),
    (
        "_kCMTimeNegativeInfinity",
        HostConstant::Custom(cm_time_negative_infinity),
    ),
];

pub const FUNCTIONS: FunctionExports = &[];

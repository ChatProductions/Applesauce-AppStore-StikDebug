/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Crash-proof helpers for converting Apple time values into [`Duration`].
//!
//! Apple frameworks model durations and time intervals as `NSTimeInterval`
//! (a `double` measured in seconds) and `CFTimeInterval` (`CFAbsoluteTime`,
//! also a `double`). The official Apple documentation says these are simply
//! "double-precision floating-point values" — there is no requirement that
//! they be finite or non-negative. See:
//!
//! * `NSTimeInterval`:
//!   <https://developer.apple.com/documentation/foundation/nstimeinterval>
//! * `CFTimeInterval`:
//!   <https://developer.apple.com/documentation/corefoundation/cftimeinterval>
//!
//! Real-world iPhone apps (Unity-based games, NIB-driven apps that compute
//! intervals from animation curves, network code that hands `NaN` straight
//! into `[NSTimer scheduledTimerWithTimeInterval:...]`, etc.) routinely pass
//! `NaN`, ±∞, negative numbers and absurdly large numbers into these APIs.
//! On a real device the underlying CFRunLoop / dispatch_after either coerces
//! them to `0` (immediate fire) or quietly clamps them.
//!
//! Rust's [`Duration::from_secs_f64`] aborts the process on any of those
//! inputs (`>= u64::MAX seconds`, negative, NaN, or ±∞), which would crash
//! the whole emulator. The helpers in this module mirror Apple's "best
//! effort" behaviour by saturating instead of panicking.

use std::time::Duration;

/// Largest `NSTimeInterval` we'll faithfully convert. Anything larger
/// (including `f64::INFINITY`) is clamped to this. ~292 billion years is
/// already wildly beyond any realistic Apple API caller.
pub const MAX_SAFE_SECONDS: f64 = (u64::MAX / 2) as f64;

/// Convert an `NSTimeInterval` (seconds, may be negative / NaN / ±∞) into a
/// non-negative [`Duration`], saturating. This intentionally never returns
/// an error: callers like NSTimer/CADisplayLink/animation drivers always
/// need *some* duration, and the closest safe analogue to "fire immediately"
/// is `Duration::ZERO`. NaN and negative inputs both clamp to `ZERO`, which
/// matches CoreFoundation's documented behaviour for negative
/// `kCFRunLoopRunInMode` timeouts (treated as "poll").
pub fn duration_from_secs_f64_saturating(seconds: f64) -> Duration {
    if !seconds.is_finite() || seconds <= 0.0 {
        // Catches NaN, -∞, and any negative number.
        if seconds.is_finite() && seconds > 0.0 {
            // Defensive: should never hit this branch, but keeps the
            // contract honest if a future refactor changes the predicate.
            return Duration::ZERO;
        }
        if seconds == f64::INFINITY {
            return Duration::from_secs_f64(MAX_SAFE_SECONDS);
        }
        return Duration::ZERO;
    }
    let clamped = seconds.min(MAX_SAFE_SECONDS);
    // `try_from_secs_f64` is the panic-free variant added in Rust 1.66.
    Duration::try_from_secs_f64(clamped).unwrap_or(Duration::ZERO)
}

/// Variant that returns `None` for NaN / negative / non-finite inputs.
/// Useful at API boundaries where the caller wants to log+skip the call
/// (e.g. `[NSThread sleepForTimeInterval:NaN]`) instead of substituting
/// "fire immediately".
pub fn duration_from_secs_f64_strict(seconds: f64) -> Option<Duration> {
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    Duration::try_from_secs_f64(seconds.min(MAX_SAFE_SECONDS)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nan_and_infinities_saturate() {
        assert_eq!(duration_from_secs_f64_saturating(f64::NAN), Duration::ZERO);
        assert_eq!(
            duration_from_secs_f64_saturating(f64::NEG_INFINITY),
            Duration::ZERO
        );
        assert!(duration_from_secs_f64_saturating(f64::INFINITY) > Duration::from_secs(1));
    }

    #[test]
    fn negative_clamps_to_zero() {
        assert_eq!(duration_from_secs_f64_saturating(-1.0), Duration::ZERO);
        assert_eq!(duration_from_secs_f64_saturating(-0.0), Duration::ZERO);
    }

    #[test]
    fn normal_values_round_trip() {
        let d = duration_from_secs_f64_saturating(1.5);
        assert!((d.as_secs_f64() - 1.5).abs() < 1e-9);
    }

    #[test]
    fn strict_rejects_nan_and_negative() {
        assert!(duration_from_secs_f64_strict(f64::NAN).is_none());
        assert!(duration_from_secs_f64_strict(-0.5).is_none());
        assert!(duration_from_secs_f64_strict(f64::NEG_INFINITY).is_none());
        assert!(duration_from_secs_f64_strict(0.0).is_some());
    }
}

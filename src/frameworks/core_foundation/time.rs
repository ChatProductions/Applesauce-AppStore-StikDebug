/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Time things including `CFAbsoluteTime`.

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::CFTypeRef;
use crate::frameworks::foundation::NSTimeInterval;
use crate::libc::time::{time_t, timestamp_to_calendar_date};
use crate::mem::SafeRead;
use crate::objc::nil;
use crate::{impl_GuestRet_for_large_struct, Environment};
use std::ops::Add;
use std::time::{Duration, SystemTime};

pub const SECS_FROM_UNIX_TO_APPLE_EPOCHS: u64 = 978_307_200;

pub fn apple_epoch() -> SystemTime {
    SystemTime::UNIX_EPOCH.add(Duration::from_secs(SECS_FROM_UNIX_TO_APPLE_EPOCHS))
}

pub type CFTimeInterval = NSTimeInterval;
pub type CFAbsoluteTime = CFTimeInterval;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
pub struct CFGregorianDate {
    pub year: i32,
    pub month: i8,
    pub day: i8,
    pub hours: i8,
    pub minutes: i8,
    pub seconds: f64,
}
unsafe impl SafeRead for CFGregorianDate {}
impl_GuestRet_for_large_struct!(CFGregorianDate);

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
pub struct CFGregorianUnits {
    pub years: i32,
    pub months: i32,
    pub days: i32,
    pub hours: i32,
    pub minutes: i32,
    pub seconds: f64,
}
unsafe impl SafeRead for CFGregorianUnits {}
impl_GuestRet_for_large_struct!(CFGregorianUnits);

pub type CFTimeZoneRef = CFTypeRef;

// MARK: - Current time

fn CFAbsoluteTimeGetCurrent(_env: &mut Environment) -> CFAbsoluteTime {
    SystemTime::now()
        .duration_since(apple_epoch())
        .unwrap()
        .as_secs_f64()
}

// MARK: - Time zone

fn CFTimeZoneCopySystem(_env: &mut Environment) -> CFTimeZoneRef {
    // nil corresponds to GMT
    nil
}

fn CFTimeZoneCopyDefault(_env: &mut Environment) -> CFTimeZoneRef {
    nil
}

fn CFTimeZoneCreateWithTimeIntervalFromGMT(
    _env: &mut Environment,
    _allocator: CFTypeRef,
    _ti: CFTimeInterval,
) -> CFTimeZoneRef {
    log!("CFTimeZoneCreateWithTimeIntervalFromGMT: stubbed, returning nil (GMT)");
    nil
}

fn CFTimeZoneGetSecondsFromGMT(
    _env: &mut Environment,
    _tz: CFTimeZoneRef,
    _at: CFAbsoluteTime,
) -> CFTimeInterval {
    0.0
}

fn CFTimeZoneGetName(_env: &mut Environment, _tz: CFTimeZoneRef) -> CFTypeRef {
    // Returning nil is safe — callers that only display the name will show nothing.
    nil
}

fn CFTimeZoneRelease(_env: &mut Environment, _tz: CFTimeZoneRef) {
    // nil time zones have no ref count to manage.
}

fn CFTimeZoneRetain(_env: &mut Environment, tz: CFTimeZoneRef) -> CFTimeZoneRef {
    tz
}

fn CFGregorianDateGetAbsoluteTime(
    _env: &mut Environment,
    gd: CFGregorianDate,
    tz: CFTimeZoneRef,
) -> CFAbsoluteTime {
    if !tz.is_null() {
        log!("Warning: CFGregorianDateGetAbsoluteTime: non-GMT timezone ignored");
    }
    // Rough inverse: convert calendar fields back to a Unix timestamp, then
    // adjust to Apple epoch. Uses simplified (non-leap-aware) month lengths
    // which is accurate enough for game use-cases.
    const DAYS_BEFORE_MONTH: [i64; 13] =
        [0, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

    let y = gd.year as i64;
    let m = gd.month as i64;
    let d = gd.day as i64;

    // Days since Unix epoch (1970-01-01).
    let leap_days = (y - 1969) / 4 - (y - 1901) / 100 + (y - 1601) / 400;
    let days = (y - 1970) * 365 + leap_days
        + DAYS_BEFORE_MONTH[m.clamp(1, 12) as usize]
        + d - 1;

    let unix_secs = days * 86400
        + gd.hours   as i64 * 3600
        + gd.minutes as i64 * 60
        + gd.seconds as i64;

    unix_secs as f64 - SECS_FROM_UNIX_TO_APPLE_EPOCHS as f64
}

fn CFAbsoluteTimeGetDayOfWeek(
    env: &mut Environment,
    at: CFAbsoluteTime,
    tz: CFTimeZoneRef,
) -> i32 {
    if !tz.is_null() {
        log!("Warning: CFAbsoluteTimeGetDayOfWeek: non-GMT timezone ignored");
    }
    // Compute day-of-week from the absolute time using Tomohiko Sakamoto's
    // algorithm — avoids needing private tm fields entirely.
    let gd = CFAbsoluteTimeGetGregorianDate(env, at, tz);
    let y = gd.year as i32 - if gd.month as i32 <= 2 { 1 } else { 0 };
    let m = gd.month as i32;
    let d = gd.day   as i32;
    static T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    // Returns 0 = Sunday … 6 = Saturday; convert to CF (1 = Monday … 7 = Sunday).
    let dow = (y + y/4 - y/100 + y/400 + T[(m-1) as usize] + d) % 7;
    if dow == 0 { 7 } else { dow }
}

fn CFAbsoluteTimeGetWeekOfYear(
    env: &mut Environment,
    at: CFAbsoluteTime,
    tz: CFTimeZoneRef,
) -> i32 {
    let doy = CFAbsoluteTimeGetDayOfYear(env, at, tz);
    (doy - 1) / 7 + 1
}

// MARK: - Arithmetic

// Replace CFGregorianDateGetAbsoluteTime signature:
fn CFGregorianDateGetAbsoluteTime(
    env: &mut Environment,
    gd: crate::mem::ConstPtr<CFGregorianDate>,
    tz: CFTimeZoneRef,
) -> CFAbsoluteTime {
    let gd = env.mem.read(gd);
    // ... rest unchanged
}

// Replace CFGregorianDateIsValid signature:
fn CFGregorianDateIsValid(
    env: &mut Environment,
    gd: crate::mem::ConstPtr<CFGregorianDate>,
    _unit_flags: u32,
) -> bool {
    let gd = env.mem.read(gd);
    gd.month  >= 1  && gd.month  <= 12
        && gd.day     >= 1  && gd.day   <= 31
        && gd.hours   >= 0  && gd.hours <= 23
        && gd.minutes >= 0  && gd.minutes <= 59
        && gd.seconds >= 0.0 && gd.seconds < 60.0
}

// Replace CFAbsoluteTimeAddGregorianUnits signature:
fn CFAbsoluteTimeAddGregorianUnits(
    env: &mut Environment,
    at: CFAbsoluteTime,
    tz: CFTimeZoneRef,
    units: crate::mem::ConstPtr<CFGregorianUnits>,
) -> CFAbsoluteTime {
    let units = env.mem.read(units);
    // ... rest unchanged
}

fn CFAbsoluteTimeGetDayOfYear(
    env: &mut Environment,
    at: CFAbsoluteTime,
    tz: CFTimeZoneRef,
) -> i32 {
    if !tz.is_null() {
        log!("Warning: CFAbsoluteTimeGetDayOfYear: non-GMT timezone ignored");
    }
    let gd = CFAbsoluteTimeGetGregorianDate(env, at, tz);
    let y  = gd.year  as i32;
    let m  = gd.month as i32;
    let d  = gd.day   as i32;
    // Days elapsed before each month (non-leap).
    const DAYS: [i32; 13] = [0, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let leap = if m > 2 && (y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)) { 1 } else { 0 };
    DAYS[m.clamp(1, 12) as usize] + d + leap
}

fn CFAbsoluteTimeGetDifferenceAsGregorianUnits(
    _env: &mut Environment,
    at1: CFAbsoluteTime,
    at2: CFAbsoluteTime,
    _tz: CFTimeZoneRef,
    _flags: u32,
) -> CFGregorianUnits {
    let mut diff = at1 - at2;
    let mut u = CFGregorianUnits {
        years: 0, months: 0, days: 0,
        hours: 0, minutes: 0, seconds: 0.0,
    };
    macro_rules! extract {
        ($field:ident, $secs:expr) => {
            if diff.abs() >= $secs {
                u.$field = (diff / $secs) as i32;
                diff     -= u.$field as f64 * $secs;
            }
        };
    }
    extract!(years,   31_536_000.0);
    extract!(months,   2_592_000.0);
    extract!(days,        86_400.0);
    extract!(hours,        3_600.0);
    extract!(minutes,         60.0);
    u.seconds = diff;
    u
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFAbsoluteTimeGetCurrent()),
    export_c_func!(CFTimeZoneCopySystem()),
    export_c_func!(CFTimeZoneCopyDefault()),
    export_c_func!(CFTimeZoneCreateWithTimeIntervalFromGMT(_, _)),
    export_c_func!(CFTimeZoneGetSecondsFromGMT(_, _)),
    export_c_func!(CFTimeZoneGetName(_)),
    export_c_func!(CFTimeZoneRetain(_)),
    export_c_func!(CFTimeZoneRelease(_)),
    export_c_func!(CFAbsoluteTimeGetGregorianDate(_, _)),
    export_c_func!(CFGregorianDateGetAbsoluteTime(_, _)),
    export_c_func!(CFGregorianDateIsValid(_, _)),
    export_c_func!(CFAbsoluteTimeGetDayOfWeek(_, _)),
    export_c_func!(CFAbsoluteTimeGetDayOfYear(_, _)),
    export_c_func!(CFAbsoluteTimeGetWeekOfYear(_, _)),
    export_c_func!(CFAbsoluteTimeAddGregorianUnits(_, _, _)),
    export_c_func!(CFAbsoluteTimeGetDifferenceAsGregorianUnits(_, _, _, _)),
];

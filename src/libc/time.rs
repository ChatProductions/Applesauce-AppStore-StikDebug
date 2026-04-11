/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `time.h` (C) and `sys/time.h` (POSIX)

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::clocale::{setlocale, LC_CTYPE};
use crate::libc::errno::set_errno;
use crate::libc::stdio::printf::{isspace, isspace_inner};
use crate::mem::{guest_size_of, ConstPtr, GuestUSize, MutPtr, Ptr, SafeRead};
use crate::Environment;
use std::ops::Range;
use std::time::{Duration, Instant, SystemTime};

#[derive(Default)]
pub struct State {
    /// Temporary static storage for the return value of `gmtime` or
    /// `localtime`. The standard allows calls to either to overwrite it.
    gmtime_tmp: Option<MutPtr<tm>>,
    /// Temporary static storage for `asctime` / `ctime`.
    asctime_buf: Option<MutPtr<u8>>,
}

// =========================================================================
// MARK: - time.h types
// =========================================================================

#[allow(non_camel_case_types)]
pub type time_t = i32;

#[allow(non_camel_case_types)]
type clock_t = u64;

const CLOCKS_PER_SEC: clock_t = 1_000_000;

fn clock(env: &mut Environment) -> clock_t {
    Instant::now()
        .duration_since(env.startup_time)
        .as_secs()
        .wrapping_mul(CLOCKS_PER_SEC)
}

fn time(env: &mut Environment, out: MutPtr<time_t>) -> time_t {
    set_errno(env, 0);
    let time64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let t = time64 as time_t;
    if time64 != t as u64 {
        log_once!("Warning: [time] system clock is beyond Y2K38 and might confuse the app");
    }
    if !out.is_null() {
        env.mem.write(out, t);
    }
    t
}

fn tzset(_env: &mut Environment) {
    log!("TODO: tzset()");
}

#[allow(non_camel_case_types)]
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct tm {
    pub tm_sec:   i32,
    pub tm_min:   i32,
    pub tm_hour:  i32,
    pub tm_mday:  i32,
    pub tm_mon:   i32,
    pub tm_year:  i32,
    pub tm_wday:  i32,
    pub tm_yday:  i32,
    pub tm_isdst: i32,
    pub tm_gmtoff: i32,
    pub tm_zone:  ConstPtr<u8>,
}
unsafe impl SafeRead for tm {}

impl tm {
    pub fn from(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        tm {
            tm_year:  (year - 1900).into(),
            tm_mon:   (month - 1).into(),
            tm_mday:  day.into(),
            tm_hour:  hour.into(),
            tm_min:   minute.into(),
            tm_sec:   second.into(),
            tm_wday:  0,
            tm_yday:  0,
            tm_isdst: 0,
            tm_gmtoff: 0,
            tm_zone:  Ptr::null(),
        }
    }
}

// =========================================================================
// MARK: - Calendar helpers
// =========================================================================

const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}
const CYCLE_YEARS: i32 = 400;
const YEAR_TO_DAY: [i32; CYCLE_YEARS as usize] = calc_year_to_day().0;
const CYCLE_DAYS: i32 = calc_year_to_day().1;
const fn calc_year_to_day() -> ([i32; CYCLE_YEARS as usize], i32) {
    let mut table = [0i32; CYCLE_YEARS as usize];
    let mut day = 0;
    let mut year = 0;
    while year < CYCLE_YEARS {
        table[year as usize] = day;
        day += if is_leap_year(year) { 366 } else { 365 };
        year += 1;
    }
    (table, day)
}
const DAYS_IN_MONTH: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const MONTH_TO_DAY_NONLEAP: [i32; 12] = calc_month_to_day(false);
const MONTH_TO_DAY_LEAP:    [i32; 12] = calc_month_to_day(true);
const fn calc_month_to_day(leap_year: bool) -> [i32; 12] {
    let mut table = [0i32; 12];
    let mut day = 0;
    let mut month = 0;
    while month < 12 {
        table[month] = day;
        day += DAYS_IN_MONTH[month] + ((leap_year && month == 1) as i32);
        month += 1;
    }
    table
}

pub fn timestamp_to_calendar_date(timestamp: time_t) -> tm {
    const MINUTE_SECONDS: i32 = 60;
    const HOUR_SECONDS:   i32 = MINUTE_SECONDS * 60;
    const DAY_SECONDS:    i32 = HOUR_SECONDS * 24;

    let seconds_since_unix_epoch: i32 = timestamp;
    let days_since_unix_epoch = seconds_since_unix_epoch.div_euclid(DAY_SECONDS);
    let second_in_day         = seconds_since_unix_epoch.rem_euclid(DAY_SECONDS);

    let tm_sec  = second_in_day % MINUTE_SECONDS;
    let tm_min  = (second_in_day % HOUR_SECONDS) / MINUTE_SECONDS;
    let tm_hour = second_in_day / HOUR_SECONDS;

    let days_since_y2k  = days_since_unix_epoch - 10957;
    let cycles_since_y2k = days_since_y2k.div_euclid(CYCLE_DAYS);
    let day_in_cycle     = days_since_y2k.rem_euclid(CYCLE_DAYS);

    let year_in_cycle: i32 = (YEAR_TO_DAY.partition_point(|&day| day <= day_in_cycle) - 1) as _;
    let year = 2000 + cycles_since_y2k * CYCLE_YEARS + year_in_cycle;
    let day_in_year = day_in_cycle - YEAR_TO_DAY[usize::try_from(year_in_cycle).unwrap()];
    let is_leap = is_leap_year(year_in_cycle);

    let month_to_day = if is_leap { &MONTH_TO_DAY_LEAP } else { &MONTH_TO_DAY_NONLEAP };
    let month_in_year: i32 = (month_to_day.partition_point(|&day| day <= day_in_year) - 1) as _;
    let day_in_month = day_in_year - month_to_day[usize::try_from(month_in_year).unwrap()];

    let day_of_the_week = (4 + days_since_unix_epoch).rem_euclid(7);

    tm {
        tm_sec, tm_min, tm_hour,
        tm_mday: day_in_month + 1,
        tm_mon:  month_in_year,
        tm_year: year - 1900,
        tm_wday: day_of_the_week,
        tm_yday: day_in_year,
        tm_isdst: 0, tm_gmtoff: 0,
        tm_zone: Ptr::null(),
    }
}

pub fn calendar_date_to_timestamp(tm: tm) -> time_t {
    let year = tm.tm_year + 1900;
    let mut seconds = 0i64;
    for y in 1970..year {
        seconds += if is_leap_year(y) { 366 } else { 365 };
    }
    seconds *= 86400;
    let mtd = if is_leap_year(year) {
        MONTH_TO_DAY_LEAP[tm.tm_mon as usize]
    } else {
        MONTH_TO_DAY_NONLEAP[tm.tm_mon as usize]
    };
    seconds += mtd as i64 * 86400;
    seconds += (tm.tm_mday as i64 - 1) * 86400;
    seconds += tm.tm_hour as i64 * 3600;
    seconds += tm.tm_min  as i64 * 60;
    seconds += tm.tm_sec  as i64;
    if year < 1970 {
        let mut days_before = 0i64;
        for y in year..1970 {
            days_before += if is_leap_year(y) { 366 } else { 365 };
        }
        seconds -= days_before * 86400;
    }
    seconds.try_into().unwrap()
}

// =========================================================================
// MARK: - gmtime / localtime / mktime
// =========================================================================

fn gmtime_r(env: &mut Environment, timestamp: ConstPtr<time_t>, res: MutPtr<tm>) -> MutPtr<tm> {
    let t = env.mem.read(timestamp);
    env.mem.write(res, timestamp_to_calendar_date(t));
    res
}

fn gmtime(env: &mut Environment, timestamp: ConstPtr<time_t>) -> MutPtr<tm> {
    let tmp = *env.libc_state.time.gmtime_tmp
        .get_or_insert_with(|| env.mem.alloc(guest_size_of::<tm>()).cast());
    gmtime_r(env, timestamp, tmp)
}

fn localtime_r(env: &mut Environment, timestamp: ConstPtr<time_t>, res: MutPtr<tm>) -> MutPtr<tm> {
    gmtime_r(env, timestamp, res)
}

fn localtime(env: &mut Environment, timestamp: ConstPtr<time_t>) -> MutPtr<tm> {
    gmtime(env, timestamp)
}

fn mktime(env: &mut Environment, tm: MutPtr<tm>) -> time_t {
    let tm_value = env.mem.read(tm);
    let res = calendar_date_to_timestamp(tm_value);
    log_dbg!("mktime({:?}) => {}", tm_value, res);
    res
}

/// `time_t timegm(struct tm *tm)` — like mktime but always treats the input
/// as UTC (GNU extension, widely used on Darwin).
fn timegm(env: &mut Environment, tm: MutPtr<tm>) -> time_t {
    mktime(env, tm)
}

/// `double difftime(time_t time1, time_t time0)` — signed difference in seconds.
fn difftime(_env: &mut Environment, time1: time_t, time0: time_t) -> f64 {
    (time1 as f64) - (time0 as f64)
}

// =========================================================================
// MARK: - asctime / ctime
// =========================================================================

const WDAY_ABBREV: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MON_ABBREV:  [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn asctime_format(t: &tm) -> Vec<u8> {
    // "Www Mmm DD HH:MM:SS YYYY\n\0"  (26 bytes per POSIX)
    let wday = WDAY_ABBREV.get(t.tm_wday as usize).copied().unwrap_or("???");
    let mon  = MON_ABBREV .get(t.tm_mon  as usize).copied().unwrap_or("???");
    let s = format!(
        "{} {} {:2} {:02}:{:02}:{:02} {:04}\n",
        wday, mon,
        t.tm_mday,
        t.tm_hour, t.tm_min, t.tm_sec,
        t.tm_year + 1900
    );
    let mut bytes: Vec<u8> = s.into_bytes();
    bytes.push(0); // NUL terminator
    bytes
}

fn asctime_r(env: &mut Environment, tm_ptr: ConstPtr<tm>, buf: MutPtr<u8>) -> MutPtr<u8> {
    let t = env.mem.read(tm_ptr);
    let s = asctime_format(&t);
    let dst = env.mem.bytes_at_mut(buf, s.len() as u32);
    dst.copy_from_slice(&s);
    buf
}

fn asctime(env: &mut Environment, tm_ptr: ConstPtr<tm>) -> ConstPtr<u8> {
    // Use a static 26-byte buffer (reused across calls, per C standard).
    let buf = *env.libc_state.time.asctime_buf
        .get_or_insert_with(|| env.mem.alloc(26).cast());
    asctime_r(env, tm_ptr, buf);
    buf.cast_const()
}

fn ctime_r(env: &mut Environment, timestamp: ConstPtr<time_t>, buf: MutPtr<u8>) -> MutPtr<u8> {
    let t = env.mem.read(timestamp);
    let tmp_ptr: MutPtr<time_t> = env.mem.alloc(guest_size_of::<time_t>()).cast();
    env.mem.write(tmp_ptr, t);
    let tm_ptr = gmtime(env, tmp_ptr.cast_const());
    asctime_r(env, tm_ptr.cast_const(), buf)
}

fn ctime(env: &mut Environment, timestamp: ConstPtr<time_t>) -> ConstPtr<u8> {
    let buf = *env.libc_state.time.asctime_buf
        .get_or_insert_with(|| env.mem.alloc(26).cast());
    ctime_r(env, timestamp, buf);
    buf.cast_const()
}

// =========================================================================
// MARK: - sys/time.h
// =========================================================================

#[allow(non_camel_case_types)]
type suseconds_t = i32;

#[allow(non_camel_case_types)]
#[derive(Debug)]
#[repr(C, packed)]
pub(super) struct timeval {
    pub(super) tv_sec:  time_t,
    pub(super) tv_usec: suseconds_t,
}
unsafe impl SafeRead for timeval {}

#[allow(non_camel_case_types)]
#[derive(Default)]
#[repr(C, packed)]
pub struct timespec {
    pub tv_sec:  time_t,
    pub tv_nsec: i32,
}
unsafe impl SafeRead for timespec {}

#[allow(non_camel_case_types)]
#[repr(C, packed)]
struct timezone {
    tz_minuteswest: i32,
    tz_dsttime:     i32,
}
unsafe impl SafeRead for timezone {}

fn gettimeofday(
    env: &mut Environment,
    timeval_ptr: MutPtr<timeval>,
    timezone_ptr: MutPtr<timezone>,
) -> i32 {
    set_errno(env, 0);
    if !timezone_ptr.is_null() {
        env.mem.write(timezone_ptr, timezone { tz_minuteswest: 0, tz_dsttime: 0 });
    }
    if timeval_ptr.is_null() {
        return 0;
    }
    let t = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let time_s_64 = t.as_secs();
    let tv_sec = time_s_64 as time_t;
    if time_s_64 != tv_sec as u64 {
        log_once!("Warning: [gettimeofday] system clock is beyond Y2K38");
    }
    let tv_usec: suseconds_t = t.subsec_micros().try_into().unwrap();
    env.mem.write(timeval_ptr, timeval { tv_sec, tv_usec });
    0
}

/// `int settimeofday(const struct timeval*, const struct timezone*)` — no-op stub.
fn settimeofday(
    _env: &mut Environment,
    _tv: ConstPtr<timeval>,
    _tz: ConstPtr<timezone>,
) -> i32 {
    log!("TODO: settimeofday() — ignored, returning 0");
    0
}

/// `int clock_gettime(clockid_t clk_id, struct timespec *tp)`
fn clock_gettime(
    env: &mut Environment,
    clk_id: i32,
    tp: MutPtr<timespec>,
) -> i32 {
    set_errno(env, 0);
    if tp.is_null() {
        return -1;
    }
    let (secs, nanos): (i64, i32) = match clk_id {
        0 /* CLOCK_REALTIME */ => {
            let t = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH).unwrap();
            (t.as_secs() as i64, t.subsec_nanos() as i32)
        }
        1 /* CLOCK_MONOTONIC */ | 4 /* CLOCK_MONOTONIC_RAW */ => {
            let t = Instant::now().duration_since(env.startup_time);
            (t.as_secs() as i64, t.subsec_nanos() as i32)
        }
        _ => {
            log!("clock_gettime: unsupported clk_id={}", clk_id);
            return -1;
        }
    };
    env.mem.write(tp, timespec { tv_sec: secs as time_t, tv_nsec: nanos });
    0
}

/// `int clock_getres(clockid_t clk_id, struct timespec *res)`
fn clock_getres(
    env: &mut Environment,
    _clk_id: i32,
    res: MutPtr<timespec>,
) -> i32 {
    if !res.is_null() {
        // Report 1 microsecond resolution — conservative and safe.
        env.mem.write(res, timespec { tv_sec: 0, tv_nsec: 1_000 });
    }
    0
}

/// `int clock_settime(clockid_t, const struct timespec*)` — no-op stub.
fn clock_settime(
    _env: &mut Environment,
    clk_id: i32,
    _tp: ConstPtr<timespec>,
) -> i32 {
    log!("TODO: clock_settime(clk_id={}) — ignored", clk_id);
    0
}

fn nanosleep(env: &mut Environment, rqtp: ConstPtr<timespec>, _rmtp: MutPtr<timespec>) -> i32 {
    set_errno(env, 0);
    let t = env.mem.read(rqtp);
    log_dbg!("nanosleep {}.{:09}", t.tv_sec, t.tv_nsec);
    let total = Duration::from_secs(t.tv_sec.try_into().unwrap_or(0))
        + Duration::from_nanos(t.tv_nsec.try_into().unwrap_or(0));
    env.sleep(total);
    0
}

// =========================================================================
// MARK: - strptime
// =========================================================================

fn strptime(
    env: &mut Environment,
    buffer: ConstPtr<u8>,
    format: ConstPtr<u8>,
    time_ptr: MutPtr<tm>,
) -> MutPtr<u8> {
    log_dbg!(
        "strptime({:?}, {:?})",
        env.mem.cstr_at_utf8(buffer),
        env.mem.cstr_at_utf8(format)
    );

    let mut time_val = env.mem.read(time_ptr);
    let mut conversation_failed = false;
    let mut buffer_char_idx = 0u32;
    let mut format_char_idx = 0u32;

    loop {
        let c = env.mem.read(format + format_char_idx);
        format_char_idx += 1;
        if c == b'\0' { break; }

        if c != b'%' {
            let mut cc = env.mem.read(buffer + buffer_char_idx);
            if isspace(env, format + format_char_idx - 1) {
                while isspace_inner(cc) {
                    buffer_char_idx += 1;
                    cc = env.mem.read(buffer + buffer_char_idx);
                }
                continue;
            }
            if c != cc { conversation_failed = true; break; }
            buffer_char_idx += 1;
            continue;
        }

        let specifier = env.mem.read(format + format_char_idx);
        format_char_idx += 1;

        // Parse up to `max_digits` decimal digits, clamped to `range`.
        let mut parse_digits = |max_digits: u32, range: Range<i32>| -> Result<i32, ()> {
            let mut num: i32 = 0;
            let mut count = 0u32;
            // Skip leading whitespace.
            while isspace_inner(env.mem.read(buffer + buffer_char_idx)) {
                buffer_char_idx += 1;
            }
            while count < max_digits {
                match env.mem.read(buffer + buffer_char_idx) {
                    c @ b'0'..=b'9' => {
                        num = num * 10 + (c - b'0') as i32;
                        buffer_char_idx += 1;
                        count += 1;
                    }
                    _ => break,
                }
            }
            if count == 0 { return Err(()); }
            if range.contains(&num) { Ok(num) } else { Err(()) }
        };

        match specifier {
            b'H' => match parse_digits(2, 0..24) {
                Ok(v) => time_val.tm_hour = v,
                Err(_) => { conversation_failed = true; break; }
            },
            b'I' => match parse_digits(2, 1..13) {
                // 12-hour clock — assume AM for now.
                Ok(v) => time_val.tm_hour = v % 12,
                Err(_) => { conversation_failed = true; break; }
            },
            b'M' => match parse_digits(2, 0..60) {
                Ok(v) => time_val.tm_min = v,
                Err(_) => { conversation_failed = true; break; }
            },
            b'S' => match parse_digits(2, 0..61) {
                Ok(v) => time_val.tm_sec = v,
                Err(_) => { conversation_failed = true; break; }
            },
            b'd' | b'e' => match parse_digits(2, 1..32) {
                Ok(v) => time_val.tm_mday = v,
                Err(_) => { conversation_failed = true; break; }
            },
            b'm' => match parse_digits(2, 1..13) {
                Ok(v) => time_val.tm_mon = v - 1,
                Err(_) => { conversation_failed = true; break; }
            },
            b'Y' => match parse_digits(4, 0..9999) {
                Ok(v) => time_val.tm_year = v - 1900,
                Err(_) => { conversation_failed = true; break; }
            },
            b'y' => match parse_digits(2, 0..100) {
                Ok(v) => time_val.tm_year = if v >= 69 { v } else { v + 100 },
                Err(_) => { conversation_failed = true; break; }
            },
            b'j' => match parse_digits(3, 1..367) {
                Ok(v) => time_val.tm_yday = v - 1,
                Err(_) => { conversation_failed = true; break; }
            },
            b'n' | b't' => {
                // Match any whitespace.
                while isspace_inner(env.mem.read(buffer + buffer_char_idx)) {
                    buffer_char_idx += 1;
                }
            }
            b'%' => {
                if env.mem.read(buffer + buffer_char_idx) != b'%' {
                    conversation_failed = true; break;
                }
                buffer_char_idx += 1;
            }
            _ => {
                log!("strptime: unhandled specifier '%{}'", specifier as char);
                conversation_failed = true;
                break;
            }
        }
    }

    env.mem.write(time_ptr, time_val);
    if conversation_failed { Ptr::null() } else { (buffer + buffer_char_idx).cast_mut() }
}

// =========================================================================
// MARK: - strftime
// =========================================================================

fn strftime(
    env: &mut Environment,
    s: MutPtr<u8>,
    max_size: GuestUSize,
    format: ConstPtr<u8>,
    time_ptr: ConstPtr<tm>,
) -> GuestUSize {
    log_dbg!(
        "strftime({:?}, {}, {:?}, {:?})",
        s, max_size,
        env.mem.cstr_at_utf8(format),
        time_ptr
    );

    let ctype_locale = setlocale(env, LC_CTYPE, Ptr::null());
    assert_eq!(env.mem.read(ctype_locale), b'C');

    let t = env.mem.read(time_ptr);
    let mut res = Vec::<u8>::new();
    let mut format_char_idx = 0u32;

    loop {
        let c = env.mem.read(format + format_char_idx);
        format_char_idx += 1;
        if c == b'\0' { break; }
        if c != b'%' { res.push(c); continue; }

        let specifier = env.mem.read(format + format_char_idx);
        format_char_idx += 1;

        match specifier {
            b'%' => res.push(b'%'),
            b'Y' => res.extend_from_slice(format!("{:04}", t.tm_year + 1900).as_bytes()),
            b'y' => res.extend_from_slice(format!("{:02}", (t.tm_year + 1900) % 100).as_bytes()),
            b'C' => res.extend_from_slice(format!("{:02}", (t.tm_year + 1900) / 100).as_bytes()),
            b'm' => res.extend_from_slice(format!("{:02}", t.tm_mon + 1).as_bytes()),
            b'd' => res.extend_from_slice(format!("{:02}", t.tm_mday).as_bytes()),
            b'e' => res.extend_from_slice(format!("{:2}",  t.tm_mday).as_bytes()),
            b'H' => res.extend_from_slice(format!("{:02}", t.tm_hour).as_bytes()),
            b'I' => {
                let h = t.tm_hour % 12;
                res.extend_from_slice(format!("{:02}", if h == 0 { 12 } else { h }).as_bytes());
            }
            b'M' => res.extend_from_slice(format!("{:02}", t.tm_min).as_bytes()),
            b'S' => res.extend_from_slice(format!("{:02}", t.tm_sec).as_bytes()),
            b'j' => res.extend_from_slice(format!("{:03}", t.tm_yday + 1).as_bytes()),
            b'u' => res.push(b'0' + (((t.tm_wday + 6) % 7) + 1) as u8), // Mon=1..Sun=7
            b'w' => res.push(b'0' + t.tm_wday as u8),                    // Sun=0..Sat=6
            b'A' => {
                let s = ["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"];
                res.extend_from_slice(s.get(t.tm_wday as usize).unwrap_or(&"?").as_bytes());
            }
            b'a' => {
                res.extend_from_slice(WDAY_ABBREV.get(t.tm_wday as usize).unwrap_or(&"???").as_bytes());
            }
            b'B' => {
                let s = ["January","February","March","April","May","June",
                         "July","August","September","October","November","December"];
                res.extend_from_slice(s.get(t.tm_mon as usize).unwrap_or(&"?").as_bytes());
            }
            b'b' | b'h' => {
                res.extend_from_slice(MON_ABBREV.get(t.tm_mon as usize).unwrap_or(&"???").as_bytes());
            }
            b'p' => res.extend_from_slice(if t.tm_hour < 12 { b"AM" } else { b"PM" }),
            b'P' => res.extend_from_slice(if t.tm_hour < 12 { b"am" } else { b"pm" }),
            b'Z' => res.extend_from_slice(b"UTC"),
            b'z' => res.extend_from_slice(b"+0000"),
            b'n' => res.push(b'\n'),
            b't' => res.push(b'\t'),
            // Composite specifiers
            b'D' => { // %m/%d/%y
                res.extend_from_slice(format!("{:02}/{:02}/{:02}",
                    t.tm_mon + 1, t.tm_mday, (t.tm_year + 1900) % 100).as_bytes());
            }
            b'F' => { // %Y-%m-%d
                res.extend_from_slice(format!("{:04}-{:02}-{:02}",
                    t.tm_year + 1900, t.tm_mon + 1, t.tm_mday).as_bytes());
            }
            b'T' => { // %H:%M:%S
                res.extend_from_slice(format!("{:02}:{:02}:{:02}",
                    t.tm_hour, t.tm_min, t.tm_sec).as_bytes());
            }
            b'R' => { // %H:%M
                res.extend_from_slice(format!("{:02}:{:02}", t.tm_hour, t.tm_min).as_bytes());
            }
            b'r' => { // %I:%M:%S %p
                let h = t.tm_hour % 12;
                let h12 = if h == 0 { 12 } else { h };
                let ampm = if t.tm_hour < 12 { "AM" } else { "PM" };
                res.extend_from_slice(format!("{:02}:{:02}:{:02} {}",
                    h12, t.tm_min, t.tm_sec, ampm).as_bytes());
            }
            b'c' => { // locale date+time: "Www Mmm DD HH:MM:SS YYYY"
                let bytes = asctime_format(&t);
                // asctime_format appends \n\0 — strip those for %c
                res.extend_from_slice(&bytes[..bytes.len().saturating_sub(2)]);
            }
            b'x' => { // locale date: %m/%d/%y
                res.extend_from_slice(format!("{:02}/{:02}/{:02}",
                    t.tm_mon + 1, t.tm_mday, (t.tm_year + 1900) % 100).as_bytes());
            }
            b'X' => { // locale time: %H:%M:%S
                res.extend_from_slice(format!("{:02}:{:02}:{:02}",
                    t.tm_hour, t.tm_min, t.tm_sec).as_bytes());
            }
            _ => {
                log!("strftime: unhandled specifier '%{}'", specifier as char);
                res.push(b'%');
                res.push(specifier);
            }
        }
    }

    if max_size == 0 { return 0; }
    let copy_len = res.len().min((max_size - 1) as usize);
    let dst = env.mem.bytes_at_mut(s, max_size);
    dst[..copy_len].copy_from_slice(&res[..copy_len]);
    dst[copy_len] = 0;
    res.len().try_into().unwrap()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(clock()),
    export_c_func!(time(_)),
    export_c_func!(tzset()),
    export_c_func!(gmtime_r(_, _)),
    export_c_func!(gmtime(_)),
    export_c_func!(localtime_r(_, _)),
    export_c_func!(localtime(_)),
    export_c_func!(mktime(_)),
    export_c_func!(timegm(_)),
    export_c_func!(difftime(_, _)),
    export_c_func!(asctime(_)),
    export_c_func!(asctime_r(_, _)),
    export_c_func!(ctime(_)),
    export_c_func!(ctime_r(_, _)),
    export_c_func!(gettimeofday(_, _)),
    export_c_func!(settimeofday(_, _)),
    export_c_func!(clock_gettime(_, _)),
    export_c_func!(clock_getres(_, _)),
    export_c_func!(clock_settime(_, _)),
    export_c_func!(nanosleep(_, _)),
    export_c_func!(strptime(_, _, _)),
    export_c_func!(strftime(_, _, _, _)),
];

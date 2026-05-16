/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `math.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::set_errno;
use crate::mem::{ConstPtr, MutPtr};
use crate::Environment;
use std::num::FpCategory;

// TODO: move to `fenv.h`
type FERoundingDirection = i32;
const FE_TONEAREST: FERoundingDirection = 0x000000;
const FE_TOWARDZERO: FERoundingDirection = 0xc00000;

#[derive(Default)]
pub struct State {
    rounding_direction: FERoundingDirection,
    timer_manager_instance: u32,
}

// The sections in this file are organized to match the C standard.

// FIXME: Many functions in this file should theoretically set errno or affect
//        the floating-point environment. We're hoping apps won't rely on that.

fn abs(_env: &mut Environment, arg: i32) -> i32 {
    arg.abs()
}
fn fabs(_env: &mut Environment, arg: f64) -> f64 {
    arg.abs()
}

// Trigonometric functions

// TODO: These should also have `long double` variants, which can probably just
// alias the `double` ones.

fn sin(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.sin()
}
fn sinf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.sin()
}
fn cos(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.cos()
}
fn cosf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.cos()
}
fn tan(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.tan()
}
fn tanf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.tan()
}

fn asin(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.asin()
}
fn asinf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.asin()
}
fn acos(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.acos()
}
fn acosf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.acos()
}
fn atan(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.atan()
}
fn atanf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.atan()
}

fn atan2f(env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg1.atan2(arg2)
}
fn atan2(env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg1.atan2(arg2)
}

// Hyperbolic functions

fn sinh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.sinh()
}
fn sinhf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.sinh()
}
fn cosh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.cosh()
}
fn coshf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.cosh()
}
fn tanh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.tanh()
}
fn tanhf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.tanh()
}

fn asinh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.asinh()
}
fn asinhf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.asinh()
}
fn acosh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.acosh()
}
fn acoshf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.acosh()
}
fn atanh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.atanh()
}
fn atanhf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.atanh()
}

// Exponential and logarithmic functions
// TODO: implement the rest
fn log(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.ln()
}
fn logf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.ln()
}
fn log1p(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.ln_1p()
}
fn log1pf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.ln_1p()
}
fn log2(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.log2()
}
fn log2f(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.log2()
}
fn log10(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.log10()
}
fn log10f(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.log10()
}
fn exp(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.exp()
}
fn expf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.exp()
}
fn expm1(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.exp_m1()
}
fn expm1f(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.exp_m1()
}
fn exp2(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.exp2()
}
fn exp2f(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.exp2()
}
fn ldexp(env: &mut Environment, arg: f64, n: i32) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    assert!(!arg.is_infinite()); // TODO

    arg * 2f64.powf(n as _)
}
fn ldexpf(env: &mut Environment, arg: f32, n: i32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    assert!(!arg.is_infinite()); // TODO

    arg * 2f32.powf(n as _)
}
fn frexpf(env: &mut Environment, arg: f32, exp: MutPtr<i32>) -> f32 {
    frexp(env, arg.into(), exp) as f32
}
fn frexp(env: &mut Environment, arg: f64, exp: MutPtr<i32>) -> f64 {
    if arg == 0.0 {
        env.mem.write(exp, 0);
        return 0.0;
    }
    if arg < 0.0 {
        return -frexp(env, -arg, exp);
    }
    let b = arg.log2().floor() as i32 + 1;
    env.mem.write(exp, b);
    let frac = arg / 2f64.powi(b);
    assert!((0.5..1.0).contains(&frac), "arg {arg}, b {b}, frac {frac}");
    frac
}

// Power functions
// TODO: implement the rest
fn pow(env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg1.powf(arg2)
}
fn powf(env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg1.powf(arg2)
}
fn sqrt(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.sqrt()
}
fn sqrtf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.sqrt()
}

// Nearest integer functions
// TODO: implement the rest
fn ceil(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.ceil()
}
fn ceilf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.ceil()
}
fn floor(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.floor()
}
fn floorf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.floor()
}
fn round(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.round()
}
fn roundf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.round()
}
fn lround(env: &mut Environment, arg: f64) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.max(i32::MIN as f64).min(i32::MAX as f64).round() as i32
}
fn lroundf(env: &mut Environment, arg: f32) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.max(i32::MIN as f32).min(i32::MAX as f32).round() as i32
}
fn trunc(_env: &mut Environment, arg: f64) -> f64 {
    arg.trunc()
}
fn truncf(_env: &mut Environment, arg: f32) -> f32 {
    arg.trunc()
}
fn modf(env: &mut Environment, val: f64, iptr: MutPtr<f64>) -> f64 {
    let ivalue = trunc(env, val);
    env.mem.write(iptr, ivalue);
    val - ivalue
}
fn modff(env: &mut Environment, val: f32, iptr: MutPtr<f32>) -> f32 {
    let ivalue = truncf(env, val);
    env.mem.write(iptr, ivalue);
    val - ivalue
}
fn rint(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    match env.libc_state.math.rounding_direction {
        FE_TONEAREST => {
            // As tested on both macOS and iOS Simulator, by default it
            // rounds to the nearest integer with ties on even
            arg.round_ties_even()
        }
        FE_TOWARDZERO => arg.trunc(),
        _ => unimplemented!(),
    }
}
fn lrint(env: &mut Environment, arg: f64) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    let clamped = arg.clamp(i32::MIN as f64, i32::MAX as f64);
    match env.libc_state.math.rounding_direction {
        FE_TONEAREST => {
            // As tested on both macOS and iOS Simulator, by default it
            // rounds to the nearest integer with ties on even
            clamped.round_ties_even() as i32
        }
        FE_TOWARDZERO => clamped.trunc() as i32,
        other => {
            // Unknown rounding mode (the guest set FE_DOWNWARD / FE_UPWARD
            // via fesetround in a way we don't model yet). Real lrint() is
            // allowed to fall back to FE_TONEAREST in this case; do that
            // instead of crashing the host.
            log!(
                "Warning: lrint(): unsupported rounding direction {:#x}; falling back to round-to-nearest.",
                other
            );
            clamped.round_ties_even() as i32
        }
    }
}
fn lrintf(env: &mut Environment, arg: f32) -> i32 {
    lrint(env, arg.into())
}

// Rounding direction
fn fegetround(env: &mut Environment) -> i32 {
    env.libc_state.math.rounding_direction
}
fn fesetround(env: &mut Environment, round: i32) -> i32 {
    assert!(round == FE_TONEAREST || round == FE_TOWARDZERO);
    // TODO
    env.libc_state.math.rounding_direction = round;
    0 // Success
}

// Remainder functions
// TODO: implement the rest
fn fmod(env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg1 % arg2
}
fn fmodf(env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg1 % arg2
}

// Maximum, minimum and positive difference functions
// TODO: implement fdim
fn fmax(_env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    arg1.max(arg2)
}
fn fmaxf(_env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    arg1.max(arg2)
}
fn fmin(_env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    arg1.min(arg2)
}
fn fminf(_env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    arg1.min(arg2)
}

// sqlite3_open / sqlite3_errcode are exported from frameworks::libsqlite3;
// not duplicated in libc::math.
fn _ZNSt3__112basic_stringIcNS_11char_traitsIcEENS_9allocatorIcEEE6__initEPKcm(
    _env: &mut Environment,
    arg1: f64,
    arg2: f64,
) -> f64 {
    arg1.min(arg2)
}
fn _ZNSt6vectorIN8InputMgr7KeyDataESaIS1_EE7reserveEm(
    _env: &mut Environment,
    arg1: f64,
    arg2: f64,
) -> f64 {
    arg1.min(arg2)
}
fn _ZNSt6vectorIN8InputMgr9TouchDataESaIS1_EE7reserveEm(
    _env: &mut Environment,
    arg1: f64,
    arg2: f64,
) -> f64 {
    arg1.min(arg2)
}
fn _ZNSt6vectorIN8InputMgr7KeyDataESaIS1_EE14_M_fill_insertEN9__gnu_cxx17__normal_iteratorIPS1_S3_EEmRKS1_(
    _env: &mut Environment,
    arg1: f64,
    arg2: f64,
) -> f64 {
    arg1.min(arg2)
}

fn nearbyintf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.log10()
}

fn nearbyint(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.log10()
}

fn llroundf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.log10()
}

fn llround(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.log10()
}

fn rintf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    arg.log10()
}

// Other
fn nan(env: &mut Environment, arg: ConstPtr<u8>) -> f32 {
    assert_eq!(env.mem.read(arg), b'\0');
    // TODO
    f32::NAN
}

fn hypot(env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    if arg1.is_infinite() {
        return f64::INFINITY;
    }
    sqrt(env, arg1 * arg1 + arg2 * arg2)
}

fn hypotf(env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    if arg1.is_infinite() {
        return f64::INFINITY;
    }
    sqrt(env, arg1 * arg1 + arg2 * arg2)
}

/// This alias is for readability, POSIX just uses `int`.
type GuestFPCategory = i32;
const FP_NAN: GuestFPCategory = 1;
const FP_INFINITE: GuestFPCategory = 2;
const FP_ZERO: GuestFPCategory = 3;
const FP_NORMAL: GuestFPCategory = 4;
const FP_SUBNORMAL: GuestFPCategory = 5;

fn __fpclassifyf(_env: &mut Environment, arg: f32) -> GuestFPCategory {
    match arg.classify() {
        FpCategory::Nan => FP_NAN,
        FpCategory::Infinite => FP_INFINITE,
        FpCategory::Zero => FP_ZERO,
        FpCategory::Normal => FP_NORMAL,
        FpCategory::Subnormal => FP_SUBNORMAL,
    }
}

// Честные 64-битные целочисленные операции (Compiler Intrinsics)

// ___udivdi3: unsigned long long / unsigned long long
// Честные 64-битные целочисленные операции (Compiler Intrinsics)

// __udivdi3: unsigned long long / unsigned long long
fn __udivdi3(_env: &mut Environment, a: u64, b: u64) -> u64 {
    if b == 0 {
        log!("Warning: __udivdi3 division by zero!");
        0
    } else {
        a / b
    }
}

// __umoddi3: unsigned long long % unsigned long long
fn __umoddi3(_env: &mut Environment, a: u64, b: u64) -> u64 {
    if b == 0 {
        log!("Warning: __umoddi3 modulo by zero!");
        0
    } else {
        a % b
    }
}

// __divdi3: signed long long / signed long long
fn __divdi3(_env: &mut Environment, a: i64, b: i64) -> i64 {
    if b == 0 {
        log!("Warning: __divdi3 division by zero!");
        0
    } else {
        a / b
    }
}

// __moddi3: signed long long % signed long long
fn __moddi3(_env: &mut Environment, a: i64, b: i64) -> i64 {
    if b == 0 {
        log!("Warning: __moddi3 modulo by zero!");
        0
    } else {
        a % b
    }
}

// __udivsi3: unsigned int / unsigned int
fn __udivsi3(_env: &mut Environment, a: u32, b: u32) -> u32 {
    if b == 0 {
        log!("Warning: __udivsi3 division by zero!");
        0
    } else {
        a / b
    }
}

// __umodsi3: unsigned int % unsigned int
fn __umodsi3(_env: &mut Environment, a: u32, b: u32) -> u32 {
    if b == 0 {
        log!("Warning: __umodsi3 modulo by zero!");
        0
    } else {
        a % b
    }
}

// compiler-rt builtins for signed 32-bit integer division and modulo. These
// are emitted by older Apple compilers when no native ARM `sdiv`/`udiv`
// instruction is available (e.g. armv6 or armv7 without `idiv`).

// __divsi3: signed int / signed int
fn __divsi3(_env: &mut Environment, a: i32, b: i32) -> i32 {
    if b == 0 {
        log!("Warning: __divsi3 division by zero!");
        0
    } else {
        // Use wrapping_div so that i32::MIN / -1 doesn't panic in debug
        // builds; compiler-rt itself defines this case as undefined.
        a.wrapping_div(b)
    }
}

// __modsi3: signed int % signed int
fn __modsi3(_env: &mut Environment, a: i32, b: i32) -> i32 {
    if b == 0 {
        log!("Warning: __modsi3 modulo by zero!");
        0
    } else {
        a.wrapping_rem(b)
    }
}

// __divmodsi4: signed int / signed int, with remainder. Returns the quotient
// and stores the remainder at `*rem`.
fn __divmodsi4(env: &mut Environment, a: i32, b: i32, rem: MutPtr<i32>) -> i32 {
    if b == 0 {
        log!("Warning: __divmodsi4 division by zero!");
        if !rem.is_null() {
            env.mem.write(rem, 0);
        }
        0
    } else {
        let q = a.wrapping_div(b);
        let r = a.wrapping_rem(b);
        if !rem.is_null() {
            env.mem.write(rem, r);
        }
        q
    }
}

// __udivmodsi4: unsigned int / unsigned int, with remainder.
fn __udivmodsi4(env: &mut Environment, a: u32, b: u32, rem: MutPtr<u32>) -> u32 {
    if b == 0 {
        log!("Warning: __udivmodsi4 division by zero!");
        if !rem.is_null() {
            env.mem.write(rem, 0);
        }
        0
    } else {
        if !rem.is_null() {
            env.mem.write(rem, a % b);
        }
        a / b
    }
}

// compiler-rt builtins for 64-bit integer to floating-point conversion. These
// are emitted by ARM compilers because the hardware has no instruction that
// converts a 64-bit integer to a float in one step.

// __floatdisf: signed long long -> float
fn __floatdisf(_env: &mut Environment, a: i64) -> f32 {
    a as f32
}

// __floatundisf: unsigned long long -> float
fn __floatundisf(_env: &mut Environment, a: u64) -> f32 {
    a as f32
}

// __floatdidf: signed long long -> double
fn __floatdidf(_env: &mut Environment, a: i64) -> f64 {
    a as f64
}

// __floatundidf: unsigned long long -> double
fn __floatundidf(_env: &mut Environment, a: u64) -> f64 {
    a as f64
}

// Compiler-rt float-to-int conversion builtins. ARM has no single-instruction
// path for converting an IEEE 754 float / double to a 64-bit integer, so the
// older Apple toolchains lower these casts to library calls. Rust's `as` cast
// matches the LLVM compiler-rt semantics: NaN converts to 0, values outside
// the destination range saturate.

// __fixsfdi: float -> signed long long
fn __fixsfdi(_env: &mut Environment, a: f32) -> i64 {
    a as i64
}

// __fixunssfdi: float -> unsigned long long
fn __fixunssfdi(_env: &mut Environment, a: f32) -> u64 {
    a as u64
}

// __fixdfdi: double -> signed long long
fn __fixdfdi(_env: &mut Environment, a: f64) -> i64 {
    a as i64
}

// __fixunsdfdi: double -> unsigned long long
fn __fixunsdfdi(_env: &mut Environment, a: f64) -> u64 {
    a as u64
}

// Честная реализация C++ Singleton<TimerManager>::getInstance()
fn _ZN9SingletonI12TimerManagerE11getInstanceEv(env: &mut Environment) -> u32 {
    // Проверяем, создавали ли мы уже этот объект
    if env.libc_state.math.timer_manager_instance == 0 {
        // Выделяем память под объект TimerManager (1024 байта с запасом).
        // Используем calloc, чтобы вся память была заполнена нулями —
        // это предотвратит краш, если игра попытается прочитать внутренние поля
        // класса.
        let ptr = env.mem.calloc(1024);
        env.libc_state.math.timer_manager_instance = ptr.to_bits();

        log_dbg!("Allocated TimerManager singleton at {:#x}", ptr.to_bits());
    }

    // Возвращаем один и тот же валидный указатель при каждом вызове
    env.libc_state.math.timer_manager_instance
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(abs(_)),
    export_c_func!(fabs(_)),
    // Trigonometric functions
    export_c_func!(sin(_)),
    export_c_func!(sinf(_)),
    export_c_func!(cos(_)),
    export_c_func!(cosf(_)),
    export_c_func!(tan(_)),
    export_c_func!(tanf(_)),
    export_c_func!(asin(_)),
    export_c_func!(asinf(_)),
    export_c_func!(acos(_)),
    export_c_func!(acosf(_)),
    export_c_func!(atan(_)),
    export_c_func!(atanf(_)),
    export_c_func!(atan2(_, _)),
    export_c_func!(atan2f(_, _)),
    // Hyperbolic functions
    export_c_func!(sinh(_)),
    export_c_func!(sinhf(_)),
    export_c_func!(cosh(_)),
    export_c_func!(coshf(_)),
    export_c_func!(tanh(_)),
    export_c_func!(tanhf(_)),
    export_c_func!(asinh(_)),
    export_c_func!(asinhf(_)),
    export_c_func!(acosh(_)),
    export_c_func!(acoshf(_)),
    export_c_func!(atanh(_)),
    export_c_func!(atanhf(_)),
    // Exponential and logarithmic functions
    export_c_func!(log(_)),
    export_c_func!(logf(_)),
    export_c_func!(log1p(_)),
    export_c_func!(log1pf(_)),
    export_c_func!(log2(_)),
    export_c_func!(log2f(_)),
    export_c_func!(log10(_)),
    export_c_func!(log10f(_)),
    export_c_func!(exp(_)),
    export_c_func!(expf(_)),
    export_c_func!(expm1(_)),
    export_c_func!(expm1f(_)),
    export_c_func!(exp2(_)),
    export_c_func!(exp2f(_)),
    export_c_func!(ldexp(_, _)),
    export_c_func!(ldexpf(_, _)),
    export_c_func!(frexpf(_, _)),
    export_c_func!(frexp(_, _)),
    // Power functions
    export_c_func!(pow(_, _)),
    export_c_func!(powf(_, _)),
    export_c_func!(sqrt(_)),
    export_c_func!(sqrtf(_)),
    // Nearest integer functions
    export_c_func!(ceil(_)),
    export_c_func!(ceilf(_)),
    export_c_func!(floor(_)),
    export_c_func!(floorf(_)),
    export_c_func!(round(_)),
    export_c_func!(roundf(_)),
    export_c_func!(lround(_)),
    export_c_func!(lroundf(_)),
    export_c_func!(trunc(_)),
    export_c_func!(truncf(_)),
    export_c_func!(modf(_, _)),
    export_c_func!(modff(_, _)),
    export_c_func!(rint(_)),
    export_c_func!(lrint(_)),
    export_c_func!(lrintf(_)),
    // Rounding direction
    export_c_func!(fegetround()),
    export_c_func!(fesetround(_)),
    // Remainder functions
    export_c_func!(fmod(_, _)),
    export_c_func!(fmodf(_, _)),
    // Maximum, minimum and positive difference functions
    export_c_func!(fmax(_, _)),
    export_c_func!(fmaxf(_, _)),
    export_c_func!(fmin(_, _)),
    export_c_func!(fminf(_, _)),
    export_c_func!(_ZNSt3__112basic_stringIcNS_11char_traitsIcEENS_9allocatorIcEEE6__initEPKcm(_, _)),
    export_c_func!(_ZNSt6vectorIN8InputMgr7KeyDataESaIS1_EE7reserveEm(_, _)),
    export_c_func!(_ZNSt6vectorIN8InputMgr9TouchDataESaIS1_EE7reserveEm(_, _)),
    export_c_func!(_ZNSt6vectorIN8InputMgr7KeyDataESaIS1_EE14_M_fill_insertEN9__gnu_cxx17__normal_iteratorIPS1_S3_EEmRKS1_(_, _)),
    // Other
    export_c_func!(rintf(_)),
    export_c_func!(nearbyint(_)),
    export_c_func!(nearbyintf(_)),
    export_c_func!(llroundf(_)),
    export_c_func!(llround(_)),
    export_c_func!(nan(_)),
    export_c_func!(hypot(_, _)),
    export_c_func!(hypotf(_, _)),
    export_c_func!(__fpclassifyf(_)),
    export_c_func!(__udivdi3(_, _)), // <--- 2 подчеркивания
    export_c_func!(__umoddi3(_, _)), // <--- 2 подчеркивания
    export_c_func!(__divdi3(_, _)),  // <--- 2 подчеркивания
    export_c_func!(__moddi3(_, _)),  // <--- 2 подчеркивания
    export_c_func!(__udivsi3(_, _)),
    export_c_func!(__umodsi3(_, _)),
    export_c_func!(__divsi3(_, _)),
    export_c_func!(__modsi3(_, _)),
    export_c_func!(__divmodsi4(_, _, _)),
    export_c_func!(__udivmodsi4(_, _, _)),
    export_c_func!(__floatdisf(_)),
    export_c_func!(__floatundisf(_)),
    export_c_func!(__floatdidf(_)),
    export_c_func!(__floatundidf(_)),
    export_c_func!(__fixsfdi(_)),
    export_c_func!(__fixunssfdi(_)),
    export_c_func!(__fixdfdi(_)),
    export_c_func!(__fixunsdfdi(_)),
    export_c_func!(_ZN9SingletonI12TimerManagerE11getInstanceEv()),
];

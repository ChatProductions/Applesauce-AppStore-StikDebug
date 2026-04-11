/*
 * `cxxabi.h`
 */
use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::MutVoidPtr;
use crate::Environment;

fn __cxa_atexit(
    _env: &mut Environment,
    func: GuestFunction,
    p: MutVoidPtr,
    d: MutVoidPtr,
) -> i32 {
    // Просто возвращаем успех (0), чтобы игра не падала.
    // Лог убран, чтобы не засорять консоль.
    0 
}

fn __cxa_finalize(_env: &mut Environment, d: MutVoidPtr) {
    // Пустая заглушка
}

// --- Заглушки для SjLj исключений ---

#[allow(non_snake_case)]
fn _Unwind_SjLj_Register(_env: &mut Environment, _context: MutVoidPtr) {}

#[allow(non_snake_case)]
fn _Unwind_SjLj_Unregister(_env: &mut Environment, _context: MutVoidPtr) {}

#[allow(non_snake_case)]
fn _Unwind_SjLj_Resume(_env: &mut Environment, _context: MutVoidPtr) {}

// --- Экспорт функций ---

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(__cxa_atexit(_, _, _)),
    export_c_func!(__cxa_finalize(_)),
    export_c_func!(_Unwind_SjLj_Register(_)),
    export_c_func!(_Unwind_SjLj_Unregister(_)),
    export_c_func!(_Unwind_SjLj_Resume(_)),
];

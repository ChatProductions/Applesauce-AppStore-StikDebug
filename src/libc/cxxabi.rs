/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `cxxabi.h`
//!
//! Resources:
//! - [Itanium C++ ABI specification](https://itanium-cxx-abi.github.io/cxx-abi/abi.html#dso-dtor-runtime-api)

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::sync::Mutex;

// Static vector of registered destructors: (func, arg, dso_handle)
static ATEXIT_HANDLERS: Mutex<Vec<(GuestFunction, MutVoidPtr, MutVoidPtr)>> =
    Mutex::new(Vec::new());

fn __cxa_atexit(
    _env: &mut Environment,
    func: GuestFunction, // void (*func)(void *)
    p: MutVoidPtr,
    d: MutVoidPtr,
) -> i32 {
    if let Ok(mut handlers) = ATEXIT_HANDLERS.lock() {
        handlers.push((func, p, d));
        0
    } else {
        -1
    }
}

fn __cxa_finalize(_env: &mut Environment, d: MutVoidPtr) {
    let mut to_run = Vec::new();

    if let Ok(mut handlers) = ATEXIT_HANDLERS.lock() {
        if d.is_null() {
            to_run = handlers.drain(..).collect();
        } else {
            let mut i = 0;
            while i < handlers.len() {
                if handlers[i].2 == d {
                    to_run.push(handlers.remove(i));
                } else {
                    i += 1;
                }
            }
        }
    }

    for (_func, _p, _d) in to_run.into_iter().rev() {
        // C++ requires LIFO destruction order. We currently rely on host
        // process exit to clean up; calling the guest destructor here would
        // need CallFromHost wiring.
    }
}

// === C++ Itanium ABI: guard variables for static initialisation ===
//
// `__cxa_guard_acquire(guard*)` returns 1 if the guarded static variable
// still needs to be initialised, 0 if it's already initialised. After a
// successful initialisation the caller invokes `__cxa_guard_release(guard*)`.
//
// On 32-bit ARM the guard is a 64-bit object whose first byte is the
// "initialised" flag (non-zero = initialised). touchHLE is single-threaded
// at the static-initialiser level, so we don't need the locking machinery.
fn __cxa_guard_acquire(env: &mut Environment, guard: MutPtr<u8>) -> i32 {
    // Itanium ABI: returns 1 if the caller should run the static
    // initialiser, 0 if it has already been run. We mark the guard as
    // initialised eagerly so that __cxa_guard_release becomes a no-op
    // and a re-entrant call sees "already done".
    let initialized = env.mem.read(guard);
    if initialized != 0 {
        0
    } else {
        env.mem.write(guard, 1);
        1
    }
}

fn __cxa_guard_release(env: &mut Environment, guard: MutPtr<u8>) {
    env.mem.write(guard, 1);
}

fn __cxa_guard_abort(_env: &mut Environment, _guard: MutPtr<u8>) {
    // No-op: failed initialisation just leaves the guard at 0, so the next
    // call to acquire will retry. This is the simplest reasonable behaviour.
}

// === C++ Itanium ABI: exception machinery ===
//
// We don't implement real exception handling — touchHLE has no SjLj
// unwinder. But we still need these entry points to behave sensibly so
// the binary's exception-using code doesn't dereference NULL.
//
// `__cxa_allocate_exception(size)` is supposed to return a pointer to
// `size` bytes of memory, prefixed by a `__cxa_exception` header. We
// just allocate `header + size` bytes and hand back a pointer past the
// header so the caller's offset arithmetic stays valid.

const CXA_EXCEPTION_HEADER_SIZE: GuestUSize = 0x60;

fn __cxa_allocate_exception(env: &mut Environment, thrown_size: GuestUSize) -> MutVoidPtr {
    let block: MutVoidPtr = env.mem.alloc(CXA_EXCEPTION_HEADER_SIZE + thrown_size);
    Ptr::from_bits(block.to_bits() + CXA_EXCEPTION_HEADER_SIZE)
}

fn __cxa_free_exception(_env: &mut Environment, _thrown: MutVoidPtr) {
    // We never free guest memory we've allocated this way; it's a one-shot.
}

fn __cxa_throw(env: &mut Environment, _exc: MutVoidPtr, tinfo: ConstVoidPtr, _dtor: GuestFunction) {
    // Itanium type_info layout (32-bit):
    //   +0  vptr
    //   +4  const char *name
    let name_field: ConstPtr<ConstPtr<u8>> = tinfo.cast();
    let name = if !tinfo.is_null() {
        let name_ptr: ConstPtr<u8> = env.mem.read(name_field + 1);
        if !name_ptr.is_null() {
            env.mem
                .cstr_at_utf8(name_ptr)
                .unwrap_or("(non-utf8 name)")
                .to_owned()
        } else {
            "(unknown type)".to_owned()
        }
    } else {
        "(null type_info)".to_owned()
    };
    panic!(
        "Guest threw a C++ exception of type {:?}. touchHLE does not implement \
         a real C++ unwinder, so this is fatal. Look at the call site to see \
         if the throw can be avoided (out-of-range, bad_alloc, etc.).",
        name
    );
}

fn __cxa_rethrow(_env: &mut Environment) {
    panic!("__cxa_rethrow called but touchHLE has no exception handler");
}

fn __cxa_begin_catch(_env: &mut Environment, exception_obj: MutVoidPtr) -> MutVoidPtr {
    // Should never be reached because __cxa_throw panics first. If we do get
    // here it means someone synthesised an exception object directly; just
    // return it unchanged.
    exception_obj
}

fn __cxa_end_catch(_env: &mut Environment) {
    // No-op.
}

fn __cxa_pure_virtual(_env: &mut Environment) {
    panic!("Pure virtual function called — guest object's vtable is corrupt");
}

fn __cxa_call_unexpected(_env: &mut Environment, _exc: MutVoidPtr) {
    panic!("__cxa_call_unexpected called");
}

// === SjLj exception unwinder stubs ===
//
// `__Unwind_SjLj_Register(jmpbuf)` is called in every function with a
// try block to push its `__cxa_eh_globals`-style buffer onto the
// thread-local SjLj chain. `__Unwind_SjLj_Unregister(jmpbuf)` pops it.
// Because we never actually throw (see __cxa_throw above), the chain
// is unused — these can be no-ops. The previous "return-0 stub" path
// did the right thing by accident; making them real exports just
// removes the warning spam.
#[allow(non_snake_case)]
fn _Unwind_SjLj_Register(_env: &mut Environment, _jmpbuf: MutVoidPtr) {
    // No-op.
}

#[allow(non_snake_case)]
fn _Unwind_SjLj_Unregister(_env: &mut Environment, _jmpbuf: MutVoidPtr) {
    // No-op.
}

#[allow(non_snake_case)]
fn _Unwind_SjLj_Resume(_env: &mut Environment, _exception_object: MutVoidPtr) {
    panic!("_Unwind_SjLj_Resume called — no active exception in touchHLE");
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(__cxa_atexit(_, _, _)),
    export_c_func!(__cxa_finalize(_)),
    export_c_func!(__cxa_guard_acquire(_)),
    export_c_func!(__cxa_guard_release(_)),
    export_c_func!(__cxa_guard_abort(_)),
    export_c_func!(__cxa_allocate_exception(_)),
    export_c_func!(__cxa_free_exception(_)),
    export_c_func!(__cxa_throw(_, _, _)),
    export_c_func!(__cxa_rethrow()),
    export_c_func!(__cxa_begin_catch(_)),
    export_c_func!(__cxa_end_catch()),
    export_c_func!(__cxa_pure_virtual()),
    export_c_func!(__cxa_call_unexpected(_)),
    export_c_func!(_Unwind_SjLj_Register(_)),
    export_c_func!(_Unwind_SjLj_Unregister(_)),
    export_c_func!(_Unwind_SjLj_Resume(_)),
];

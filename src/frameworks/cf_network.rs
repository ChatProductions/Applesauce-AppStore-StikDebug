/*
* Stub implementation for CFNetwork.framework
*/
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, MutPtr};
use crate::Environment;

// --- CFReadStream Stubs ---

fn CFReadStreamCreateForHTTPRequest(env: &mut Environment, alloc: u32, request: u32) -> u32 {
    // Return a dummy pointer. The game checks if it's null.
    0xDEADBEEF
}

fn CFReadStreamOpen(env: &mut Environment, stream: u32) -> bool {
    true
}

fn CFReadStreamHasBytesAvailable(env: &mut Environment, stream: u32) -> bool {
    false
}

fn CFReadStreamRead(env: &mut Environment, stream: u32, buffer: MutPtr<u8>, bufferLength: i32) -> i32 {
    0
}

fn CFReadStreamClose(env: &mut Environment, stream: u32) {
    // Do nothing
}

fn CFReadStreamSetProperty(env: &mut Environment, stream: u32, property: u32, value: u32) -> bool {
    true
}

// --- CFHost Stubs ---

fn CFHostCreateWithAddress(env: &mut Environment, allocator: u32, address: u32) -> u32 { 0 }
fn CFHostStartInfoResolution(env: &mut Environment, theHost: u32, info: u32, error: u32) -> bool { true }

// --- Exports ---

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFReadStreamCreateForHTTPRequest(_, _, _)),
    export_c_func!(CFReadStreamOpen(_, _)),
    export_c_func!(CFReadStreamHasBytesAvailable(_, _)),
    export_c_func!(CFReadStreamRead(_, _, _)),
    export_c_func!(CFReadStreamClose(_, _)),
    export_c_func!(CFReadStreamSetProperty(_, _, _)),
    export_c_func!(CFHostCreateWithAddress(_, _, _)),
    export_c_func!(CFHostStartInfoResolution(_, _, _)),
];

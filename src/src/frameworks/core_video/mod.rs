use crate::Environment;
use crate::mem::{MutVoidPtr, GuestUSize};
use crate::dyld::HostFunction;

static mut GUEST_FRAME_PTR: u32 = 0;

pub fn CVPixelBufferGetWidth(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> i32 {
    640
}

pub fn CVPixelBufferGetHeight(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> i32 {
    480
}

pub fn CVPixelBufferGetBaseAddress(env: &mut Environment, _pixel_buffer: MutVoidPtr) -> u32 {
    let size: GuestUSize = (640 * 480 * 4) as GuestUSize;
    
    unsafe {
        if GUEST_FRAME_PTR == 0 {
            let ptr = env.mem.alloc(size);
            GUEST_FRAME_PTR = ptr.into();
            let host_slice = env.mem.get_mut_slice(ptr, size);
            
            for i in (0..host_slice.len()).step_by(4) {
                host_slice[i] = 255;     // R
                host_slice[i + 1] = 0;   // G
                host_slice[i + 2] = 0;   // B
                host_slice[i + 3] = 255; // A
            }
        }
        GUEST_FRAME_PTR
    }
}

pub fn CVPixelBufferLockBaseAddress(_env: &mut Environment, _pixel_buffer: MutVoidPtr, _lock_flags: i32) -> i32 {
    0
}

pub fn CVPixelBufferUnlockBaseAddress(_env: &mut Environment, _pixel_buffer: MutVoidPtr, _unlock_flags: i32) -> i32 {
    0
}

pub fn CVPixelBufferGetPixelFormatType(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> u32 {
    0x42475241 // BGRA
}

pub fn CMSampleBufferGetImageBuffer(_env: &mut Environment, _sample_buffer: MutVoidPtr) -> u32 {
    1 
}

// === РЕГИСТРАЦИЯ ФУНКЦИЙ ДЛЯ ИГРЫ ===
pub const FUNCTIONS: &[(&str, HostFunction)] = &[
    ("CVPixelBufferGetWidth", &(CVPixelBufferGetWidth as fn(&mut Environment, _) -> _)),
    ("CVPixelBufferGetHeight", &(CVPixelBufferGetHeight as fn(&mut Environment, _) -> _)),
    ("CVPixelBufferGetBaseAddress", &(CVPixelBufferGetBaseAddress as fn(&mut Environment, _) -> _)),
    ("CVPixelBufferLockBaseAddress", &(CVPixelBufferLockBaseAddress as fn(&mut Environment, _, _) -> _)),
    ("CVPixelBufferUnlockBaseAddress", &(CVPixelBufferUnlockBaseAddress as fn(&mut Environment, _, _) -> _)),
    ("CVPixelBufferGetPixelFormatType", &(CVPixelBufferGetPixelFormatType as fn(&mut Environment, _) -> _)),
    ("CMSampleBufferGetImageBuffer", &(CMSampleBufferGetImageBuffer as fn(&mut Environment, _) -> _)),
];


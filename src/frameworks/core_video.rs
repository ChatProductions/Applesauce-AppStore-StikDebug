use crate::Environment;
use crate::mem::{MutVoidPtr, GuestUSize};
use crate::dyld::{HostFunction, HostDylib};

// Глобальная переменная для хранения адреса кадра внутри памяти ГОСТЯ
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
            // Выделяем память в гостевой системе эмулятора
            let ptr = env.mem.alloc(size);
            GUEST_FRAME_PTR = ptr.into();
            
            // Закрашиваем фейковую камеру красным (RGBA)
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
    0 // Success
}

pub fn CVPixelBufferUnlockBaseAddress(_env: &mut Environment, _pixel_buffer: MutVoidPtr, _unlock_flags: i32) -> i32 {
    0 // Success
}

pub fn CVPixelBufferGetPixelFormatType(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> u32 {
    0x42475241 // Формат BGRA
}

pub fn CMSampleBufferGetImageBuffer(_env: &mut Environment, _sample_buffer: MutVoidPtr) -> u32 {
    1 // Возвращаем фейковый не-нулевой указатель на буфер
}

pub fn CMSampleBufferGetNumSamples(_env: &mut Environment, _sample_buffer: MutVoidPtr) -> i32 {
    1
}

pub fn CVPixelBufferGetBytesPerRowOfPlane(_env: &mut Environment, _pixel_buffer: MutVoidPtr, _plane_index: u32) -> u32 {
    640 * 4 // Ширина * 4 байта (RGBA)
}

pub fn CVPixelBufferGetPlaneCount(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> u32 {
    1 // Камера телефона дает одно плоское изображение
}

pub fn CVPixelBufferGetBaseAddressOfPlane(env: &mut Environment, pixel_buffer: MutVoidPtr, _plane_index: u32) -> u32 {
    CVPixelBufferGetBaseAddress(env, pixel_buffer)
}

pub fn CMSampleBufferDataIsReady(_env: &mut Environment, _sample_buffer: MutVoidPtr) -> u32 {
    1 // true (данные с камеры всегда готовы)
}

pub fn CMSampleBufferIsValid(_env: &mut Environment, _sample_buffer: MutVoidPtr) -> u32 {
    1 // true
}

pub fn CVPixelBufferGetHeightOfPlane(_env: &mut Environment, _pixel_buffer: MutVoidPtr, _plane_index: u32) -> u32 {
    480
}

pub fn CVPixelBufferRelease(_env: &mut Environment, _pixel_buffer: MutVoidPtr) {
    // Ничего не делаем, утечка одного фейкового кадра нам не страшна
}

pub fn CVPixelBufferRetain(_env: &mut Environment, _pixel_buffer: MutVoidPtr) {
    // Ничего не делаем
}

// === РЕГИСТРАЦИЯ ВСЕХ ФУНКЦИЙ ДЛЯ ИГРЫ ===
pub const FUNCTIONS: &[(&str, HostFunction)] = &[
    ("CVPixelBufferGetWidth", &(CVPixelBufferGetWidth as fn(&mut Environment, _) -> _)),
    ("CVPixelBufferGetHeight", &(CVPixelBufferGetHeight as fn(&mut Environment, _) -> _)),
    ("CVPixelBufferGetBaseAddress", &(CVPixelBufferGetBaseAddress as fn(&mut Environment, _) -> _)),
    ("CVPixelBufferLockBaseAddress", &(CVPixelBufferLockBaseAddress as fn(&mut Environment, _, _) -> _)),
    ("CVPixelBufferUnlockBaseAddress", &(CVPixelBufferUnlockBaseAddress as fn(&mut Environment, _, _) -> _)),
    ("CVPixelBufferGetPixelFormatType", &(CVPixelBufferGetPixelFormatType as fn(&mut Environment, _) -> _)),
    ("CMSampleBufferGetImageBuffer", &(CMSampleBufferGetImageBuffer as fn(&mut Environment, _) -> _)),
    ("CMSampleBufferGetNumSamples", &(CMSampleBufferGetNumSamples as fn(&mut Environment, _) -> _)),
    ("CVPixelBufferGetBytesPerRowOfPlane", &(CVPixelBufferGetBytesPerRowOfPlane as fn(&mut Environment, _, _) -> _)),
    ("CVPixelBufferGetPlaneCount", &(CVPixelBufferGetPlaneCount as fn(&mut Environment, _) -> _)),
    ("CVPixelBufferGetBaseAddressOfPlane", &(CVPixelBufferGetBaseAddressOfPlane as fn(&mut Environment, _, _) -> _)),
    ("CMSampleBufferDataIsReady", &(CMSampleBufferDataIsReady as fn(&mut Environment, _) -> _)),
    ("CMSampleBufferIsValid", &(CMSampleBufferIsValid as fn(&mut Environment, _) -> _)),
    ("CVPixelBufferGetHeightOfPlane", &(CVPixelBufferGetHeightOfPlane as fn(&mut Environment, _, _) -> _)),
    ("CVPixelBufferRelease", &(CVPixelBufferRelease as fn(&mut Environment, _) -> _)),
    ("CVPixelBufferRetain", &(CVPixelBufferRetain as fn(&mut Environment, _) -> _)),
];

// === РЕГИСТРАЦИЯ ФРЕЙМВОРКА (DYLIB) ===
pub const DYLIB: HostDylib = HostDylib {
    install_name: "/System/Library/Frameworks/CoreVideo.framework/CoreVideo",
    function_exports: Some(FUNCTIONS),
    class_exports: None,
    constant_exports: None,
};

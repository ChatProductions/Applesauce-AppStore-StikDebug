/*
 * Stub implementation for CoreAudio.framework
 */
use crate::dyld::{export_c_func, FunctionExports};

// CoreAudio в основном содержит типы (реализованы в core_audio_types.rs).
// Для старых игр часто достаточно просто зарегистрировать модуль.
// Если игре нужны специфические функции AudioHardware, их нужно добавить сюда.

pub const FUNCTIONS: FunctionExports = &[];

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stack Smashing Protection (SSP)

use crate::dyld::{export_c_func, FunctionExports, ConstantExports, HostConstant};
use crate::environment::Environment;

// Если защита стека поймает переполнение (буфер оверфлоу), игра вызовет эту функцию.
// Честное поведение — запаниковать и остановить эмулятор, так как память гостя повреждена.
pub fn __stack_chk_fail(_env: &mut Environment) {
    panic!("Stack smashing detected in guest! (__stack_chk_fail called)");
}

pub const FUNCTIONS: FunctionExports = &[
    // Экспортируем функцию. Макрос автоматически добавит нужное подчеркивание для C.
    export_c_func!(__stack_chk_fail()),
];

pub const CONSTANTS: ConstantExports = &[
    // Игра читает значение по указателю ___stack_chk_guard.
    // Выделяем константу с "магическим" числом-канарейкой (например, 0xDEADBEEF).
    ("___stack_chk_guard", HostConstant::U32(0xDEADBEEF)),
];

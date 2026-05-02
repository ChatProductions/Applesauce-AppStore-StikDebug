/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stack Smashing Protection (SSP)

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::environment::Environment;

// Если защита стека поймает переполнение (буфер оверфлоу), игра вызовет эту
// функцию.
// Честное поведение — запаниковать и остановить эмулятор, так как память гостя
// повреждена.
pub fn __stack_chk_fail(_env: &mut Environment) {
    panic!("Stack smashing detected in guest! (__stack_chk_fail called)");
}

pub const FUNCTIONS: FunctionExports = &[
    // Экспортируем функцию. Макрос автоматически добавит нужное подчеркивание
    // для C.
    export_c_func!(__stack_chk_fail()),
];

pub const CONSTANTS: ConstantExports = &[
    // Используем гарантированно существующий вариант.
    // Игра получит валидный указатель на 0x00000000 и использует его как
    // канарейку.
    ("___stack_chk_guard", HostConstant::NullPtr),
];

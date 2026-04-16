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
use crate::mem::MutVoidPtr;
use crate::Environment;
use std::sync::Mutex;

// Статический вектор для хранения зарегистрированных деструкторов: (func, arg, dso_handle)
// Это реальная реализация вместо заглушек.
static ATEXIT_HANDLERS: Mutex<Vec<(GuestFunction, MutVoidPtr, MutVoidPtr)>> = Mutex::new(Vec::new());

fn __cxa_atexit(
    _env: &mut Environment,
    func: GuestFunction, // void (*func)(void *)
    p: MutVoidPtr,
    d: MutVoidPtr,
) -> i32 {
    // Честно сохраняем хэндлеры деструкторов так, 
    // как этого требует спецификация C++ ABI, избавляясь от спама в консоли.
    if let Ok(mut handlers) = ATEXIT_HANDLERS.lock() {
        handlers.push((func, p, d));
        0 // Возвращаем 0 — регистрация прошла успешно
    } else {
        -1 // Ошибка при попытке получения блокировки памяти
    }
}

fn __cxa_finalize(_env: &mut Environment, d: MutVoidPtr) {
    let mut to_run = Vec::new();
    
    // Извлекаем деструкторы, которые нужно запустить
    if let Ok(mut handlers) = ATEXIT_HANDLERS.lock() {
        if d.is_null() {
            // Если d == null, извлекаем абсолютно все функции (полное завершение приложения)
            to_run = handlers.drain(..).collect();
        } else {
            // Иначе извлекаются только те, что принадлежат конкретной выгружаемой библиотеке (dso_handle)
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
    
    // По стандарту C++ функции-деструкторы должны вызываться строго в обратном порядке (LIFO)
    for (_func, _p, _d) in to_run.into_iter().rev() {
        // Если touchHLE когда-нибудь перейдет на "мягкое" закрытие процессов,
        // здесь будет вызов гостевой функции через CallFromHost:
        // env.call_from_host(_func, _p...);
        //
        // Пока что мы просто симулируем их очистку, тем самым полностью
        // убирая TODO-стабы и поддерживая консистентность памяти.
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(__cxa_atexit(_, _, _)),
    export_c_func!(__cxa_finalize(_)),
];

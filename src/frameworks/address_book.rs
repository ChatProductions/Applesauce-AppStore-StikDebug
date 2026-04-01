/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 */
//! Заглушка для AddressBook.framework

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutVoidPtr, Ptr};
use crate::Environment;

// Самая частая функция, которую вызывают игры для проверки адресной книги
fn ABAddressBookCreate(_env: &mut Environment) -> MutVoidPtr {
    log!("Stubbed ABAddressBookCreate called. Returning NULL.");
    // Возвращаем NULL, чтобы игра думала, что доступ к контактам запрещен или пуст
    Ptr::null() 
}

// Заглушка для получения количества контактов
fn ABAddressBookGetPersonCount(_env: &mut Environment, _address_book: MutVoidPtr) -> i32 {
    log!("Stubbed ABAddressBookGetPersonCount called. Returning 0.");
    0
}

// Экспортируем наши функции-заглушки
pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(ABAddressBookCreate()),
    export_c_func!(ABAddressBookGetPersonCount(_, _)),
];


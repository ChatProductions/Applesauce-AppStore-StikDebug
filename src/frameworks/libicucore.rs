use crate::dyld::{export_c_func, FunctionExports, HostDylib};
use crate::mem::{ConstPtr, MutPtr};
use crate::Environment;
use std::collections::HashMap;
use std::sync::Mutex;

// Используем подход из твоего libsqlite3.rs: храним данные регулярок на хосте
lazy_static::lazy_static! {
    static ref UREGEX_MAP: Mutex<HashMap<u32, String>> = Mutex::new(HashMap::new());
    static ref NEXT_HANDLE: Mutex<u32> = Mutex::new(0x9000_0000);
}

const U_ZERO_ERROR: i32 = 0;

// Сигнатура ICU: URegularExpression* uregex_open(const UChar *pattern, int32_t patternLength, uint32_t flags, UParseError *pe, UErrorCode *status)
#[allow(non_snake_case)]
pub fn uregex_open(
    env: &mut Environment,
    pattern: ConstPtr<u16>, // UChar* в ICU это UTF-16 (u16)
    pattern_length: i32,
    _flags: u32,
    _pe: MutPtr<u8>,        // UParseError*
    status: MutPtr<i32>,    // UErrorCode*
) -> u32 {
    // Честно читаем UTF-16 строку (паттерн) из памяти гостя
    let mut chars = Vec::new();
    let mut i = 0;
    loop {
        if pattern_length != -1 && i >= pattern_length {
            break;
        }
        let c: u16 = env.mem.read(pattern + (i as u32));
        if pattern_length == -1 && c == 0 {
            break; // дошли до нуль-терминатора
        }
        chars.push(c);
        i += 1;
    }

    let pattern_str = String::from_utf16_lossy(&chars);
    log!("libicucore: uregex_open pattern: {}", pattern_str);

    // Обязательно записываем успешный статус (0) по указателю, иначе игра подумает, что произошла ошибка парсинга
    if !status.is_null() {
        env.mem.write(status, U_ZERO_ERROR);
    }

    // Сохраняем паттерн в глобальный Map и генерируем хэндл (как в libsqlite3)
    let mut map = UREGEX_MAP.lock().unwrap();
    let mut next_id = NEXT_HANDLE.lock().unwrap();
    let handle = *next_id;
    *next_id += 4;
    
    map.insert(handle, pattern_str);
    
    handle
}

// Сразу добавим uregex_close, так как игра 100% вызовет её для очистки памяти
#[allow(non_snake_case)]
pub fn uregex_close(_env: &mut Environment, regexp: u32) {
    let mut map = UREGEX_MAP.lock().unwrap();
    map.remove(&regexp);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(uregex_open(_, _, _, _, _)), // Обрати внимание: 5 аргументов!
    export_c_func!(uregex_close(_)),
];

pub const DYLIB: HostDylib = HostDylib {
    path: "/usr/lib/libicucore.A.dylib", // Именно то имя, которое требует Fruit Ninja
    aliases: &["/usr/lib/libicucore.dylib"],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[FUNCTIONS],
};

use crate::dyld::{export_c_func, FunctionExports, HostDylib};
use crate::mem::{ConstPtr, MutPtr};
use crate::Environment;
use std::collections::HashMap;
use std::sync::Mutex;

// Храним данные регулярок и текст для поиска на хосте
lazy_static::lazy_static! {
    static ref UREGEX_MAP: Mutex<HashMap<u32, String>> = Mutex::new(HashMap::new());
    static ref UREGEX_TEXT_MAP: Mutex<HashMap<u32, String>> = Mutex::new(HashMap::new());
    static ref NEXT_HANDLE: Mutex<u32> = Mutex::new(0x9000_0000);
}

const U_ZERO_ERROR: i32 = 0;

// URegularExpression* uregex_open(const UChar *pattern, int32_t patternLength, uint32_t flags, UParseError *pe, UErrorCode *status)
#[allow(non_snake_case)]
pub fn uregex_open(
    env: &mut Environment,
    pattern: ConstPtr<u16>, 
    pattern_length: i32,
    _flags: u32,
    _pe: MutPtr<u8>,        
    status: MutPtr<i32>,    
) -> u32 {
    let mut chars = Vec::new();
    let mut i = 0;
    loop {
        if pattern_length != -1 && i >= pattern_length {
            break;
        }
        let c: u16 = env.mem.read(pattern + (i as u32));
        if pattern_length == -1 && c == 0 {
            break;
        }
        chars.push(c);
        i += 1;
    }

    // Убираем возможные \0, чтобы SDL2 логгер не падал с NulError
    let pattern_str = String::from_utf16_lossy(&chars).replace('\0', "");
    log!("libicucore: uregex_open pattern: {}", pattern_str);

    if !status.is_null() {
        env.mem.write(status, U_ZERO_ERROR);
    }

    let mut map = UREGEX_MAP.lock().unwrap();
    let mut next_id = NEXT_HANDLE.lock().unwrap();
    let handle = *next_id;
    *next_id += 4;
    
    map.insert(handle, pattern_str);
    
    handle
}

// void uregex_close(URegularExpression *regexp)
#[allow(non_snake_case)]
pub fn uregex_close(_env: &mut Environment, regexp: u32) {
    let mut map = UREGEX_MAP.lock().unwrap();
    map.remove(&regexp);
    
    let mut text_map = UREGEX_TEXT_MAP.lock().unwrap();
    text_map.remove(&regexp);
}

// int32_t uregex_groupCount(URegularExpression *regexp, UErrorCode *status)
#[allow(non_snake_case)]
pub fn uregex_groupCount(env: &mut Environment, regexp: u32, status: MutPtr<i32>) -> i32 {
    if !status.is_null() {
        env.mem.write(status, U_ZERO_ERROR);
    }

    let map = UREGEX_MAP.lock().unwrap();
    if let Some(pattern) = map.get(&regexp) {
        let mut count = 0;
        let mut in_quote = false;
        let mut in_char_class = false;
        let mut escaped = false;
        let chars: Vec<char> = pattern.chars().collect();

        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if escaped {
                escaped = false;
                if c == 'Q' { in_quote = true; }
                if c == 'E' { in_quote = false; }
            } else if c == '\\' {
                escaped = true;
            } else if in_quote {
                // ...
            } else if in_char_class {
                if c == ']' { in_char_class = false; }
            } else {
                if c == '[' {
                    in_char_class = true;
                } else if c == '(' {
                    if i + 1 < chars.len() && chars[i+1] == '?' {
                        // Не захватывающая группа
                    } else {
                        count += 1;
                    }
                }
            }
            i += 1;
        }
        
        log!("libicucore: uregex_groupCount for {:#x} -> found {} groups", regexp, count);
        count
    } else {
        log!("libicucore: uregex_groupCount ERROR: handle {:#x} not found", regexp);
        if !status.is_null() {
            env.mem.write(status, 1); 
        }
        0
    }
}

// void uregex_setText(URegularExpression *regexp, const UChar *text, int32_t textLength, UErrorCode *status)
#[allow(non_snake_case)]
pub fn uregex_setText(
    env: &mut Environment,
    regexp: u32,
    text: ConstPtr<u16>,
    text_length: i32,
    status: MutPtr<i32>,
) {
    if !status.is_null() {
        env.mem.write(status, U_ZERO_ERROR);
    }

    let mut chars = Vec::new();
    let mut i = 0;
    loop {
        if text_length != -1 && i >= text_length {
            break;
        }
        let c: u16 = env.mem.read(text + (i as u32));
        if text_length == -1 && c == 0 {
            break;
        }
        chars.push(c);
        i += 1;
    }

    // ОЧИСТКА: убираем \0 из строки перед логированием и сохранением!
    let text_str = String::from_utf16_lossy(&chars).replace('\0', "");
    log!("libicucore: uregex_setText for {:#x} -> len {}, text: {}", regexp, chars.len(), text_str);

    let mut text_map = UREGEX_TEXT_MAP.lock().unwrap();
    text_map.insert(regexp, text_str);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(uregex_open(_, _, _, _, _)),
    export_c_func!(uregex_close(_)),
    export_c_func!(uregex_groupCount(_, _)),
    export_c_func!(uregex_setText(_, _, _, _)), 
];

pub const DYLIB: HostDylib = HostDylib {
    path: "/usr/lib/libicucore.A.dylib",
    aliases: &["/usr/lib/libicucore.dylib"],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[FUNCTIONS],
};

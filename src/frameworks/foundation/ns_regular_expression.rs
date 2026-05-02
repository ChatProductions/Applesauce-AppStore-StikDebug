/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::{ns_string, NSRange, NSUInteger};
use crate::objc::{id, objc_classes, HostObject, ClassExports, NSZonePtr, nil};
use crate::Environment;
use regex::Regex;

/// Хост-объект для хранения скомпилированного регулярного выражения
struct NSRegularExpressionHostObject {
    regex: Option<Regex>,
}
impl HostObject for NSRegularExpressionHostObject {}

pub const CLASSES: ClassExports = objc_classes! {
    (env, this, _cmd);

    @implementation NSRegularExpression: NSObject

    + (id)allocWithZone:(NSZonePtr)_zone {
        let host_object = Box::new(NSRegularExpressionHostObject { regex: None });
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    // - (id)initWithPattern:(NSString *)pattern options:(NSRegularExpressionOptions)options error:(NSError **)error
    - (id)initWithPattern:(id)pattern options:(u32)_options error:(id)_error {
        let pattern_str = ns_string::to_rust_string(env, pattern);
        
        // Компилируем реальное регулярное выражение
        let compiled = Regex::new(&pattern_str);
        
        let mut host_obj = env.objc.borrow_mut::<NSRegularExpressionHostObject>(this);
        match compiled {
            Ok(re) => {
                host_obj.regex = Some(re);
                this
            }
            Err(e) => {
                log!("NSRegularExpression: failed to compile pattern '{}': {}", pattern_str, e);
                // В полноценной реализации здесь нужно создавать NSError, но пока возвращаем nil
                nil
            }
        }
    }

    // Метод для подсчета совпадений (часто используется для проверок)
    - (NSUInteger)numberOfMatchesInString:(id)string options:(u32)_options range:(NSRange)range {
        let full_text = ns_string::to_rust_string(env, string);
        
        // Извлекаем подстроку согласно NSRange
        let start = range.location as usize;
        let end = (range.location + range.length) as usize;
        
        if start > full_text.len() || end > full_text.len() {
            return 0;
        }
        
        let target_text = &full_text[start..end];
        let host_obj = env.objc.borrow::<NSRegularExpressionHostObject>(this);
        
        if let Some(re) = &host_obj.regex {
            re.find_iter(target_text).count() as NSUInteger
        } else {
            0
        }
    }

    @end
};

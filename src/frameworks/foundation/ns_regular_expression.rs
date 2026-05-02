/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::{ns_string, NSRange, NSUInteger, NSNotFound}; // Добавили NSNotFound
use crate::objc::{id, objc_classes, HostObject, ClassExports, NSZonePtr, nil, autorelease}; // Добавили autorelease
use crate::Environment;
use regex::Regex;

/// Хост-объект для хранения скомпилированного регулярного выражения
struct NSRegularExpressionHostObject {
    regex: Option<Regex>,
}
impl HostObject for NSRegularExpressionHostObject {}

/// Хост-объект для хранения результата поиска
struct NSTextCheckingResultHostObject {
    ranges: Vec<NSRange>,
}
impl HostObject for NSTextCheckingResultHostObject {}

pub const CLASSES: ClassExports = objc_classes! {
    (env, this, _cmd);

    @implementation NSRegularExpression: NSObject

    + (id)allocWithZone:(NSZonePtr)_zone {
        let host_object = Box::new(NSRegularExpressionHostObject { regex: None });
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    - (id)initWithPattern:(id)pattern options:(u32)_options error:(id)_error {
        let pattern_str = ns_string::to_rust_string(env, pattern);
        let compiled = Regex::new(&pattern_str);
        
        let mut host_obj = env.objc.borrow_mut::<NSRegularExpressionHostObject>(this);
        match compiled {
            Ok(re) => {
                host_obj.regex = Some(re);
                this
            }
            Err(e) => {
                log!("NSRegularExpression: failed to compile pattern '{}': {}", pattern_str, e);
                nil
            }
        }
    }

    // ИСПРАВЛЕНИЕ: Реализация первого совпадения
    - (id)firstMatchInString:(id)string options:(u32)_options range:(NSRange)range {
        let full_text = ns_string::to_rust_string(env, string);
        let start = range.location as usize;
        let end = (range.location + range.length) as usize;

        if start > full_text.len() || end > full_text.len() {
            return nil;
        }

        let target_text = &full_text[start..end];
        let host_obj = env.objc.borrow::<NSRegularExpressionHostObject>(this);

        if let Some(re) = &host_obj.regex {
            // Используем captures, чтобы поддержать не только всё совпадение, но и скобки
            if let Some(caps) = re.captures(target_text) {
                let mut ranges = Vec::new();
                for i in 0..caps.len() {
                    if let Some(m) = caps.get(i) {
                        ranges.push(NSRange {
                            location: (range.location as usize + m.start()) as NSUInteger,
                            length: (m.end() - m.start()) as NSUInteger,
                        });
                    } else {
                        ranges.push(NSRange { location: NSNotFound, length: 0 });
                    }
                }
                
                // Создаем объект результата
                let result_host = Box::new(NSTextCheckingResultHostObject { ranges });
                let class = env.objc.get_known_class("NSTextCheckingResult", &mut env.mem);
                let result = env.objc.alloc_object(class, result_host, &mut env.mem);
                return autorelease(env, result);
            }
        }
        nil
    }

    - (NSUInteger)numberOfMatchesInString:(id)string options:(u32)_options range:(NSRange)range {
        let full_text = ns_string::to_rust_string(env, string);
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

    // Вспомогательный класс для возврата результатов
    @implementation NSTextCheckingResult: NSObject

    - (NSRange)range {
        let host_obj = env.objc.borrow::<NSTextCheckingResultHostObject>(this);
        host_obj.ranges.first().cloned().unwrap_or(NSRange { location: NSNotFound, length: 0 })
    }

    - (NSRange)rangeAtIndex:(NSUInteger)idx {
        let host_obj = env.objc.borrow::<NSTextCheckingResultHostObject>(this);
        host_obj.ranges.get(idx as usize).cloned().unwrap_or(NSRange { location: NSNotFound, length: 0 })
    }

    - (NSUInteger)numberOfRanges {
        let host_obj = env.objc.borrow::<NSTextCheckingResultHostObject>(this);
        host_obj.ranges.len() as NSUInteger
    }

    @end
};

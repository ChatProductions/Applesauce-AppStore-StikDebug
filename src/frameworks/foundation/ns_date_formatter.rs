/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSDateFormatter`.
//!
//! Resources:
//! - Apple's [Introduction to Data Formatting Programming Guide For Cocoa](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/DataFormatting/DataFormatting.html)
//! - [Unicode Technical Standard #35](https://unicode.org/reports/tr35/tr35-10.html#Date_Format_Patterns)

use crate::frameworks::core_foundation::time::CFAbsoluteTimeGetGregorianDate;
use crate::frameworks::foundation::{ns_string, NSTimeInterval};
use crate::objc::{autorelease, id, msg, nil, objc_classes, ClassExports, HostObject, NSZonePtr};

struct NSDateFormatterHostObject {
    date_format: Option<id>,
}
impl HostObject for NSDateFormatterHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSDateFormatter: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSDateFormatterHostObject {
        date_format: None,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setDateFormat:(id)format { // NSString *
    let date_format: id = msg![env; format copy];
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).date_format = Some(date_format);
}

- (id)stringFromDate:(id)date {
    let &NSDateFormatterHostObject {
        date_format
    } = env.objc.borrow(this);
    
    let format_str = ns_string::to_rust_string(env, date_format.unwrap()).to_string();
    log_dbg!("date_format before: {:?}", format_str);

    let ti: NSTimeInterval = msg![env; date timeIntervalSinceReferenceDate];
    // Получаем время в UTC (т.к. передаем nil вместо таймзоны)
    let greg_date = CFAbsoluteTimeGetGregorianDate(env, ti, nil);
    
    let mut result = String::new();
    let mut chars = format_str.chars().peekable();
    let mut in_quotes = false;

    while let Some(c) = chars.next() {
        // Обработка экранирования через одинарные кавычки
        if c == '\'' {
            if chars.peek() == Some(&'\'') {
                // Два апострофа подряд означают один реальный апостроф в строке
                result.push('\'');
                chars.next();
            } else {
                // Переключаем режим "внутри кавычек"
                in_quotes = !in_quotes;
            }
            continue;
        }

        // Если мы внутри кавычек (например 'T'), просто добавляем символ как есть
        if in_quotes {
            result.push(c);
            continue;
        }

        // Если символ - буква, собираем токен (например: yyyy, MM, ZZZZ)
        if c.is_ascii_alphabetic() {
            let mut token = String::from(c);
            while let Some(&next_c) = chars.peek() {
                if next_c == c {
                    token.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            // Подставляем значения согласно стандарту Unicode TR35
            match token.as_str() {
                "yyyy" | "YYYY" => result.push_str(&format!("{:04}", greg_date.year)),
                "MM" => result.push_str(&format!("{:02}", greg_date.month)),
                "M" => result.push_str(&format!("{}", greg_date.month)),
                "dd" => result.push_str(&format!("{:02}", greg_date.day)),
                "d" => result.push_str(&format!("{}", greg_date.day)),
                "HH" => result.push_str(&format!("{:02}", greg_date.hours)),
                "H" => result.push_str(&format!("{}", greg_date.hours)),
                "mm" => result.push_str(&format!("{:02}", greg_date.minutes)),
                "m" => result.push_str(&format!("{}", greg_date.minutes)),
                "ss" => result.push_str(&format!("{:02}", greg_date.seconds)),
                "s" => result.push_str(&format!("{}", greg_date.seconds)),
                
                // Правильная реализация таймзон (UTC/GMT)
                "Z" | "ZZ" | "ZZZ" => result.push_str("+0000"),
                "ZZZZ" => result.push_str("GMT+00:00"),
                "ZZZZZ" => result.push_str("+00:00"),
                "z" | "zz" | "zzz" | "zzzz" => result.push_str("GMT"),
                
                // Если игре нужен паттерн, которого тут нет, она упадет здесь
                _ => unimplemented!("date string contains unsubstituted format pattern: {}", token),
            }
        } else {
            // Разделители (пробелы, тире, двоеточия) оставляем как есть
            result.push(c);
        }
    }

    log_dbg!("date_format after: {:?}", result);

    let res = ns_string::from_rust_string(env, result);
    autorelease(env, res)
}

@end

};

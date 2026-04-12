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
    let greg_date = CFAbsoluteTimeGetGregorianDate(env, ti, nil);
    
    // Копируем поля упакованной структуры в локальные переменные,
    // чтобы избежать ошибки E0793 (unaligned reference) в макросе format!
    let year = greg_date.year;
    let month = greg_date.month;
    let day = greg_date.day;
    let hours = greg_date.hours;
    let minutes = greg_date.minutes;
    let seconds = greg_date.seconds;
    
    let mut result = String::new();
    let mut chars = format_str.chars().peekable();
    let mut in_quotes = false;

    while let Some(c) = chars.next() {
        if c == '\'' {
            if chars.peek() == Some(&'\'') {
                result.push('\'');
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
            continue;
        }

        if in_quotes {
            result.push(c);
            continue;
        }

        if c.is_ascii_alphabetic() {
            let mut token = String::from(c);
            while let Some(&next_c) = chars.peek() {
                if next_c == c {
                    token.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            match token.as_str() {
                "yyyy" | "YYYY" => result.push_str(&format!("{:04}", year)),
                "MM" => result.push_str(&format!("{:02}", month)),
                "M" => result.push_str(&format!("{}", month)),
                "dd" => result.push_str(&format!("{:02}", day)),
                "d" => result.push_str(&format!("{}", day)),
                "HH" => result.push_str(&format!("{:02}", hours)),
                "H" => result.push_str(&format!("{}", hours)),
                "mm" => result.push_str(&format!("{:02}", minutes)),
                "m" => result.push_str(&format!("{}", minutes)),
                "ss" => result.push_str(&format!("{:02}", seconds)),
                "s" => result.push_str(&format!("{}", seconds)),
                
                "Z" | "ZZ" | "ZZZ" => result.push_str("+0000"),
                "ZZZZ" => result.push_str("GMT+00:00"),
                "ZZZZZ" => result.push_str("+00:00"),
                "z" | "zz" | "zzz" | "zzzz" => result.push_str("GMT"),
                
                _ => unimplemented!("date string contains unsubstituted format pattern: {}", token),
            }
        } else {
            result.push(c);
        }
    }

    log_dbg!("date_format after: {:?}", result);

    let res = ns_string::from_rust_string(env, result);
    autorelease(env, res)
}

@end

};


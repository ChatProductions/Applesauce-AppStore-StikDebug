/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 */
//!
//! `NSNumberFormatter` - formats numbers into strings and parses strings into numbers.

use crate::frameworks::foundation::NSUInteger;
use crate::frameworks::foundation::ns_string::{from_rust_string, to_rust_string};
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, ClassExports, HostObject, NSZonePtr,
};

// =========================================================================
// MARK: - NSNumberFormatter Host Object
// =========================================================================

struct NSNumberFormatterHostObject {
    number_style: NSUInteger,
    locale: id,
    grouping_separator: id,
    uses_grouping_separator: bool,
    minimum_fraction_digits: NSUInteger,
    maximum_fraction_digits: NSUInteger,
}
impl HostObject for NSNumberFormatterHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSNumberFormatter : NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSNumberFormatterHostObject {
        number_style: 0, // По умолчанию NSNumberFormatterNoStyle
        locale: nil,
        grouping_separator: nil,
        uses_grouping_separator: false,
        minimum_fraction_digits: 0,
        maximum_fraction_digits: 0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

// =========================================================================
// Установка и чтение стиля форматирования
// =========================================================================

- (NSUInteger)numberStyle {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).number_style
}

- (())setNumberStyle:(NSUInteger)style {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).number_style = style;
}

// =========================================================================
// Добавленные свойства (Locale, Grouping Separator, Fraction Digits)
// =========================================================================

- (id)locale {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).locale
}

- (())setLocale:(id)locale {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).locale = locale;
}

- (id)groupingSeparator {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).grouping_separator
}

- (())setGroupingSeparator:(id)separator {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).grouping_separator = separator;
}

- (bool)usesGroupingSeparator {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).uses_grouping_separator
}

- (())setUsesGroupingSeparator:(bool)uses {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).uses_grouping_separator = uses;
}

- (NSUInteger)minimumFractionDigits {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).minimum_fraction_digits
}

- (())setMinimumFractionDigits:(NSUInteger)digits {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).minimum_fraction_digits = digits;
}

- (NSUInteger)maximumFractionDigits {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).maximum_fraction_digits
}

- (())setMaximumFractionDigits:(NSUInteger)digits {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).maximum_fraction_digits = digits;
}

// =========================================================================
// Главные методы: конвертация
// =========================================================================

// Главный метод: конвертация объекта NSNumber в NSString
- (id)stringFromNumber:(id)number {
    if number == nil {
        return nil;
    }

    [span_1](start_span)// Запрашиваем у NSNumber его значение в виде f64 (double)[span_1](end_span)
    let val: f64 = msg![env; number doubleValue];
    let host_obj = env.objc.borrow::<NSNumberFormatterHostObject>(this);
    let style = host_obj.number_style;

    // Стили в Objective-C:
    [span_2](start_span)// 0 = NoStyle, 1 = DecimalStyle, 2 = CurrencyStyle, 3 = PercentStyle, 4 = ScientificStyle[span_2](end_span)
    let rust_string = match style {
        1 => format!("{}", val), // Decimal: обычное десятичное число
        [span_3](start_span)2 => format!("${:.2}", val), // Currency: добавляем знак доллара и 2 знака после запятой[span_3](end_span)
        [span_4](start_span)3 => format!("{}%", val * 100.0), // Percent: умножаем на 100 и добавляем %[span_4](end_span)
        [span_5](start_span)4 => format!("{:e}", val), // Scientific: экспоненциальная запись[span_5](end_span)
        _[span_6](start_span) => format!("{}", val), // NoStyle или неизвестный стиль[span_6](end_span)
    };

    [span_7](start_span)// Конвертируем строку Rust обратно в объект NSString[span_7](end_span)
    from_rust_string(env, rust_string)
}

// Обратный метод: конвертация NSString в NSNumber
- (id)numberFromString:(id)string {
    if string == nil {
        return nil;
    }

    let rust_str = to_rust_string(env, string);
    
    [span_8](start_span)// Очищаем строку от знаков валюты и процентов, чтобы Rust мог её распарсить[span_8](end_span)
    let clean_str = rust_str.replace("$", "").replace(",", "").replace("%", "").trim().to_string();

    [span_9](start_span)// Пытаемся распарсить строку в f64[span_9](end_span)
    if let Ok(val) = clean_str.parse::<f64>() {
        let ns_number_class = env.objc.get_known_class("NSNumber", &mut env.mem);
        [span_10](start_span)// Создаем новый объект NSNumber[span_10](end_span)
        msg![env; ns_number_class numberWithDouble:val]
    } else {
        log!("Warning: NSNumberFormatter failed to parse string '{}'", rust_str);
        nil
    }
}

@end

};

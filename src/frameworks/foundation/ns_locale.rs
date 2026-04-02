/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSLocale`.

use super::{ns_array, ns_string};
use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_foundation::cf_locale::kCFLocaleCountryCode;
use crate::objc::{id, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr};
use crate::window::{get_preferred_country_codes, get_preferred_language_codes};
use crate::Environment;

const NSLocaleCountryCode: &str = "NSLocaleCountryCode";
const NSLocaleIdentifier: &str = "kCFLocaleIdentifierKey";

pub const CONSTANTS: ConstantExports = &[
    (
        "_NSLocaleCountryCode",
        HostConstant::NSString(NSLocaleCountryCode),
    ),
    (
        "_NSLocaleIdentifier",
        HostConstant::NSString(NSLocaleIdentifier),
    ),
];

#[derive(Default)]
pub struct State {
    current_locale: Option<id>,
    system_locale: Option<id>,
    preferred_languages: Option<id>,
}
impl State {
    fn get(env: &mut Environment) -> &mut State {
        &mut env.framework_state.foundation.ns_locale
    }
}

/// Use `msg_class![env; NSLocale preferredLanguages]` rather than calling this
/// directly, because it may be slow and there is no caching.
fn get_preferred_languages(env: &mut Environment) -> Vec<String> {
    let options = env.options.as_ref();
    if let Some(ref preferred_languages) = options.preferred_languages {
        log!("The app requested your preferred languages. {:?} will reported based on your --preferred-languages= option.", preferred_languages);
        return preferred_languages.clone();
    }

    let languages = get_preferred_language_codes(env);
    if languages.is_empty() {
        let lang = "en".to_string();
        log!("The app requested your preferred languages. No information could be retrieved, so {:?} (English) will be reported.", lang);
        vec![lang]
    } else {
        log!("The app requested your preferred languages. {:?} will be reported based on your system language preferences.", languages);
        languages
    }
}

fn get_preferred_countries(env: &mut Environment) -> Vec<String> {
    let countries = get_preferred_country_codes(env);
    if countries.is_empty() {
        let country = "US".to_string();
        log!("The app requested your current locale. No country information could be retrieved, so {:?} will be reported.", country);
        vec![country]
    } else {
        log!("The app requested your current locale. {:?} will be reported based on your system region settings.", countries);
        countries
    }
}

/// Extract the language subtag from a locale identifier.
/// Handles "ru", "ru_RU", "ru-RU", "en_US" etc.
fn language_from_locale_identifier(identifier: &str) -> &str {
    let sep = identifier.find('_').or_else(|| identifier.find('-'));
    match sep {
        Some(idx) => &identifier[..idx],
        None => identifier,
    }
}

struct NSLocaleHostObject {
    /// `NSString *`
    country_code: id,
    /// `NSString *`
    language_code: id,
}
impl HostObject for NSLocaleHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSLocale: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSLocaleHostObject {
        country_code: nil,
        language_code: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// The documentation is not clear about what the format of the strings should
// be, but Super Monkey Ball does isEqualToString against "fr", "es", "de",
// "it" and "ja", and its locale detection works properly, so presumably they
// do not usually have region suffixes.
+ (id)preferredLanguages {
    if let Some(existing) = State::get(env).preferred_languages {
        existing
    } else {
        let langs = get_preferred_languages(env);
        let lang_ns_strings = langs.into_iter().map(|lang| ns_string::from_rust_string(env, lang)).collect();
        let new = ns_array::from_vec(env, lang_ns_strings);
        State::get(env).preferred_languages = Some(new);
        new
    }
}

+ (id)currentLocale {
    if let Some(locale) = State::get(env).current_locale {
        locale
    } else {
        let countries = get_preferred_countries(env);
        let country_code = ns_string::from_rust_string(env, countries[0].clone());
        let languages = get_preferred_languages(env);
        let language_code = ns_string::from_rust_string(env, languages[0].clone());
        let host_object = NSLocaleHostObject {
            country_code,
            language_code,
        };
        let new_locale = env.objc.alloc_object(
            this,
            Box::new(host_object),
            &mut env.mem
        );
        State::get(env).current_locale = Some(new_locale);
        new_locale
    }
}

+ (id)systemLocale {
    if let Some(locale) = State::get(env).system_locale {
        locale
    } else {
        let host_object = NSLocaleHostObject {
            // Was confirmed on the iOS Simulator
            country_code: nil,
            language_code: nil,
        };
        let new_locale = env.objc.alloc_object(
            this,
            Box::new(host_object),
            &mut env.mem
        );
        State::get(env).system_locale = Some(new_locale);
        new_locale
    }
}

+ (id)ISOLanguageCodes {
    // These are common codes. A full list is quite large, so we provide the primary ones.
    let codes = vec![
        "en", "fr", "de", "zh", "ja", "es", "it", "ko", "pt", "ru", "nl", "tr"
    ];
    let ns_codes = codes.into_iter()
        .map(|c| ns_string::from_rust_string(env, c.to_string()))
        .collect();
    ns_array::from_vec(env, ns_codes)
}

// Returns a list of all available ISO country codes (ISO 3166-1).
+ (id)ISOCountryCodes {
    let codes = vec![
        "US", "CA", "GB", "DE", "FR", "CN", "JP", "KR", "BR", "AU", "RU", "IN"
    ];
    let ns_codes = codes.into_iter()
        .map(|c| ns_string::from_rust_string(env, c.to_string()))
        .collect();
    ns_array::from_vec(env, ns_codes)
}

// Returns a list of all available ISO currency codes (ISO 4217).
+ (id)ISOCurrencyCodes {
    let codes = vec![
        "USD", "EUR", "JPY", "GBP", "AUD", "CAD", "CHF", "CNY", "HKD", "NZD"
    ];
    let ns_codes = codes.into_iter()
        .map(|c| ns_string::from_rust_string(env, c.to_string()))
        .collect();
    ns_array::from_vec(env, ns_codes)
}

// Returns a list of common ISO currency codes (usually the same as ISOCurrencyCodes).
+ (id)commonISOCurrencyCodes {
    msg_send![this, ISOCurrencyCodes]
}

// A helper to convert a locale identifier to a canonical form.
+ (id)canonicalLocaleIdentifierFromString:(id)string {
    let raw = ns_string::to_rust_string(env, string);
    // Canonical format usually replaces hyphens with underscores: "en-us" -> "en_US"
    let canonical = raw.replace('-', "_");
    // In a full implementation, we'd handle case sensitivity (lang low, region up)
    ns_string::from_rust_string(env, canonical)
}

// A helper to convert a language identifier to a canonical form.
+ (id)canonicalLanguageIdentifierFromString:(id)string {
    let raw = ns_string::to_rust_string(env, string);
    // e.g., "English" -> "en" or "en-US" -> "en"
    let lang = language_from_locale_identifier(&raw);
    ns_string::from_rust_string(env, lang.to_lowercase())
}

- (id)initWithLocaleIdentifier:(id)string { // NSString *
    let str = ns_string::to_rust_string(env, string);
    log_dbg!("[(NSLocale *){:?} initWithLocaleIdentifier:'{}']", this, str);
    
    // In a real implementation, identifiers like "en_US" would set both.
    // For now, we extract the language and store the full identifier if needed.
    let lang = language_from_locale_identifier(&str).to_string();
    let lang_ns_string = ns_string::from_rust_string(env, lang);
    
    // If the identifier contains an underscore, try to extract country
    let country_ns_string = if let Some(idx) = str.find('_') {
        let country = &str[idx + 1..];
        ns_string::from_rust_string(env, country.to_string())
    } else {
        nil
    };

    let host = env.objc.borrow_mut::<NSLocaleHostObject>(this);
    host.language_code = lang_ns_string;
    host.country_code = country_ns_string;
    
    this
}

// Returns the identifier for the locale.
- (id)localeIdentifier {
    let &NSLocaleHostObject { language_code, country_code } = env.objc.borrow(this);
    if country_code == nil {
        return language_code;
    }
    
    // Combine: e.g., "en" + "_" + "US"
    let lang = ns_string::to_rust_string(env, language_code);
    let country = ns_string::to_rust_string(env, country_code);
    ns_string::from_rust_string(env, format!("{}_{}", lang, country))
}

// Returns the language code of the locale.
- (id)languageCode {
    let &NSLocaleHostObject { language_code, .. } = env.objc.borrow(this);
    language_code
}

// Returns the country code of the locale.
- (id)countryCode {
    let &NSLocaleHostObject { country_code, .. } = env.objc.borrow(this);
    country_code
}

- (id)objectForKey:(id)key {
    let key_str: &str = &ns_string::to_rust_string(env, key);
    let host = env.objc.borrow::<NSLocaleHostObject>(this);
    
    match key_str {
        // NSLocaleCountryCode / kCFLocaleCountryCode
        "NSLocaleCountryCode" | "kCFLocaleCountryCode" => host.country_code,
        // NSLocaleLanguageCode / kCFLocaleLanguageCode
        "NSLocaleLanguageCode" | "kCFLocaleLanguageCode" => host.language_code,
        // NSLocaleIdentifier / kCFLocaleIdentifier
        "NSLocaleIdentifier" | "kCFLocaleIdentifierKey" => {
            // Re-use the logic from localeIdentifier method
            msg_send![this, localeIdentifier]
        },
        _ => {
            log_dbg!("NSLocale objectForKey: unknown key '{}'", key_str);
            nil
        }
    }
}

// Common helper for currency and decimal separators
- (id)decimalSeparator {
    ns_string::from_rust_string(env, ".".to_string())
}

- (id)groupingSeparator {
    ns_string::from_rust_string(env, ",".to_string())
}

- (id)currencySymbol {
    // Defaulting to USD for a safe stub; a better way would be checking country_code
    ns_string::from_rust_string(env, "$".to_string())
}

- (id)currencyCode {
    ns_string::from_rust_string(env, "USD".to_string())
}

- (id)displayNameForKey:(id)key value:(id)value {
    // A common use case is getting the name of a language in that language
    // e.g., [englishLocale displayNameForKey:NSLocaleIdentifier value:@"fr"] -> "French"
    log_dbg!("TODO: displayNameForKey for value: {}", ns_string::to_rust_string(env, value));
    value
}

- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

- (())dealloc {
    let &NSLocaleHostObject { country_code, language_code } = env.objc.borrow::<NSLocaleHostObject>(this);
    if country_code != nil { release(env, country_code); }
    if language_code != nil { release(env, language_code); }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

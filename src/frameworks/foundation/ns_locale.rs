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

pub const CONSTANTS: ConstantExports = &[(
    "_NSLocaleCountryCode",
    HostConstant::NSString(NSLocaleCountryCode),
)];

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

// The documentation isn't clear about what the format of the strings should be,
// but Super Monkey Ball does

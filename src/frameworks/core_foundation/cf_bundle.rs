/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFBundle`.
//!
//! This is not even toll-free bridged to `NSBundle` in Apple's implementation,
//! but here it is the same type.

use super::cf_array::CFArrayRef;
use super::cf_string::CFStringRef;
use super::cf_url::CFURLRef;
use super::CFTypeRef;
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::ns_bundle::NSBundleHostObject;
use crate::frameworks::foundation::{ns_array, ns_string, NSUInteger};
use crate::objc::{id, msg, msg_class, nil, retain};
use crate::Environment;

const kCFBundleVersionKey: &str = "CFBundleVersion";
const kCFBundleNameKey: &str = "CFBundleName";
const kCFBundleIdentifierKey: &str = "CFBundleIdentifier";
const kCFBundleExecutableKey: &str = "CFBundleExecutable";
const kCFBundleInfoDictionaryVersionKey: &str = "CFBundleInfoDictionaryVersion";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCFBundleVersionKey",
        HostConstant::NSString(kCFBundleVersionKey),
    ),
    (
        "_kCFBundleNameKey",
        HostConstant::NSString(kCFBundleNameKey),
    ),
    (
        "_kCFBundleIdentifierKey",
        HostConstant::NSString(kCFBundleIdentifierKey),
    ),
    (
        "_kCFBundleExecutableKey",
        HostConstant::NSString(kCFBundleExecutableKey),
    ),
    (
        "_kCFBundleInfoDictionaryVersionKey",
        HostConstant::NSString(kCFBundleInfoDictionaryVersionKey),
    ),
];

pub type CFBundleRef = CFTypeRef;

// MARK: - Bundle access

fn CFBundleGetMainBundle(env: &mut Environment) -> CFBundleRef {
    msg_class![env; NSBundle mainBundle]
}

fn CFBundleGetBundleWithIdentifier(
    env: &mut Environment,
    bundle_id: CFStringRef,
) -> CFBundleRef {
    // NSBundle doesn't expose a bundleWithIdentifier: equivalent in our
    // implementation, so check the main bundle's identifier and return it if
    // it matches; otherwise return nil.
    let main: CFBundleRef = msg_class![env; NSBundle mainBundle];
    let main_id: id = CFBundleGetIdentifier(env, main);
    if main_id == nil || bundle_id == nil {
        return nil;
    }
    let equal: bool = msg![env; main_id isEqualToString:bundle_id];
    if equal { main } else { nil }
}

// MARK: - Info dictionary

fn CFBundleGetValueForInfoDictionaryKey(
    env: &mut Environment,
    bundle: CFBundleRef,
    key: CFStringRef,
) -> CFTypeRef {
    let dict: id = msg![env; bundle infoDictionary];
    msg![env; dict objectForKey:key]
}

fn CFBundleCopyInfoDictionaryForURL(
    env: &mut Environment,
    url: CFURLRef,
) -> CFTypeRef {
    // Load the bundle at the given URL and return its info dictionary.
    if url == nil {
        return nil;
    }
    let bundle: id = msg_class![env; NSBundle bundleWithURL:url];
    if bundle == nil {
        return nil;
    }
    let dict: id = msg![env; bundle infoDictionary];
    msg![env; dict copy]
}

// MARK: - Common info-key helpers

fn CFBundleGetIdentifier(env: &mut Environment, bundle: CFBundleRef) -> CFStringRef {
    let key: id = ns_string::get_static_str(env, kCFBundleIdentifierKey);
    CFBundleGetValueForInfoDictionaryKey(env, bundle, key)
}

fn CFBundleCopyBundleIdentifier(env: &mut Environment, bundle: CFBundleRef) -> CFStringRef {
    let id_str = CFBundleGetIdentifier(env, bundle);
    if id_str == nil {
        return nil;
    }
    msg![env; id_str copy]
}

fn CFBundleGetVersionNumber(env: &mut Environment, bundle: CFBundleRef) -> u32 {
    let version_key: id = ns_string::get_static_str(env, kCFBundleVersionKey);
    let vers: id = CFBundleGetValueForInfoDictionaryKey(env, bundle, version_key);
    if vers == nil {
        return 0;
    }
    let vers_str = ns_string::to_rust_string(env, vers);
    log_dbg!("CFBundleGetVersionNumber {}", vers_str);

    let parts: Vec<&str> = vers_str.split('.').collect();
    assert!(parts.len() <= 3);

    let mut result: u32 = 1 << 15;
    let major: u32 = parts[0].parse().unwrap_or(0);
    assert!(major <= 99);
    result |= (major / 10) << 28;
    result |= (major % 10) << 24;
    if parts.len() >= 2 {
        let minor: u32 = parts[1].parse().unwrap_or(0);
        assert!(minor <= 9);
        result |= minor << 20;
    }
    if parts.len() == 3 {
        let bug_fix: u32 = parts[2].parse().unwrap_or(0);
        assert!(bug_fix <= 9);
        result |= bug_fix << 16;
    }
    result
}

fn CFBundleCopyShortVersionString(env: &mut Environment, bundle: CFBundleRef) -> CFStringRef {
    let key: id = ns_string::get_static_str(env, "CFBundleShortVersionString");
    let val: id = CFBundleGetValueForInfoDictionaryKey(env, bundle, key);
    if val == nil {
        return nil;
    }
    msg![env; val copy]
}

// MARK: - URLs

fn CFBundleCopyBundleURL(env: &mut Environment, bundle: CFBundleRef) -> CFURLRef {
    let url: CFURLRef = msg![env; bundle bundleURL];
    msg![env; url copy]
}

fn CFBundleCopyResourcesDirectoryURL(env: &mut Environment, bundle: CFBundleRef) -> CFURLRef {
    let url: CFURLRef = msg![env; bundle resourceURL];
    msg![env; url copy]
}

fn CFBundleCopyExecutableURL(env: &mut Environment, bundle: CFBundleRef) -> CFURLRef {
    let url: CFURLRef = msg![env; bundle executableURL];
    if url == nil {
        return nil;
    }
    msg![env; url copy]
}

fn CFBundleCopyPrivateFrameworksURL(env: &mut Environment, bundle: CFBundleRef) -> CFURLRef {
    let url: CFURLRef = msg![env; bundle privateFrameworksURL];
    if url == nil {
        return nil;
    }
    msg![env; url copy]
}

fn CFBundleCopySharedFrameworksURL(env: &mut Environment, bundle: CFBundleRef) -> CFURLRef {
    let url: CFURLRef = msg![env; bundle sharedFrameworksURL];
    if url == nil {
        return nil;
    }
    msg![env; url copy]
}

fn CFBundleCopyBuiltInPlugInsURL(env: &mut Environment, bundle: CFBundleRef) -> CFURLRef {
    let url: CFURLRef = msg![env; bundle builtInPlugInsURL];
    if url == nil {
        return nil;
    }
    msg![env; url copy]
}

fn CFBundleCopyResourceURL(
    env: &mut Environment,
    bundle: CFBundleRef,
    resource_name: CFStringRef,
    resource_type: CFStringRef,
    sub_dir_name: CFStringRef,
) -> CFURLRef {
    let url: CFURLRef = msg![env; bundle URLForResource:resource_name
                                          withExtension:resource_type
                                           subdirectory:sub_dir_name];
    msg![env; url copy]
}

fn CFBundleCopyResourceURLInDirectory(
    env: &mut Environment,
    bundle_url: CFURLRef,
    resource_name: CFStringRef,
    resource_type: CFStringRef,
    sub_dir_name: CFStringRef,
) -> CFURLRef {
    if bundle_url == nil {
        return nil;
    }
    let bundle: id = msg_class![env; NSBundle bundleWithURL:bundle_url];
    if bundle == nil {
        return nil;
    }
    CFBundleCopyResourceURL(env, bundle, resource_name, resource_type, sub_dir_name)
}

fn CFBundleCopyResourceURLsOfType(
    env: &mut Environment,
    bundle: CFBundleRef,
    resource_type: CFStringRef,
    sub_dir_name: CFStringRef,
) -> CFArrayRef {
    msg![env; bundle URLsForResourcesWithExtension:resource_type subdirectory:sub_dir_name]
}

// MARK: - Localizations

pub fn CFBundleCopyBundleLocalizations(env: &mut Environment, bundle: CFBundleRef) -> CFArrayRef {
    let bundle_localizations = env
        .objc
        .borrow_mut::<NSBundleHostObject>(bundle)
        .bundle
        .as_ref()
        .unwrap_or(&env.bundle)
        .bundle_localizations()
        .iter()
        .map(|value| value.as_string().unwrap().to_string())
        .collect::<Vec<String>>();
    let guest_bundle_localizations = bundle_localizations
        .iter()
        .map(|loc| ns_string::from_rust_string(env, loc.to_owned()))
        .collect::<Vec<id>>();
    let loc_array = ns_array::from_vec(env, guest_bundle_localizations);
    log_dbg!(
        "CFBundleCopyBundleLocalizations({:?}) => {:?} ({})",
        bundle,
        loc_array,
        bundle_localizations.join(", ")
    );
    loc_array
}

pub fn CFBundleCopyPreferredLocalizationsFromArray(
    env: &mut Environment,
    loc_array: CFArrayRef,
) -> CFArrayRef {
    let mut result = Vec::new();

    let preferred_languages: id = msg_class![env; NSLocale preferredLanguages];

    let loc_count: NSUInteger = msg![env; loc_array count];
    let pref_loc_count: NSUInteger = msg![env; preferred_languages count];
    for loc_index in 0..loc_count {
        let loc: id = msg![env; loc_array objectAtIndex:loc_index];
        for pref_loc_index in 0..pref_loc_count {
            let pref_loc: id = msg![env; preferred_languages objectAtIndex:pref_loc_index];
            let equal: bool = msg![env; loc isEqualToString:pref_loc];
            if equal {
                result.push(loc);
                retain(env, loc);
                break;
            }
        }
    }

    if loc_count > 0 {
        // Add the first element as fallback.
        let first_loc: id = msg![env; loc_array objectAtIndex:(0 as NSUInteger)];
        result.push(first_loc);
        retain(env, first_loc);
    } else {
        // Behaviour was verified on macOS.
        let en_loc = ns_string::get_static_str(env, "en");
        result.push(en_loc);
    };

    let result = ns_array::from_vec(env, result);
    log_dbg!(
        "CFBundleCopyPreferredLocalizationsFromArray({:?}) => {:?}",
        loc_array,
        result
    );
    result
}

fn CFBundleCopyLocalizedString(
    env: &mut Environment,
    bundle: CFBundleRef,
    key: CFStringRef,
    value: CFStringRef,
    table_name: CFStringRef,
) -> CFStringRef {
    let res = msg![env; bundle localizedStringForKey:key value:value table:table_name];
    msg![env; res copy]
}

// MARK: - Load state

fn CFBundleIsExecutableLoaded(env: &mut Environment, bundle: CFBundleRef) -> bool {
    // In touchHLE the guest executable is always considered loaded.
    let _ = bundle;
    true
}

fn CFBundlePreflightExecutable(
    env: &mut Environment,
    bundle: CFBundleRef,
    _error: id, // CFErrorRef* — ignored
) -> bool {
    // We don't support dynamic loading; report success for the main bundle,
    // false for anything else.
    let main: CFBundleRef = CFBundleGetMainBundle(env);
    bundle == main
}

fn CFBundleLoadExecutable(env: &mut Environment, bundle: CFBundleRef) -> bool {
    log!("TODO: CFBundleLoadExecutable({:?}) — returning false", bundle);
    false
}

fn CFBundleUnloadExecutable(env: &mut Environment, bundle: CFBundleRef) {
    log!("TODO: CFBundleUnloadExecutable({:?}) — ignored", bundle);
}

// MARK: - Function lookup stub

fn CFBundleGetFunctionPointerForName(
    env: &mut Environment,
    bundle: CFBundleRef,
    function_name: CFStringRef,
) -> CFTypeRef /* void* */ {
    let name = ns_string::to_rust_string(env, function_name);
    log!(
        "TODO: CFBundleGetFunctionPointerForName({:?}, {:?}) — returning NULL",
        bundle,
        name
    );
    nil
}

pub const FUNCTIONS: FunctionExports = &[
    // Bundle access
    export_c_func!(CFBundleGetMainBundle()),
    export_c_func!(CFBundleGetBundleWithIdentifier(_)),
    // Info dictionary
    export_c_func!(CFBundleGetValueForInfoDictionaryKey(_, _)),
    export_c_func!(CFBundleCopyInfoDictionaryForURL(_)),
    export_c_func!(CFBundleGetIdentifier(_)),
    export_c_func!(CFBundleCopyBundleIdentifier(_)),
    export_c_func!(CFBundleGetVersionNumber(_)),
    export_c_func!(CFBundleCopyShortVersionString(_)),
    // URLs
    export_c_func!(CFBundleCopyBundleURL(_)),
    export_c_func!(CFBundleCopyResourcesDirectoryURL(_)),
    export_c_func!(CFBundleCopyExecutableURL(_)),
    export_c_func!(CFBundleCopyPrivateFrameworksURL(_)),
    export_c_func!(CFBundleCopySharedFrameworksURL(_)),
    export_c_func!(CFBundleCopyBuiltInPlugInsURL(_)),
    export_c_func!(CFBundleCopyResourceURL(_, _, _, _)),
    export_c_func!(CFBundleCopyResourceURLInDirectory(_, _, _, _)),
    export_c_func!(CFBundleCopyResourceURLsOfType(_, _, _)),
    // Localizations
    export_c_func!(CFBundleCopyBundleLocalizations(_)),
    export_c_func!(CFBundleCopyPreferredLocalizationsFromArray(_)),
    export_c_func!(CFBundleCopyLocalizedString(_, _, _, _)),
    // Load state
    export_c_func!(CFBundleIsExecutableLoaded(_)),
    export_c_func!(CFBundlePreflightExecutable(_, _)),
    export_c_func!(CFBundleLoadExecutable(_)),
    export_c_func!(CFBundleUnloadExecutable(_)),
    // Symbol lookup
    export_c_func!(CFBundleGetFunctionPointerForName(_, _)),
];

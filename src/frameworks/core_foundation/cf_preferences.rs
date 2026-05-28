/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFPreferences`.
//!
//! According to Apple's docs, it's not toll-free bridged to `NSUserDefaults`,
//! but we are still implementing one atop of another.

use super::cf_string::CFStringRef;
use super::CFTypeRef;
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::ns_string;
use crate::objc::{id, msg, msg_class, nil};
use crate::Environment;

type CFPropertyListRef = CFTypeRef;

/// Decide whether a CFPreferences `applicationID` argument refers to the
/// running app's preferences and so can be routed to `NSUserDefaults`.
///
/// In a real iOS sandbox a process can normally only read its own
/// preferences anyway, so we accept:
///
///   - `kCFPreferencesCurrentApplication` (the documented "use the current
///     app" sentinel),
///   - the running app's own bundle identifier (e.g. PopCap's PvZ2 calls
///     `CFPreferencesCopyAppValue(key, CFSTR("com.popcap.ios.PvZ2"))`
///     instead of going through the sentinel — appdb report #49),
///   - `NULL` (some apps pass nil as a shorthand for "current app").
///
/// Any other identifier produces a one-line warning and is treated as the
/// current app, so the call still degrades to a normal `NSUserDefaults`
/// lookup instead of asserting.
fn app_id_is_current(env: &mut Environment, app_id: CFStringRef) -> bool {
    if app_id == nil {
        return true;
    }
    let current_app = ns_string::get_static_str(env, kCFPreferencesCurrentApplication);
    if msg![env; app_id isEqualToString:current_app] {
        return true;
    }
    let app_id_str = ns_string::to_rust_string(env, app_id);
    let bundle_id = env.bundle.bundle_identifier();
    if app_id_str == bundle_id {
        return true;
    }
    log!(
        "Warning: CFPreferences called with applicationID {:?}, which is \
         neither kCFPreferencesCurrentApplication nor this app's bundle id \
         ({:?}); routing to NSUserDefaults anyway.",
        app_id_str,
        bundle_id,
    );
    false
}

fn CFPreferencesCopyAppValue(
    env: &mut Environment,
    key: CFStringRef,
    app_id: CFStringRef,
) -> CFPropertyListRef {
    let _ = app_id_is_current(env, app_id);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    let value: id = msg![env; user_defaults objectForKey:key];
    msg![env; value copy]
}

fn CFPreferencesSetAppValue(
    env: &mut Environment,
    key: CFStringRef,
    value: CFPropertyListRef,
    app_id: CFStringRef,
) {
    let _ = app_id_is_current(env, app_id);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    if value.is_null() {
        let _: () = msg![env; user_defaults removeObjectForKey:key];
        return;
    }
    msg![env; user_defaults setObject:value forKey:key]
}

fn CFPreferencesAppSynchronize(env: &mut Environment, app_id: CFStringRef) -> bool {
    let _ = app_id_is_current(env, app_id);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; user_defaults synchronize]
}

pub const kCFPreferencesCurrentApplication: &str = "kCFPreferencesCurrentApplication";
pub const kCFPreferencesAnyApplication: &str = "kCFPreferencesAnyApplication";
pub const kCFPreferencesAnyHost: &str = "kCFPreferencesAnyHost";
pub const kCFPreferencesCurrentHost: &str = "kCFPreferencesCurrentHost";
pub const kCFPreferencesAnyUser: &str = "kCFPreferencesAnyUser";
pub const kCFPreferencesCurrentUser: &str = "kCFPreferencesCurrentUser";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCFPreferencesCurrentApplication",
        HostConstant::NSString(kCFPreferencesCurrentApplication),
    ),
    (
        "_kCFPreferencesAnyApplication",
        HostConstant::NSString(kCFPreferencesAnyApplication),
    ),
    (
        "_kCFPreferencesAnyHost",
        HostConstant::NSString(kCFPreferencesAnyHost),
    ),
    (
        "_kCFPreferencesCurrentHost",
        HostConstant::NSString(kCFPreferencesCurrentHost),
    ),
    (
        "_kCFPreferencesAnyUser",
        HostConstant::NSString(kCFPreferencesAnyUser),
    ),
    (
        "_kCFPreferencesCurrentUser",
        HostConstant::NSString(kCFPreferencesCurrentUser),
    ),
];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFPreferencesCopyAppValue(_, _)),
    export_c_func!(CFPreferencesSetAppValue(_, _, _)),
    export_c_func!(CFPreferencesAppSynchronize(_)),
];

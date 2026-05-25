/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `Social.framework/Social`.
//!
//! On iOS 6+, Social provides `SLComposeViewController` and the
//! `SLServiceType*` string identifiers used to talk to system-level Twitter,
//! Facebook, Sina/Tencent Weibo and LinkedIn accounts.
//!
//! touchHLE only targets pre-iOS-4-era games today, but many later apps still
//! list Social as a Mach-O dependency (often pulled in transitively by an
//! analytics or sharing SDK such as Flurry or Facebook Audience Network) and
//! reference its `SLServiceType*` constants by symbol name.  Without a
//! [crate::dyld::HostDylib] entry, those constants stay NULL and any
//! guest-side `[NSString isEqualToString:SLServiceTypeFacebook]` crashes the
//! emulator with a NULL-page read.  This stub satisfies the dependency and
//! returns plain NSString constants whose values are the canonical Apple
//! identifiers.

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};
use crate::objc::{id, nil, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// `SLComposeViewController` (iOS 6.0+). Presents the system share sheet
// for a given service. touchHLE has no UIKit presentation pipeline and no
// Settings-app account configuration, so following Apple's documented
// contract we report every service as unavailable. Apps then take their
// "service is not configured" code path instead of crashing on
// `[SLComposeViewController alloc]` / `setInitialText:`.
// <https://developer.apple.com/documentation/social/slcomposeviewcontroller>
@implementation SLComposeViewController: UIViewController

// `+ (BOOL)isAvailableForServiceType:(NSString *)serviceType` — `YES` if
// the user is signed into the requested service. touchHLE has no Accounts
// framework, so always `NO`.
+ (bool)isAvailableForServiceType:(id)_service_type {
    false
}

// `+ (SLComposeViewController *)composeViewControllerForServiceType:(NSString *)serviceType`
// — Apple's docs explicitly note this returns `nil` when the service is
// unavailable. Since `+isAvailableForServiceType:` always reports `NO`,
// this returns `nil` too.
+ (id)composeViewControllerForServiceType:(id)_service_type {
    nil
}

@end

};

pub const CONSTANTS: ConstantExports = &[
    (
        "_SLServiceTypeTwitter",
        HostConstant::NSString("com.apple.social.twitter"),
    ),
    (
        "_SLServiceTypeFacebook",
        HostConstant::NSString("com.apple.social.facebook"),
    ),
    (
        "_SLServiceTypeSinaWeibo",
        HostConstant::NSString("com.apple.social.sinaweibo"),
    ),
    (
        "_SLServiceTypeTencentWeibo",
        HostConstant::NSString("com.apple.social.tencentweibo"),
    ),
    (
        "_SLServiceTypeLinkedIn",
        HostConstant::NSString("com.apple.social.linkedin"),
    ),
];

pub const FUNCTIONS: FunctionExports = &[];

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFError` global constants.
//!
//! `CFError` is toll-free bridged with `NSError`, so error domain strings
//! exposed by the C API are simply `CFStringRef` aliases for the
//! Objective-C `NSError` domain strings of the same name.
//!
//! Apple reference:
//! <https://developer.apple.com/documentation/corefoundation/cferror_ref>
//!
//! The Mach-O symbols referenced by guest binaries are `_kCFErrorDomain*`.
//! Each one is published as a CFString (NSString) whose contents match
//! the documented domain identifier so that comparisons made with
//! `CFEqual` / `-[NSString isEqualToString:]` succeed.

use crate::dyld::{ConstantExports, HostConstant};

pub const CONSTANTS: ConstantExports = &[
    // <https://developer.apple.com/documentation/corefoundation/kcferrordomainposix>
    (
        "_kCFErrorDomainPOSIX",
        HostConstant::NSString("NSPOSIXErrorDomain"),
    ),
    // <https://developer.apple.com/documentation/corefoundation/kcferrordomainosstatus>
    (
        "_kCFErrorDomainOSStatus",
        HostConstant::NSString("NSOSStatusErrorDomain"),
    ),
    // <https://developer.apple.com/documentation/corefoundation/kcferrordomainmach>
    (
        "_kCFErrorDomainMach",
        HostConstant::NSString("NSMachErrorDomain"),
    ),
    // <https://developer.apple.com/documentation/corefoundation/kcferrordomaincocoa>
    (
        "_kCFErrorDomainCocoa",
        HostConstant::NSString("NSCocoaErrorDomain"),
    ),
    // Localized-description user-info keys. These match the NSError
    // string constants because CFError is toll-free bridged with
    // NSError.
    // <https://developer.apple.com/documentation/corefoundation/kcferrorlocalizeddescriptionkey>
    (
        "_kCFErrorLocalizedDescriptionKey",
        HostConstant::NSString("NSLocalizedDescription"),
    ),
    // <https://developer.apple.com/documentation/corefoundation/kcferrorlocalizedfailurereasonkey>
    (
        "_kCFErrorLocalizedFailureReasonKey",
        HostConstant::NSString("NSLocalizedFailureReason"),
    ),
    // <https://developer.apple.com/documentation/corefoundation/kcferrorlocalizedrecoverysuggestionkey>
    (
        "_kCFErrorLocalizedRecoverySuggestionKey",
        HostConstant::NSString("NSLocalizedRecoverySuggestion"),
    ),
    // <https://developer.apple.com/documentation/corefoundation/kcferrordescriptionkey>
    (
        "_kCFErrorDescriptionKey",
        HostConstant::NSString("NSDescription"),
    ),
    // <https://developer.apple.com/documentation/corefoundation/kcferrorunderlyingerrorkey>
    (
        "_kCFErrorUnderlyingErrorKey",
        HostConstant::NSString("NSUnderlyingError"),
    ),
    // <https://developer.apple.com/documentation/corefoundation/kcferrorurlkey>
    ("_kCFErrorURLKey", HostConstant::NSString("NSURL")),
    // <https://developer.apple.com/documentation/corefoundation/kcferrorfilepathkey>
    ("_kCFErrorFilePathKey", HostConstant::NSString("NSFilePath")),
];

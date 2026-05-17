/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `CoreTelephony.framework/CoreTelephony`.
//!
//! Many iOS 5+ apps (and most analytics/ads SDKs of that era) link against
//! CoreTelephony just to read the carrier name or listen for the
//! `CTRadioAccessTechnologyDidChange` notification.  touchHLE has no real
//! cellular support, but the symbols still need to resolve to a non-NULL
//! pointer; otherwise the guest's `[CTTelephonyNetworkInfo
//! addObserver:…name:CTRadioAccessTechnologyDidChangeNotification …]`
//! dereferences NULL inside `CFStringHash` and we crash on launch.

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};

pub const CONSTANTS: ConstantExports = &[
    (
        "_CTRadioAccessTechnologyDidChangeNotification",
        HostConstant::NSString("CTRadioAccessTechnologyDidChange"),
    ),
    (
        "_CTRadioAccessTechnologyGPRS",
        HostConstant::NSString("CTRadioAccessTechnologyGPRS"),
    ),
    (
        "_CTRadioAccessTechnologyEdge",
        HostConstant::NSString("CTRadioAccessTechnologyEdge"),
    ),
    (
        "_CTRadioAccessTechnologyWCDMA",
        HostConstant::NSString("CTRadioAccessTechnologyWCDMA"),
    ),
    (
        "_CTRadioAccessTechnologyHSDPA",
        HostConstant::NSString("CTRadioAccessTechnologyHSDPA"),
    ),
    (
        "_CTRadioAccessTechnologyHSUPA",
        HostConstant::NSString("CTRadioAccessTechnologyHSUPA"),
    ),
    (
        "_CTRadioAccessTechnologyCDMA1x",
        HostConstant::NSString("CTRadioAccessTechnologyCDMA1x"),
    ),
    (
        "_CTRadioAccessTechnologyCDMAEVDORev0",
        HostConstant::NSString("CTRadioAccessTechnologyCDMAEVDORev0"),
    ),
    (
        "_CTRadioAccessTechnologyCDMAEVDORevA",
        HostConstant::NSString("CTRadioAccessTechnologyCDMAEVDORevA"),
    ),
    (
        "_CTRadioAccessTechnologyCDMAEVDORevB",
        HostConstant::NSString("CTRadioAccessTechnologyCDMAEVDORevB"),
    ),
    (
        "_CTRadioAccessTechnologyeHRPD",
        HostConstant::NSString("CTRadioAccessTechnologyeHRPD"),
    ),
    (
        "_CTRadioAccessTechnologyLTE",
        HostConstant::NSString("CTRadioAccessTechnologyLTE"),
    ),
];

pub const FUNCTIONS: FunctionExports = &[];

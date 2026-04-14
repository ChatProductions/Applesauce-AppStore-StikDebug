/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Security framework — Keychain Services stubs.
//!
//! Only the minimum needed to prevent null-pointer crashes is implemented.
//! All Keychain queries return `errSecItemNotFound`.

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant, HostDylib};
use crate::frameworks::core_foundation::cf_dictionary::CFDictionaryRef;
use crate::frameworks::core_foundation::CFTypeRef;
use crate::mem::MutPtr;
use crate::objc::nil;
use crate::Environment;

// OSStatus result codes (Security/SecBase.h)
pub type OSStatus = i32;
pub const ERR_SEC_SUCCESS: OSStatus = 0;
pub const ERR_SEC_ITEM_NOT_FOUND: OSStatus = -25300;

// MARK: - Keychain Services functions

fn SecItemCopyMatching(
    env: &mut Environment,
    _query: CFDictionaryRef,
    result: MutPtr<CFTypeRef>,
) -> OSStatus {
    log!("TODO: SecItemCopyMatching (stub, returning errSecItemNotFound)");
    if !result.is_null() {
        env.mem.write(result, nil);
    }
    ERR_SEC_ITEM_NOT_FOUND
}

fn SecItemAdd(
    _env: &mut Environment,
    _attributes: CFDictionaryRef,
    _result: MutPtr<CFTypeRef>,
) -> OSStatus {
    log!("TODO: SecItemAdd (stub, returning errSecSuccess)");
    ERR_SEC_SUCCESS
}

fn SecItemDelete(
    _env: &mut Environment,
    _query: CFDictionaryRef,
) -> OSStatus {
    log!("TODO: SecItemDelete (stub, returning errSecItemNotFound)");
    ERR_SEC_ITEM_NOT_FOUND
}

fn SecItemUpdate(
    _env: &mut Environment,
    _query: CFDictionaryRef,
    _attributes_to_update: CFDictionaryRef,
) -> OSStatus {
    log!("TODO: SecItemUpdate (stub, returning errSecItemNotFound)");
    ERR_SEC_ITEM_NOT_FOUND
}

// MARK: - Exports

// kSec* constants are CFString (toll-free bridged NSString) values used as
// keys in dictionaries passed to SecItem* functions.
// The actual string content is not significant since our SecItem* stubs
// ignore all dictionary contents; we only need non-null pointers.
pub const CONSTANTS: ConstantExports = &[
    // kSecClass and its values
    ("_kSecClass",
        HostConstant::NSString("kSecClass")),
    ("_kSecClassGenericPassword",
        HostConstant::NSString("kSecClassGenericPassword")),
    ("_kSecClassInternetPassword",
        HostConstant::NSString("kSecClassInternetPassword")),
    ("_kSecClassCertificate",
        HostConstant::NSString("kSecClassCertificate")),
    ("_kSecClassKey",
        HostConstant::NSString("kSecClassKey")),
    ("_kSecClassIdentity",
        HostConstant::NSString("kSecClassIdentity")),
    // Attribute keys
    ("_kSecAttrAccessGroup",
        HostConstant::NSString("kSecAttrAccessGroup")),
    ("_kSecAttrAccessible",
        HostConstant::NSString("kSecAttrAccessible")),
    ("_kSecAttrAccount",
        HostConstant::NSString("kSecAttrAccount")),
    ("_kSecAttrDescription",
        HostConstant::NSString("kSecAttrDescription")),
    ("_kSecAttrGeneric",
        HostConstant::NSString("kSecAttrGeneric")),
    ("_kSecAttrLabel",
        HostConstant::NSString("kSecAttrLabel")),
    ("_kSecAttrService",
        HostConstant::NSString("kSecAttrService")),
    ("_kSecAttrServer",
        HostConstant::NSString("kSecAttrServer")),
    ("_kSecAttrCreationDate",
        HostConstant::NSString("kSecAttrCreationDate")),
    ("_kSecAttrModificationDate",
        HostConstant::NSString("kSecAttrModificationDate")),
    ("_kSecAttrComment",
        HostConstant::NSString("kSecAttrComment")),
    ("_kSecAttrCreator",
        HostConstant::NSString("kSecAttrCreator")),
    ("_kSecAttrType",
        HostConstant::NSString("kSecAttrType")),
    ("_kSecAttrIsInvisible",
        HostConstant::NSString("kSecAttrIsInvisible")),
    ("_kSecAttrIsNegative",
        HostConstant::NSString("kSecAttrIsNegative")),
    ("_kSecAttrSynchronizable",
        HostConstant::NSString("kSecAttrSynchronizable")),
    // kSecAttrAccessible values
    ("_kSecAttrAccessibleWhenUnlocked",
        HostConstant::NSString("kSecAttrAccessibleWhenUnlocked")),
    ("_kSecAttrAccessibleAfterFirstUnlock",
        HostConstant::NSString("kSecAttrAccessibleAfterFirstUnlock")),
    ("_kSecAttrAccessibleAlways",
        HostConstant::NSString("kSecAttrAccessibleAlways")),
    ("_kSecAttrAccessibleWhenUnlockedThisDeviceOnly",
        HostConstant::NSString("kSecAttrAccessibleWhenUnlockedThisDeviceOnly")),
    ("_kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly",
        HostConstant::NSString("kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly")),
    ("_kSecAttrAccessibleAlwaysThisDeviceOnly",
        HostConstant::NSString("kSecAttrAccessibleAlwaysThisDeviceOnly")),
    // Value keys
    ("_kSecValueData",
        HostConstant::NSString("kSecValueData")),
    ("_kSecValueRef",
        HostConstant::NSString("kSecValueRef")),
    ("_kSecValuePersistentRef",
        HostConstant::NSString("kSecValuePersistentRef")),
    // Return-type keys
    ("_kSecReturnData",
        HostConstant::NSString("kSecReturnData")),
    ("_kSecReturnAttributes",
        HostConstant::NSString("kSecReturnAttributes")),
    ("_kSecReturnRef",
        HostConstant::NSString("kSecReturnRef")),
    ("_kSecReturnPersistentRef",
        HostConstant::NSString("kSecReturnPersistentRef")),
    // Match keys
    ("_kSecMatchLimit",
        HostConstant::NSString("kSecMatchLimit")),
    ("_kSecMatchLimitOne",
        HostConstant::NSString("kSecMatchLimitOne")),
    ("_kSecMatchLimitAll",
        HostConstant::NSString("kSecMatchLimitAll")),
    ("_kSecMatchIssuers",
        HostConstant::NSString("kSecMatchIssuers")),
    ("_kSecMatchEmailAddressIfPresent",
        HostConstant::NSString("kSecMatchEmailAddressIfPresent")),
    ("_kSecMatchSubjectContains",
        HostConstant::NSString("kSecMatchSubjectContains")),
    ("_kSecMatchCaseInsensitive",
        HostConstant::NSString("kSecMatchCaseInsensitive")),
    ("_kSecMatchTrustedOnly",
        HostConstant::NSString("kSecMatchTrustedOnly")),
    ("_kSecMatchValidOnDate",
        HostConstant::NSString("kSecMatchValidOnDate")),
    ("_kSecMatchPolicy",
        HostConstant::NSString("kSecMatchPolicy")),
    ("_kSecMatchSearchList",
        HostConstant::NSString("kSecMatchSearchList")),
    // Use keys
    ("_kSecUseItemList",
        HostConstant::NSString("kSecUseItemList")),
    ("_kSecUseOperationPrompt",
        HostConstant::NSString("kSecUseOperationPrompt")),
];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(SecItemCopyMatching(_, _)),
    export_c_func!(SecItemAdd(_, _)),
    export_c_func!(SecItemDelete(_)),
    export_c_func!(SecItemUpdate(_, _)),
];

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/Security.framework/Security",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[FUNCTIONS],
};


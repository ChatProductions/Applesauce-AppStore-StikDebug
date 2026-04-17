/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 */
//! Legacy Facebook iOS SDK implementation.

use crate::dyld::HostDylib;
use crate::mem::MutVoidPtr;
use crate::objc::{objc_class, ClassExport};

pub mod fb_classes {
    use super::*;

    pub const CLASSES: &[ClassExport] = &[
        // Основной класс сессии. Возвращаем 1 (YES) на resume, чтобы игра 
        // думала, что сессия активна, или хотя бы не крашилась.
        objc_class!(FBSession, NSObject,
            "-" => "resume" => |_env, _self: MutVoidPtr| -> u8 { 
                crate::log_dbg!("FBSession resume called");
                1 
            },
            "-" => "isConnected" => |_env, _self: MutVoidPtr| -> u8 { 0 },
            "-" => "logout" => |_env, _self: MutVoidPtr| {},
        ),
        
        // Классы сетевых запросов и парсинга
        objc_class!(FBRequest, NSObject,
            "-" => "connect" => |_env, _self: MutVoidPtr| {
                crate::log_dbg!("FBRequest connect called and ignored (no network)");
            },
        ),
        objc_class!(FBXMLHandler, NSObject),
        
        // Диалоговые окна наследуются от UIView
        objc_class!(FBDialog, UIView,
            "-" => "show" => |_env, _self: MutVoidPtr| {
                crate::log_dbg!("FBDialog show called");
            },
            "-" => "dismissWithSuccess:animated:" => |_env, _self: MutVoidPtr, _success: u8, _animated: u8| {
                crate::log_dbg!("FBDialog dismissWithSuccess:animated: called");
            },
        ),
        objc_class!(FBFeedDialog, FBDialog),
        objc_class!(FBLoginDialog, FBDialog),
        objc_class!(FBPermissionDialog, FBDialog),
        
        // Кнопка наследуется от UIButton
        objc_class!(FBLoginButton, UIButton),
    ];
}

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/FacebookSDK.framework/FacebookSDK",
    aliases: &["Facebook"],
    class_exports: &[
        fb_classes::CLASSES,
    ],
    constant_exports: &[],
    function_exports: &[],
};

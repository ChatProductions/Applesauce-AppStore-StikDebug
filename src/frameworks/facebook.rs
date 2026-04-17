/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 */
//! Legacy Facebook iOS SDK implementation.

use crate::dyld::HostDylib;
use crate::mem::MutVoidPtr;
use crate::objc::{objc_classes, ClassExports};

pub mod fb_classes {
    use super::*;

    pub const CLASSES: ClassExports = &objc_classes![
        @implementation FBSession : NSObject {
            "-" => "resume" => |_env, _self: MutVoidPtr| -> u8 { 
                crate::log_dbg!("FBSession resume called");
                1 
            },
            "-" => "isConnected" => |_env, _self: MutVoidPtr| -> u8 { 0 },
            "-" => "logout" => |_env, _self: MutVoidPtr| {},
        }

        @implementation FBRequest : NSObject {
            "-" => "connect" => |_env, _self: MutVoidPtr| {
                crate::log_dbg!("FBRequest connect called");
            },
        }
        
        @implementation FBXMLHandler : NSObject {}
        
        @implementation FBDialog : UIView {
            "-" => "show" => |_env, _self: MutVoidPtr| {
                crate::log_dbg!("FBDialog show called");
            },
            "-" => "dismissWithSuccess:animated:" => |_env, _self: MutVoidPtr, _success: u8, _animated: u8| {
                crate::log_dbg!("FBDialog dismiss called");
            },
        }
        
        @implementation FBFeedDialog : FBDialog {}
        @implementation FBLoginDialog : FBDialog {}
        @implementation FBPermissionDialog : FBDialog {}
        
        @implementation FBLoginButton : UIButton {}
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

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

    // В твоей версии макрос требует (env, this, _cmd); перед списком классов
    pub const CLASSES: ClassExports = &objc_classes!(
        (env, this, _cmd); 

        FBSession : NSObject {
            "-" => "resume" => |env, this| -> u8 { 
                crate::log_dbg!("FBSession resume called");
                1 
            },
            "-" => "isConnected" => |env, this| -> u8 { 0 },
            "-" => "logout" => |env, this| {},
        },
        
        FBRequest : NSObject {
            "-" => "connect" => |env, this| {
                crate::log_dbg!("FBRequest connect called");
            },
        },
        
        FBXMLHandler : NSObject {},
        
        FBDialog : UIView {
            "-" => "show" => |env, this| {
                crate::log_dbg!("FBDialog show called");
            },
            "-" => "dismissWithSuccess:animated:" => |env, this, _success: u8, _animated: u8| {
                crate::log_dbg!("FBDialog dismiss called");
            },
        },
        
        FBFeedDialog : FBDialog {},
        FBLoginDialog : FBDialog {},
        FBPermissionDialog : FBDialog {},
        
        FBLoginButton : UIButton {}
    );
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

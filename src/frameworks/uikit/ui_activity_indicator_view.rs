/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//!
//! `UIActivityIndicatorView`.

use crate::frameworks::core_graphics::cg_rect::{CGRect, CGPoint, CGSize};
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, nil, objc_classes, release, ClassExports,
    NSZonePtr,
};

// UIActivityIndicatorViewStyle constants
type UIActivityIndicatorViewStyle = NSInteger;
pub const UIActivityIndicatorViewStyleWhiteLarge: UIActivityIndicatorViewStyle = 0;
pub const UIActivityIndicatorViewStyleWhite: UIActivityIndicatorViewStyle = 1;
pub const UIActivityIndicatorViewStyleGray: UIActivityIndicatorViewStyle = 2;
// iOS 13+ styles (for compatibility)
pub const UIActivityIndicatorViewStyleMedium: UIActivityIndicatorViewStyle = 100;
pub const UIActivityIndicatorViewStyleLarge: UIActivityIndicatorViewStyle = 101;

// Default sizes for different styles
const SIZE_WHITE_LARGE: f32 = 37.0;
const SIZE_WHITE: f32 = 20.0;
const SIZE_GRAY: f32 = 20.0;
const SIZE_MEDIUM: f32 = 20.0;
const SIZE_LARGE: f32 = 37.0;

pub struct UIActivityIndicatorViewHostObject {
    superclass: super::ui_view::UIViewHostObject,
    animating: bool,
    hides_when_stopped: bool,
    style: UIActivityIndicatorViewStyle,
    color: Option<id>, // UIColor - if nil, uses default color for style
}
impl_HostObject_with_superclass!(UIActivityIndicatorViewHostObject);

impl Default for UIActivityIndicatorViewHostObject {
    fn default() -> Self {
        UIActivityIndicatorViewHostObject {
            superclass: Default::default(),
            animating: false,
            hides_when_stopped: true,
            style: UIActivityIndicatorViewStyleWhite,
            color: None,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIActivityIndicatorView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIActivityIndicatorViewHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Initializers

- (id)init {
    msg![env; this initWithActivityIndicatorStyle:UIActivityIndicatorViewStyleWhite]
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg![env; this initWithActivityIndicatorStyle:UIActivityIndicatorViewStyleWhite];
    let _: () = msg![env; this setFrame:frame];
    this
}

- (id)initWithActivityIndicatorStyle:(UIActivityIndicatorViewStyle)style {
    // Call UIView init
    let this: id = msg![env; this init];
    
    if this == nil {
        return nil;
    }
    
    // Set the style
    let mut host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
    host.style = style;
    drop(host);
    
    // Set appropriate frame size based on style
    let size = get_size_for_style(style);
    let frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize { width: size, height: size },
    };
    let _: () = msg![env; this setFrame:frame];
    
    // Start hidden if hidesWhenStopped is true (default)
    let _: () = msg![env; this setHidden:true];
    
    log_dbg!("UIActivityIndicatorView initWithStyle:{} => {:?}", style, this);
    this
}

// NSCoding support
- (id)initWithCoder:(id)coder {
    let this: id = msg![env; this init];
    
    if this == nil {
        return nil;
    }
    
    // Decode style
    let style_str = "UIActivityIndicatorViewStyle\0".as_ptr();
    let style_key: id = msg_class![env; NSString stringWithUTF8String:style_str];
    let has_style: bool = msg![env; coder containsValueForKey:style_key];
    if has_style {
        let style: UIActivityIndicatorViewStyle = msg![env; coder decodeIntegerForKey:style_key];
        let _: () = msg![env; this setActivityIndicatorViewStyle:style];
    }
    
    // Decode hidesWhenStopped
    let hides_str = "UIActivityIndicatorViewHidesWhenStopped\0".as_ptr();
    let hides_key: id = msg_class![env; NSString stringWithUTF8String:hides_str];
    let has_hides: bool = msg![env; coder containsValueForKey:hides_key];
    if has_hides {
        let hides: bool = msg![env; coder decodeBoolForKey:hides_key];
        let _: () = msg![env; this setHidesWhenStopped:hides];
    }
    
    // Decode color
    let color_str = "UIActivityIndicatorViewColor\0".as_ptr();
    let color_key: id = msg_class![env; NSString stringWithUTF8String:color_str];
    let has_color: bool = msg![env; coder containsValueForKey:color_key];
    if has_color {
        let color: id = msg![env; coder decodeObjectForKey:color_key];
        if color != nil {
            let _: () = msg![env; this setColor:color];
        }
    }
    
    // Decode animating state
    let animating_str = "UIActivityIndicatorViewAnimating\0".as_ptr();
    let animating_key: id = msg_class![env; NSString stringWithUTF8String:animating_str];
    let has_animating: bool = msg![env; coder containsValueForKey:animating_key];
    if has_animating {
        let animating: bool = msg![env; coder decodeBoolForKey:animating_key];
        if animating {
            let _: () = msg![env; this startAnimating];
        }
    }
    
    this
}

- (())encodeWithCoder:(id)coder {
    // Encode superclass
    // let _: () = msg_super![env; this encodeWithCoder:coder];
    
    let host = env.objc.borrow::<UIActivityIndicatorViewHostObject>(this);
    
    // Encode style
    let style_str = "UIActivityIndicatorViewStyle\0".as_ptr();
    let style_key: id = msg_class![env; NSString stringWithUTF8String:style_str];
    let style = host.style;
    let _: () = msg![env; coder encodeInteger:style forKey:style_key];
    
    // Encode hidesWhenStopped
    let hides_str = "UIActivityIndicatorViewHidesWhenStopped\0".as_ptr();
    let hides_key: id = msg_class![env; NSString stringWithUTF8String:hides_str];
    let hides_when_stopped = host.hides_when_stopped;
    let _: () = msg![env; coder encodeBool:hides_when_stopped forKey:hides_key];

    // Encode color
    if let Some(color) = host.color {
        let color_str = "UIActivityIndicatorViewColor\0".as_ptr();
        let color_key: id = msg_class![env; NSString stringWithUTF8String:color_str];
        let _: () = msg![env; coder encodeObject:color forKey:color_key];
    }
    
    // Encode animating state
    let animating_str = "UIActivityIndicatorViewAnimating\0".as_ptr();
    let animating_key: id = msg_class![env; NSString stringWithUTF8String:animating_str];
    let animating = host.animating;
    let _: () = msg![env; coder encodeBool:animating forKey:animating_key];
}

// MARK: - Configuring the Activity Indicator Appearance

- (())setActivityIndicatorViewStyle:(UIActivityIndicatorViewStyle)style {
    let mut host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
    if host.style == style {
        return;
    }
    
    host.style = style;
    drop(host);

    // Update frame size to match new style
    let size = get_size_for_style(style);
    let current_frame: CGRect = msg![env; this frame];
    let new_frame = CGRect {
        origin: current_frame.origin,
        size: CGSize { width: size, height: size },
    };
    let _: () = msg![env; this setFrame:new_frame];
    
    // Trigger a redraw if needed
    let _: () = msg![env; this setNeedsDisplay];
    
    log_dbg!("UIActivityIndicatorView setStyle:{} [{:?}]", style, this);
}

- (UIActivityIndicatorViewStyle)activityIndicatorViewStyle {
    env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).style
}

- (())setColor:(id)color { // UIColor*
    let mut host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
    
    // Release old color if any
    if let Some(old_color) = host.color {
        drop(host);
        release(env, old_color);
        host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
    }
    
    // Retain new color
    if color != nil {
        let retained_color: id = msg![env; color retain];
        host.color = Some(retained_color);
    } else {
        host.color = None;
    }
    
    drop(host);
    
    // Trigger a redraw
    let _: () = msg![env; this setNeedsDisplay];
    
    log_dbg!("UIActivityIndicatorView setColor:{:?} [{:?}]", color, this);
}

- (id)color { // UIColor*
    let host = env.objc.borrow::<UIActivityIndicatorViewHostObject>(this);
    if let Some(color) = host.color {
        color
    } else {
        // Return default color for style
        get_default_color_for_style(env, host.style)
    }
}

// MARK: - Managing Animation

- (())startAnimating {
    let mut host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
    if host.animating {
        return; // Already animating
    }
    
    host.animating = true;
    drop(host);
    
    // Make visible when starting animation
    let _: () = msg![env; this setHidden:false];
    
    // Trigger display update
    let _: () = msg![env; this setNeedsDisplay];
    
    log_dbg!("UIActivityIndicatorView startAnimating [{:?}]", this);
}

- (())stopAnimating {
    let mut host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
    if !host.animating {
        return; // Already stopped
    }
    
    host.animating = false;
    let hides = host.hides_when_stopped;
    drop(host);

    log_dbg!("UIActivityIndicatorView stopAnimating [{:?}]", this);

    // Hide if hidesWhenStopped is true
    if hides {
        let _: () = msg![env; this setHidden:true];
    }
    
    // Trigger display update
    let _: () = msg![env; this setNeedsDisplay];
}

- (bool)isAnimating {
    env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).animating
}

// MARK: - Configuring Visibility Behavior

- (())setHidesWhenStopped:(bool)hides {
    let mut host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
    if host.hides_when_stopped == hides {
        return;
    }
    
    host.hides_when_stopped = hides;
    let animating = host.animating;
    drop(host);
    
    // Update visibility based on new setting
    if !animating && hides {
        let _: () = msg![env; this setHidden:true];
    } else if !animating && !hides {
        let _: () = msg![env; this setHidden:false];
    }
    
    log_dbg!("UIActivityIndicatorView setHidesWhenStopped:{} [{:?}]", hides, this);
}

- (bool)hidesWhenStopped {
    env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).hides_when_stopped
}

// MARK: - UIView overrides

- (())layoutSubviews {
    // Call super
    // let _: () = msg_super![env; this layoutSubviews];
    
    // Center the indicator in its bounds
    let bounds: CGRect = msg![env; this bounds];
    let size = get_size_for_style(env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).style);
    
    let center_x = bounds.size.width / 2.0;
    let center_y = bounds.size.height / 2.0;
    let center = CGPoint { x: center_x, y: center_y };
    let _: () = msg![env; this setCenter:center];
}

- (CGSize)intrinsicContentSize {
    let style = env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).style;
    let size = get_size_for_style(style);
    CGSize { width: size, height: size }
}

// MARK: - Cleanup

- (())dealloc {
    // Release color if any
    let host = env.objc.borrow::<UIActivityIndicatorViewHostObject>(this);
    if let Some(color) = host.color {
        drop(host);
        release(env, color);
    }
}

@end

};

// MARK: - Helper functions

fn get_size_for_style(style: UIActivityIndicatorViewStyle) -> f32 {
    match style {
        UIActivityIndicatorViewStyleWhiteLarge => SIZE_WHITE_LARGE,
        UIActivityIndicatorViewStyleWhite => SIZE_WHITE,
        UIActivityIndicatorViewStyleGray => SIZE_GRAY,
        UIActivityIndicatorViewStyleMedium => SIZE_MEDIUM,
        UIActivityIndicatorViewStyleLarge => SIZE_LARGE,
        _ => {
            log!("Warning: Unknown UIActivityIndicatorViewStyle {}, using default", style);
            SIZE_WHITE
        }
    }
}

fn get_default_color_for_style(env: &mut crate::Environment, style: UIActivityIndicatorViewStyle) -> id {
    match style {
        UIActivityIndicatorViewStyleWhiteLarge |
        UIActivityIndicatorViewStyleWhite => {
            msg_class![env; UIColor whiteColor]
        }
        UIActivityIndicatorViewStyleGray => {
            msg_class![env; UIColor grayColor]
        }
        UIActivityIndicatorViewStyleMedium |
        UIActivityIndicatorViewStyleLarge => {
            // iOS 13+ uses system gray color
            msg_class![env; UIColor systemGrayColor]
        }
        _ => {
            msg_class![env; UIColor whiteColor]
        }
    }
}

// MARK: - Public helper functions

/// Helper to create an activity indicator with a specific style
pub fn create_activity_indicator(
    env: &mut crate::Environment,
    style: UIActivityIndicatorViewStyle,
) -> id {
    let indicator: id = msg_class![env; UIActivityIndicatorView alloc];
    msg![env; indicator initWithActivityIndicatorStyle:style]
}

/// Helper to check if an indicator is currently animating
pub fn is_animating(env: &mut crate::Environment, indicator: id) -> bool {
    if indicator == nil {
        return false;
    }
    msg![env; indicator isAnimating]
}

/// Helper to start animation on an indicator
pub fn start_animating(env: &mut crate::Environment, indicator: id) {
    if indicator != nil {
        let _: () = msg![env; indicator startAnimating];
    }
}

/// Helper to stop animation on an indicator
pub fn stop_animating(env: &mut crate::Environment, indicator: id) {
    if indicator != nil {
        let _: () = msg![env; indicator stopAnimating];
    }
}

/// Helper to set custom color
pub fn set_indicator_color(env: &mut crate::Environment, indicator: id, color: id) {
    if indicator != nil && color != nil {
        let _: () = msg![env; indicator setColor:color];
    }
}

/// Helper to create and configure an indicator in one call
pub fn create_configured_indicator(
    env: &mut crate::Environment,
    style: UIActivityIndicatorViewStyle,
    color: Option<id>,
    hides_when_stopped: bool,
    start_animating: bool,
) -> id {
    let indicator = create_activity_indicator(env, style);
    if indicator == nil {
        return nil;
    }
    
    // Set color if provided
    if let Some(c) = color {
        let _: () = msg![env; indicator setColor:c];
    }
    
    // Set hidesWhenStopped
    let _: () = msg![env; indicator setHidesWhenStopped:hides_when_stopped];
    
    // Start animating if requested
    if start_animating {
        let _: () = msg![env; indicator startAnimating];
    }
    
    indicator
}


/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIActivityIndicatorView`.

use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, nil, objc_classes, release, retain,
    ClassExports, NSZonePtr,
};

pub type UIActivityIndicatorViewStyle = NSInteger;

// UIActivityIndicatorViewStyle constants
pub const UIActivityIndicatorViewStyleWhiteLarge: UIActivityIndicatorViewStyle = 0;
pub const UIActivityIndicatorViewStyleWhite:      UIActivityIndicatorViewStyle = 1;
pub const UIActivityIndicatorViewStyleGray:       UIActivityIndicatorViewStyle = 2;
// iOS 13+ large styles (map to equivalent older ones for compatibility)
pub const UIActivityIndicatorViewStyleLarge:      UIActivityIndicatorViewStyle = 100;
pub const UIActivityIndicatorViewStyleMedium:     UIActivityIndicatorViewStyle = 101;

pub struct UIActivityIndicatorViewHostObject {
    superclass: super::ui_view::UIViewHostObject,
    animating: bool,
    hides_when_stopped: bool,
    style: UIActivityIndicatorViewStyle,
    /// UIColor* — tint/indicator color override.
    color: id,
}
impl_HostObject_with_superclass!(UIActivityIndicatorViewHostObject);
impl Default for UIActivityIndicatorViewHostObject {
    fn default() -> Self {
        UIActivityIndicatorViewHostObject {
            superclass: Default::default(),
            animating: false,
            hides_when_stopped: true,
            style: UIActivityIndicatorViewStyleWhite,
            color: nil,
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

// =========================================================================
// MARK: - Initializers
// =========================================================================

- (id)initWithActivityIndicatorStyle:(UIActivityIndicatorViewStyle)style {
    // Choose a default frame size based on style, matching UIKit defaults.
    let size: f32 = match style {
        UIActivityIndicatorViewStyleWhiteLarge | UIActivityIndicatorViewStyleLarge => 37.0,
        _ => 20.0,
    };
    let frame = CGRect {
        origin: crate::frameworks::core_graphics::CGPoint { x: 0.0, y: 0.0 },
        size: crate::frameworks::core_graphics::CGSize { width: size, height: size },
    };
    let this: id = msg![env; this initWithFrame:frame];
    env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this).style = style;
    this
}

- (id)initWithFrame:(CGRect)frame {
    msg![env; this initWithFrame:frame]
}

- (id)initWithCoder:(id)coder {
    let this: id = msg![env; this initWithCoder:coder];
    this
}

// =========================================================================
// MARK: - Dealloc
// =========================================================================

- (())dealloc {
    let color = env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).color;
    release(env, color);
    // Superclass dealloc is handled by UIView.
}

// =========================================================================
// MARK: - Style
// =========================================================================

- (UIActivityIndicatorViewStyle)activityIndicatorViewStyle {
    env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).style
}

- (())setActivityIndicatorViewStyle:(UIActivityIndicatorViewStyle)style {
    env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this).style = style;
    // Resize to match the new style's default dimensions.
    let size: f32 = match style {
        UIActivityIndicatorViewStyleWhiteLarge | UIActivityIndicatorViewStyleLarge => 37.0,
        _ => 20.0,
    };
    let bounds: CGRect = msg![env; this bounds];
    let new_bounds = CGRect {
        origin: bounds.origin,
        size: crate::frameworks::core_graphics::CGSize { width: size, height: size },
    };
    let _: () = msg![env; this setBounds:new_bounds];
}

// =========================================================================
// MARK: - Color
// =========================================================================

- (id)color { // UIColor*
    env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).color
}

- (())setColor:(id)color { // UIColor*
    let old = env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).color;
    release(env, old);
    retain(env, color);
    env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this).color = color;
}

// =========================================================================
// MARK: - Animation control
// =========================================================================

- (())startAnimating {
    {
        let host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
        if host.animating {
            return; // Already animating — no-op.
        }
        host.animating = true;
    }
    // Make visible when starting.
    let _: () = msg![env; this setHidden:false];
    log_dbg!("UIActivityIndicatorView startAnimating {:?}", this);
}

- (())stopAnimating {
    let hides = {
        let host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
        if !host.animating {
            return; // Already stopped — no-op.
        }
        host.animating = false;
        host.hides_when_stopped
    };
    if hides {
        let _: () = msg![env; this setHidden:true];
    }
    log_dbg!("UIActivityIndicatorView stopAnimating {:?}", this);
}

- (bool)isAnimating {
    env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).animating
}

// =========================================================================
// MARK: - hidesWhenStopped
// =========================================================================

- (bool)hidesWhenStopped {
    env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).hides_when_stopped
}

- (())setHidesWhenStopped:(bool)hides {
    env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this).hides_when_stopped = hides;
    // Apply immediately if not currently animating.
    let animating = env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).animating;
    if !animating {
        let _: () = msg![env; this setHidden:hides];
    }
}

// =========================================================================
// MARK: - Size / layout
// =========================================================================

- (id)sizeThatFits:(id)_size { // CGSize param ignored — returns intrinsic size
    let style = env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).style;
    let dim: f32 = match style {
        UIActivityIndicatorViewStyleWhiteLarge | UIActivityIndicatorViewStyleLarge => 37.0,
        _ => 20.0,
    };
    let size = crate::frameworks::core_graphics::CGSize { width: dim, height: dim };
    msg_class![env; NSValue valueWithCGSize:size]
}

// =========================================================================
// MARK: - Description
// =========================================================================

- (id)description {
    let (animating, style, hides) = {
        let h = env.objc.borrow::<UIActivityIndicatorViewHostObject>(this);
        (h.animating, h.style, h.hides_when_stopped)
    };
    let s = format!(
        "<UIActivityIndicatorView: {:?}; style={}; animating={}; hidesWhenStopped={}>",
        this, style, animating, hides
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};

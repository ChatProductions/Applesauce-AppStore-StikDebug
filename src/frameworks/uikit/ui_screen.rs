/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIScreen`.

use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::objc::{id, msg, msg_class, nil, objc_classes, ClassExports, TrivialHostObject};

#[derive(Default)]
pub struct State {
    main_screen: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIScreen: NSObject

// MARK: - Singleton

+ (id)mainScreen {
    if let Some(screen) = env.framework_state.uikit.ui_screen.main_screen {
        screen
    } else {
        let new = env.objc.alloc_static_object(
            this,
            Box::new(TrivialHostObject),
            &mut env.mem,
        );
        env.framework_state.uikit.ui_screen.main_screen = Some(new);
        new
    }
}

+ (id)screens {
    // Only one screen is ever available.
    let main: id = msg![env; this mainScreen];
    msg_class![env; NSArray arrayWithObject:main]
}

// MARK: - Retain / release (singleton — no-ops)

- (id)retain      { this }
- (())release     {}
- (id)autorelease { this }

// MARK: - Geometry

- (CGRect)bounds {
    let (width, height) = env.window().device_family().portrait_size();
    CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width:  width  as CGFloat,
            height: height as CGFloat,
        },
    }
}

- (CGRect)nativeBounds {
    // Same as bounds at scale 1 — we don't model the physical pixel grid.
    msg![env; this bounds]
}

- (CGRect)applicationFrame {
    let mut bounds: CGRect = msg![env; this bounds];
    const STATUS_BAR_HEIGHT: CGFloat = 20.0;
    if !env.framework_state.uikit.ui_application.status_bar_hidden {
        bounds.origin.y    += STATUS_BAR_HEIGHT;
        bounds.size.height -= STATUS_BAR_HEIGHT;
    }
    bounds
}

// MARK: - Scale

- (CGFloat)scale {
    1.0
}

- (CGFloat)nativeScale {
    // Physical pixels == points for our purposes.
    1.0
}

// MARK: - Brightness

- (CGFloat)brightness {
    1.0
}

- (())setBrightness:(CGFloat)_brightness {
    log!("TODO: [UIScreen setBrightness:] (not implemented)");
}

- (bool)wantsSoftwareDimming {
    false
}

- (())setWantsSoftwareDimming:(bool)_value {
    log!("TODO: [UIScreen setWantsSoftwareDimming:] (not implemented)");
}

// MARK: - Display mode / overscan

- (id)currentMode {
    // UIScreenMode — return nil; apps that need this are uncommon.
    log!("TODO: [UIScreen currentMode] returning nil");
    nil
}

- (id)preferredMode {
    nil
}

- (id)availableModes {
    msg_class![env; NSArray new]
}

- (CGFloat)overscanCompensationInsets {
    // UIEdgeInsetsZero as four floats would need a custom return type.
    // Return 0.0 as a stand-in; the real return type is UIEdgeInsets.
    0.0
}

// MARK: - Mirroring (iOS 4.3+)

- (id)mirroredScreen {
    nil
}

- (bool)isCaptured {
    false
}

// MARK: - Coordinate conversion helpers

- (CGRect)convertRect:(CGRect)rect toScreen:(id)_other_screen {
    // Single-screen device — coordinates are always the same.
    rect
}

- (CGRect)convertRect:(CGRect)rect fromScreen:(id)_other_screen {
    rect
}

- (CGPoint)convertPoint:(CGPoint)point toScreen:(id)_other_screen {
    point
}

- (CGPoint)convertPoint:(CGPoint)point fromScreen:(id)_other_screen {
    point
}

// MARK: - Description

- (id)description {
    let bounds: CGRect = msg![env; this bounds];
    let scale:  CGFloat = msg![env; this scale];
    let desc = format!(
        "<UIScreen: bounds=({}, {}, {}, {}) scale={}>",
        bounds.origin.x, bounds.origin.y,
        bounds.size.width, bounds.size.height,
        scale
    );
    let ns = crate::frameworks::foundation::ns_string::from_rust_string(env, desc);
    crate::objc::autorelease(env, ns)
}

@end

};

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIAlertView` — renders an iOS-style alert dialog on screen.
//!
//! Architecture: when `show` is called we create a full-screen overlay UIView
//! that draws the dimmed background + alert box, and blocks all touch input
//! until the user taps a button. On tap we fire the delegate callbacks and
//! remove the overlay.

use crate::frameworks::core_graphics::cg_context::{
    CGContextFillRect, CGContextRef, CGContextSetRGBFillColor,
    
};
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::{ns_string, NSInteger, NSUInteger};
use crate::frameworks::uikit::ui_font::{
    UITextAlignmentCenter, UILineBreakModeMiddleTruncation,
};
use crate::frameworks::uikit::ui_graphics::UIGraphicsGetCurrentContext;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release, retain, ClassExports, NSZonePtr,
};

// UIAlertViewStyle constants
pub type UIAlertViewStyle = NSInteger;
pub const UIAlertViewStyleDefault:               UIAlertViewStyle = 0;
pub const UIAlertViewStyleSecureTextInput:       UIAlertViewStyle = 1;
pub const UIAlertViewStylePlainTextInput:        UIAlertViewStyle = 2;
pub const UIAlertViewStyleLoginAndPasswordInput: UIAlertViewStyle = 3;

// Layout constants (points, matching iOS 3 look)
const ALERT_WIDTH:        CGFloat = 276.0;
const TITLE_FONT_SIZE:    CGFloat = 17.0;
const MESSAGE_FONT_SIZE:  CGFloat = 14.0;
const BUTTON_HEIGHT:      CGFloat = 44.0;
const PADDING:            CGFloat = 16.0;
const CORNER_RADIUS:      CGFloat = 10.0;

pub struct UIAlertViewHostObject {
    superclass:          super::UIViewHostObject,
    title:               id, // NSString*
    message:             id, // NSString*
    delegate:            id,
    button_titles:       id, // NSMutableArray<NSString*>*
    cancel_button_index: NSInteger,
    visible:             bool,
    alert_view_style:    UIAlertViewStyle,
    tag:                 NSInteger,
    /// The overlay UIView we add to the key window.
    overlay_view:        id,
}
impl_HostObject_with_superclass!(UIAlertViewHostObject);

// =========================================================================
// Internal helper: draw a rounded rectangle path fill + stroke
// =========================================================================
fn draw_rounded_rect(
    env:    &mut crate::Environment,
    ctx:    CGContextRef,
    rect:   CGRect,
    _r:     CGFloat,
    fr: (CGFloat, CGFloat, CGFloat, CGFloat),
    _sr: (CGFloat, CGFloat, CGFloat, CGFloat),
    _lw:     CGFloat,
) {
    // touchHLE's CGContext doesn't expose AddRoundedRect, so we approximate
    // with a plain rect — good enough for a functional alert.
    CGContextSetRGBFillColor(env, ctx, fr.0, fr.1, fr.2, fr.3);
    CGContextFillRect(env, ctx, rect);

}

// =========================================================================
// _UIAlertOverlayView — the actual drawing view
// =========================================================================
//
// This is an internal class. It holds a weak back-pointer to its UIAlertView
// owner so it can read title/message/buttons and call dismissal.

struct UIAlertOverlayHostObject {
    superclass: super::UIViewHostObject,
    /// Weak reference back to the owning UIAlertView.
    alert_view: id,
}
impl_HostObject_with_superclass!(UIAlertOverlayHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - _UIAlertOverlayView
// =========================================================================

@implementation _UIAlertOverlayView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(UIAlertOverlayHostObject {
        superclass: Default::default(),
        alert_view: nil,
    });
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    let this = msg_super![env; this initWithFrame:frame];
    this
}

// The view draws the entire alert: dimmed bg + box + text + buttons.
- (())drawRect:(CGRect)_rect {
    let ctx: CGContextRef = UIGraphicsGetCurrentContext(env);
    let bounds: CGRect = msg![env; this bounds];

    // --- Dimmed background ---
    CGContextSetRGBFillColor(env, ctx, 0.0, 0.0, 0.0, 0.5);
    CGContextFillRect(env, ctx, bounds);

    // --- Compute alert box rect ---
    let alert_x = (bounds.size.width  - ALERT_WIDTH) * 0.5;

    // Measure content height dynamically
    let av: id = env.objc.borrow::<UIAlertOverlayHostObject>(this).alert_view;
    if av == nil { return; }

    let (title, message, buttons, _style) = {
        let h = env.objc.borrow::<UIAlertViewHostObject>(av);
        (h.title, h.message, h.button_titles, h.alert_view_style)
    };
    let btn_count: NSUInteger = msg![env; buttons count];

    let title_font:   id = msg_class![env; UIFont boldSystemFontOfSize:TITLE_FONT_SIZE];
    let message_font: id = msg_class![env; UIFont systemFontOfSize:MESSAGE_FONT_SIZE];

    let max_text_size = CGSize { width: ALERT_WIDTH - PADDING * 2.0, height: 9999.0 };

    let title_h: CGFloat = if title != nil {
        let s: CGSize = msg![env; title sizeWithFont:title_font
                                  constrainedToSize:max_text_size
                                      lineBreakMode:UILineBreakModeMiddleTruncation];
        s.height + PADDING
    } else { 0.0 };

    let message_h: CGFloat = if message != nil {
        let s: CGSize = msg![env; message sizeWithFont:message_font
                                     constrainedToSize:max_text_size
                                         lineBreakMode:UILineBreakModeMiddleTruncation];
        s.height + PADDING
    } else { 0.0 };

    let buttons_h = BUTTON_HEIGHT * btn_count as CGFloat;
    let alert_h   = PADDING + title_h + message_h + buttons_h + PADDING;
    let alert_y   = (bounds.size.height - alert_h) * 0.5;

    let alert_rect = CGRect {
        origin: CGPoint { x: alert_x, y: alert_y },
        size:   CGSize  { width: ALERT_WIDTH, height: alert_h },
    };

    // --- Box background: dark blue-grey (iOS 2/3 style) ---
    draw_rounded_rect(
        env, ctx, alert_rect, CORNER_RADIUS,
        (0.12, 0.18, 0.28, 0.97),  // fill
        (0.55, 0.65, 0.80, 1.0),   // stroke
        1.5,
    );

    // --- Title ---
    let mut y = alert_y + PADDING;
    if title != nil {
        let text_rect = CGRect {
            origin: CGPoint { x: alert_x + PADDING, y },
            size:   CGSize  { width: ALERT_WIDTH - PADDING * 2.0, height: title_h },
        };
        CGContextSetRGBFillColor(env, ctx, 1.0, 1.0, 1.0, 1.0);
        let _: () = msg![env; title drawInRect:text_rect
                                      withFont:title_font
                                 lineBreakMode:UILineBreakModeMiddleTruncation
                                     alignment:UITextAlignmentCenter];
        y += title_h;
    }

    // --- Message ---
    if message != nil {
        let text_rect = CGRect {
            origin: CGPoint { x: alert_x + PADDING, y },
            size:   CGSize  { width: ALERT_WIDTH - PADDING * 2.0, height: message_h },
        };
        CGContextSetRGBFillColor(env, ctx, 0.85, 0.85, 0.85, 1.0);
        let _: () = msg![env; message drawInRect:text_rect
                                        withFont:message_font
                                   lineBreakMode:UILineBreakModeMiddleTruncation
                                       alignment:UITextAlignmentCenter];
        y += message_h;
    }

    // --- Divider line above buttons ---
    CGContextSetRGBFillColor(env, ctx, 0.4, 0.5, 0.65, 1.0);
    let div = CGRect { origin: CGPoint { x: alert_x, y },
                       size:   CGSize  { width: ALERT_WIDTH, height: 1.0 } };
    CGContextFillRect(env, ctx, div);

    // --- Buttons ---
    let btn_font: id = msg_class![env; UIFont boldSystemFontOfSize:MESSAGE_FONT_SIZE];
    for i in 0..(btn_count as NSInteger) {
        let btn_title: id = msg![env; buttons objectAtIndex:(i as NSUInteger)];
        let btn_rect = CGRect {
            origin: CGPoint { x: alert_x, y: y + i as CGFloat * BUTTON_HEIGHT },
            size:   CGSize  { width: ALERT_WIDTH, height: BUTTON_HEIGHT },
        };

        // Subtle highlight for cancel button
        let cancel = env.objc.borrow::<UIAlertViewHostObject>(av).cancel_button_index;
        if i == cancel {
            CGContextSetRGBFillColor(env, ctx, 0.08, 0.13, 0.22, 0.6);
            CGContextFillRect(env, ctx, btn_rect);
        }

        // Divider between buttons
        if i > 0 {
            CGContextSetRGBFillColor(env, ctx, 0.4, 0.5, 0.65, 1.0);
            let bdiv = CGRect { origin: CGPoint { x: alert_x, y: y + i as CGFloat * BUTTON_HEIGHT },
                                size:   CGSize  { width: ALERT_WIDTH, height: 1.0 } };
            CGContextFillRect(env, ctx, bdiv);
        }

        // Button text
        let text_rect = CGRect {
            origin: CGPoint { x: alert_x + PADDING, y: y + i as CGFloat * BUTTON_HEIGHT + (BUTTON_HEIGHT - MESSAGE_FONT_SIZE) * 0.5 },
            size:   CGSize  { width: ALERT_WIDTH - PADDING * 2.0, height: MESSAGE_FONT_SIZE + 4.0 },
        };
        CGContextSetRGBFillColor(env, ctx, 1.0, 1.0, 1.0, 1.0);
        if btn_title != nil {
            let _: () = msg![env; btn_title drawInRect:text_rect
                                             withFont:btn_font
                                        lineBreakMode:UILineBreakModeMiddleTruncation
                                            alignment:UITextAlignmentCenter];
        }
    }
}

// Hit testing: detect button taps
- (id)hitTest:(CGPoint)point withEvent:(id)event {
    let av: id = env.objc.borrow::<UIAlertOverlayHostObject>(this).alert_view;
    if av == nil {
        return msg_super![env; this hitTest:point withEvent:event];
    }

    let bounds: CGRect = msg![env; this bounds];
    let btn_count: NSUInteger = {
        let buttons = env.objc.borrow::<UIAlertViewHostObject>(av).button_titles;
        msg![env; buttons count]
    };

    // Recompute alert box position (same as drawRect)
    let alert_x = (bounds.size.width - ALERT_WIDTH) * 0.5;

    let (title, message, _) = {
        let h = env.objc.borrow::<UIAlertViewHostObject>(av);
        (h.title, h.message, h.alert_view_style)
    };

    let title_font:   id = msg_class![env; UIFont boldSystemFontOfSize:TITLE_FONT_SIZE];
    let message_font: id = msg_class![env; UIFont systemFontOfSize:MESSAGE_FONT_SIZE];
    let max_text_size = CGSize { width: ALERT_WIDTH - PADDING * 2.0, height: 9999.0 };

    let title_h: CGFloat = if title != nil {
        let s: CGSize = msg![env; title sizeWithFont:title_font
                                  constrainedToSize:max_text_size
                                      lineBreakMode:UILineBreakModeMiddleTruncation];
        s.height + PADDING
    } else { 0.0 };

    let message_h: CGFloat = if message != nil {
        let s: CGSize = msg![env; message sizeWithFont:message_font
                                     constrainedToSize:max_text_size
                                         lineBreakMode:UILineBreakModeMiddleTruncation];
        s.height + PADDING
    } else { 0.0 };

    let buttons_h = BUTTON_HEIGHT * btn_count as CGFloat;
    let alert_h   = PADDING + title_h + message_h + buttons_h + PADDING;
    let alert_y   = (bounds.size.height - alert_h) * 0.5;
    let buttons_y = alert_y + PADDING + title_h + message_h + 1.0; // +1 for divider

    // Check if tap is within a button
    if point.x >= alert_x && point.x <= alert_x + ALERT_WIDTH {
        for i in 0..(btn_count as NSInteger) {
            let btn_y = buttons_y + i as CGFloat * BUTTON_HEIGHT;
            if point.y >= btn_y && point.y <= btn_y + BUTTON_HEIGHT {
                log_dbg!("UIAlertView: tapped button {}", i);
                let _: () = msg![env; av dismissWithClickedButtonIndex:i animated:true];
                return this; // consume the touch
            }
        }
    }
    this // consume all touches (modal)
}

@end

// =========================================================================
// MARK: - UIAlertView
// =========================================================================

@implementation UIAlertView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIAlertViewHostObject {
        superclass:          Default::default(),
        title:               nil,
        message:             nil,
        delegate:            nil,
        button_titles:       nil,
        cancel_button_index: -1,
        visible:             false,
        alert_view_style:    UIAlertViewStyleDefault,
        tag:                 0,
        overlay_view:        nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    msg_super![env; this initWithFrame:frame]
}

// =========================================================================
// MARK: - Designated initializer
// =========================================================================

- (id)initWithTitle:(id)title
            message:(id)message
           delegate:(id)delegate
  cancelButtonTitle:(id)cancel_title
  otherButtonTitles:(id)other_titles {
    // Initialize the UIView superclass with a zero frame
    let zero = crate::frameworks::core_graphics::CGRect {
        origin: crate::frameworks::core_graphics::CGPoint { x: 0.0, y: 0.0 },
        size:   crate::frameworks::core_graphics::CGSize  { width: 0.0, height: 0.0 },
    };
    let this = msg_super![env; this initWithFrame:zero];
    let buttons: id = msg_class![env; NSMutableArray new];
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).button_titles = buttons;

    retain(env, title);
    retain(env, message);
    retain(env, delegate);
    {
        let host = env.objc.borrow_mut::<UIAlertViewHostObject>(this);
        host.title    = title;
        host.message  = message;
        host.delegate = delegate;
    }

    if cancel_title != nil {
        let idx: NSUInteger = msg![env; buttons count];
        let _: () = msg![env; buttons addObject:cancel_title];
        env.objc.borrow_mut::<UIAlertViewHostObject>(this).cancel_button_index =
            idx as NSInteger;
    }
    if other_titles != nil {
        let _: () = msg![env; buttons addObject:other_titles];
    }

    let title_str = if title != nil {
        ns_string::to_rust_string(env, title).into_owned()
    } else { "(nil)".into() };
    let msg_str = if message != nil {
        ns_string::to_rust_string(env, message).into_owned()
    } else { "(nil)".into() };
    log!("UIAlertView init title={:?} message={:?}", title_str, msg_str);
    this
}

// =========================================================================
// MARK: - Dealloc
// =========================================================================

- (())dealloc {
    let host = env.objc.borrow::<UIAlertViewHostObject>(this);
    let (title, message, delegate, buttons, overlay) =
        (host.title, host.message, host.delegate, host.button_titles, host.overlay_view);
    
    // БЕЗОПАСНАЯ ОЧИСТКА ПОДЛОЖКИ (OVERLAY)
    if overlay != nil {
        // Убираем висячую ссылку, чтобы overlay не пытался обратиться к удаленному UIAlertView
        env.objc.borrow_mut::<UIAlertOverlayHostObject>(overlay).alert_view = nil;
        // Убираем с экрана
        let _: () = msg![env; overlay removeFromSuperview];
        release(env, overlay);
    }

    release(env, title);
    release(env, message);
    release(env, delegate);
    release(env, buttons);
    
    env.objc.dealloc_object(this, &mut env.mem)
}

// =========================================================================
// MARK: - Accessors
// =========================================================================

- (id)title { env.objc.borrow::<UIAlertViewHostObject>(this).title }
- (())setTitle:(id)title {
    let old = env.objc.borrow::<UIAlertViewHostObject>(this).title;
    release(env, old); retain(env, title);
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).title = title;
}

- (id)message { env.objc.borrow::<UIAlertViewHostObject>(this).message }
- (())setMessage:(id)message {
    let old = env.objc.borrow::<UIAlertViewHostObject>(this).message;
    release(env, old); retain(env, message);
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).message = message;
}

- (id)delegate { env.objc.borrow::<UIAlertViewHostObject>(this).delegate }
- (())setDelegate:(id)delegate {
    let old = env.objc.borrow::<UIAlertViewHostObject>(this).delegate;
    release(env, old); retain(env, delegate);
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).delegate = delegate;
}

- (NSInteger)tag { env.objc.borrow::<UIAlertViewHostObject>(this).tag }
- (())setTag:(NSInteger)tag {
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).tag = tag;
}

- (UIAlertViewStyle)alertViewStyle {
    env.objc.borrow::<UIAlertViewHostObject>(this).alert_view_style
}
- (())setAlertViewStyle:(UIAlertViewStyle)style {
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).alert_view_style = style;
}

- (bool)isVisible { env.objc.borrow::<UIAlertViewHostObject>(this).visible }

// =========================================================================
// MARK: - Buttons
// =========================================================================

- (NSInteger)addButtonWithTitle:(id)title {
    let buttons = env.objc.borrow::<UIAlertViewHostObject>(this).button_titles;
    let idx: NSUInteger = msg![env; buttons count];
    let _: () = msg![env; buttons addObject:title];
    idx as NSInteger
}

- (NSUInteger)numberOfButtons {
    let buttons = env.objc.borrow::<UIAlertViewHostObject>(this).button_titles;
    msg![env; buttons count]
}

- (id)buttonTitleAtIndex:(NSInteger)index {
    let buttons = env.objc.borrow::<UIAlertViewHostObject>(this).button_titles;
    let count: NSUInteger = msg![env; buttons count];
    if index < 0 || index as NSUInteger >= count { return nil; }
    msg![env; buttons objectAtIndex:(index as NSUInteger)]
}

- (NSInteger)cancelButtonIndex {
    env.objc.borrow::<UIAlertViewHostObject>(this).cancel_button_index
}
- (())setCancelButtonIndex:(NSInteger)index {
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).cancel_button_index = index;
}

- (NSInteger)firstOtherButtonIndex {
    let host    = env.objc.borrow::<UIAlertViewHostObject>(this);
    let buttons = host.button_titles;
    let cancel  = host.cancel_button_index;
    let count: NSUInteger = msg![env; buttons count];
    for i in 0..count {
        if i as NSInteger != cancel { return i as NSInteger; }
    }
    -1
}

- (id)textFieldAtIndex:(NSInteger)_index {
    log_dbg!("UIAlertView textFieldAtIndex: — returning nil (no text field UI)");
    nil
}

// =========================================================================
// MARK: - Show / dismiss
// =========================================================================

- (())show {
    log!("UIAlertView show");
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).visible = true;

    // Find the key window
    let key_window: id = msg_class![env; UIApplication sharedApplication];
    let key_window: id = msg![env; key_window keyWindow];
    if key_window == nil {
        log!("UIAlertView show: no key window, falling back to instant dismiss");
        let cancel = env.objc.borrow::<UIAlertViewHostObject>(this).cancel_button_index;
        let idx = if cancel >= 0 { cancel } else { 0 };
        let _: () = msg![env; this dismissWithClickedButtonIndex:idx animated:false];
        return;
    }

    // Create full-screen overlay
    let win_bounds: CGRect = msg![env; key_window bounds];
    let overlay: id = msg_class![env; _UIAlertOverlayView alloc];
    let overlay: id = msg![env; overlay initWithFrame:win_bounds];

    // Point overlay back to us (weak ref — no retain)
    env.objc.borrow_mut::<UIAlertOverlayHostObject>(overlay).alert_view = this;

    // Убрано лишнее retain(env, overlay); чтобы не было утечки памяти
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).overlay_view = overlay;

    // Add to window
    let _: () = msg![env; key_window addSubview:overlay];
    let _: () = msg![env; overlay setNeedsDisplay];
}

- (())dismissWithClickedButtonIndex:(NSInteger)button_index
                           animated:(bool)_animated {
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).visible = false;

    // Remove overlay from window
    let overlay = env.objc.borrow::<UIAlertViewHostObject>(this).overlay_view;
    if overlay != nil {
        // Очищаем слабую ссылку
        env.objc.borrow_mut::<UIAlertOverlayHostObject>(overlay).alert_view = nil;
        let _: () = msg![env; overlay removeFromSuperview];
        release(env, overlay);
        env.objc.borrow_mut::<UIAlertViewHostObject>(this).overlay_view = nil;
    }

    let delegate = env.objc.borrow::<UIAlertViewHostObject>(this).delegate;
    if delegate == nil { return; }

    if let Some(sel) = env.objc.lookup_selector("alertView:clickedButtonAtIndex:") {
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate alertView:this clickedButtonAtIndex:button_index];
        }
    }
    if let Some(sel) = env.objc.lookup_selector("alertView:willDismissWithButtonIndex:") {
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate alertView:this willDismissWithButtonIndex:button_index];
        }
    }
    if let Some(sel) = env.objc.lookup_selector("alertView:didDismissWithButtonIndex:") {
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate alertView:this didDismissWithButtonIndex:button_index];
        }
    }
}

// =========================================================================
// MARK: - Description
// =========================================================================

- (id)description {
    let (title, visible) = {
        let h = env.objc.borrow::<UIAlertViewHostObject>(this);
        (h.title, h.visible)
    };
    let title_str = if title != nil {
        ns_string::to_rust_string(env, title).into_owned()
    } else { "(nil)".into() };
    let s = format!(
        "<UIAlertView: {:?}; title={:?}; visible={}>",
        this, title_str, visible
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};

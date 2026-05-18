/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UILabel`.

use crate::frameworks::core_graphics::cg_context::CGContextSetRGBFillColor;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::frameworks::uikit::ui_color;
use crate::frameworks::uikit::ui_font::{
    UILineBreakMode, UILineBreakModeTailTruncation, UITextAlignment, UITextAlignmentCenter,
    UITextAlignmentLeft, UITextAlignmentRight,
};
use crate::frameworks::uikit::ui_graphics::UIGraphicsGetCurrentContext;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    retain, todo_objc_setter, ClassExports, NSZonePtr,
};

type UIBaselineAdjustment = NSInteger;

pub struct UILabelHostObject {
    superclass: super::UIViewHostObject,
    text: id,
    font: id,
    text_color: id,
    highlighted_text_color: id,
    text_alignment: UITextAlignment,
    line_break_mode: UILineBreakMode,
    number_of_lines: NSInteger,
    enabled: bool,
}
impl_HostObject_with_superclass!(UILabelHostObject);

impl Default for UILabelHostObject {
    fn default() -> Self {
        UILabelHostObject {
            superclass: Default::default(),
            text: nil,
            font: nil,
            text_color: nil,
            highlighted_text_color: nil,
            text_alignment: UITextAlignmentLeft,
            line_break_mode: UILineBreakModeTailTruncation,
            number_of_lines: 1,
            enabled: true,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UILabel: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UILabelHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];

    let key_text = get_static_str(env, "UIText");
    if msg![env; coder containsValueForKey:key_text] {
        let text: id = msg![env; coder decodeObjectForKey:key_text];
        () = msg![env; this setText:text];
    }

    let key_font = get_static_str(env, "UIFont");
    if msg![env; coder containsValueForKey:key_font] {
        let font: id = msg![env; coder decodeObjectForKey:key_font];
        () = msg![env; this setFont:font];
    } else {
        () = msg![env; this setFont:nil];
    }

    let key_color = get_static_str(env, "UITextColor");
    if msg![env; coder containsValueForKey:key_color] {
        let text_color: id = msg![env; coder decodeObjectForKey:key_color];
        () = msg![env; this setTextColor:text_color];
    } else {
        () = msg![env; this setTextColor:nil];
    }

    let key_align = get_static_str(env, "UITextAlignment");
    if msg![env; coder containsValueForKey:key_align] {
        let align: UITextAlignment = msg![env; coder decodeIntegerForKey:key_align];
        () = msg![env; this setTextAlignment:align];
    }

    let key_lines = get_static_str(env, "UINumberOfLines");
    if msg![env; coder containsValueForKey:key_lines] {
        let lines: NSInteger = msg![env; coder decodeIntegerForKey:key_lines];
        () = msg![env; this setNumberOfLines:lines];
    }

    let key_break = get_static_str(env, "UILineBreakMode");
    if msg![env; coder containsValueForKey:key_break] {
        let break_mode: UILineBreakMode = msg![env; coder decodeIntegerForKey:key_break];
        () = msg![env; this setLineBreakMode:break_mode];
    }

    let key_bg = get_static_str(env, "UIBackgroundColor");
    let bg_color = if msg![env; coder containsValueForKey:key_bg] {
        msg![env; coder decodeObjectForKey:key_bg]
    } else {
        nil
    };

    if bg_color == nil {
        let clear_color: id = msg_class![env; UIColor clearColor];
        () = msg![env; this setBackgroundColor:clear_color];
    } else {
        () = msg![env; this setBackgroundColor:bg_color];
    }

    () = msg_super![env; this setOpaque:false];
    this
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    () = msg![env; this setFont:nil];
    () = msg![env; this setTextColor:nil];
    () = msg![env; this setBackgroundColor:nil];
    () = msg_super![env; this setOpaque:false];
    this
}

- (())dealloc {
    let &UILabelHostObject { text, font, text_color, highlighted_text_color, .. } = env.objc.borrow(this);
    release(env, text);
    release(env, font);
    release(env, text_color);
    release(env, highlighted_text_color);
    msg_super![env; this dealloc]
}

- (id)shadowOffset { nil }

- (id)text { env.objc.borrow::<UILabelHostObject>(this).text }
- (())setText:(id)new_text {
    let new_text: id = if new_text != nil { msg![env; new_text copy] } else { nil };
    let old_text = std::mem::replace(&mut env.objc.borrow_mut::<UILabelHostObject>(this).text, new_text);
    release(env, old_text);
    () = msg![env; this setNeedsDisplay];
}

- (id)font { env.objc.borrow::<UILabelHostObject>(this).font }
- (())setFont:(id)new_font {
    let new_font: id = if new_font == nil {
        let size: CGFloat = 17.0;
        msg_class![env; UIFont systemFontOfSize:size]
    } else { new_font };

    let old_font = std::mem::replace(&mut env.objc.borrow_mut::<UILabelHostObject>(this).font, new_font);
    retain(env, new_font);
    release(env, old_font);
    () = msg![env; this setNeedsDisplay];
}

- (bool)adjustsFontSizeToFitWidth { false }
- (())setAdjustsFontSizeToFitWidth:(bool)_adjusts { }

- (bool)isEnabled { env.objc.borrow::<UILabelHostObject>(this).enabled }
- (())setEnabled:(bool)enabled {
    env.objc.borrow_mut::<UILabelHostObject>(this).enabled = enabled;
    () = msg![env; this setNeedsDisplay];
}

- (id)textColor { env.objc.borrow::<UILabelHostObject>(this).text_color }
- (())setTextColor:(id)new_text_color {
    let new_text_color: id = if new_text_color == nil {
        msg_class![env; UIColor blackColor]
    } else { new_text_color };

    let old_text_color = std::mem::replace(&mut env.objc.borrow_mut::<UILabelHostObject>(this).text_color, new_text_color);
    retain(env, new_text_color);
    release(env, old_text_color);
    () = msg![env; this setNeedsDisplay];
}

- (id)highlightedTextColor { env.objc.borrow::<UILabelHostObject>(this).highlighted_text_color }
- (())setHighlightedTextColor:(id)new_color {
    let old_color = std::mem::replace(&mut env.objc.borrow_mut::<UILabelHostObject>(this).highlighted_text_color, new_color);
    retain(env, new_color);
    release(env, old_color);
    () = msg![env; this setNeedsDisplay];
}

- (())setBackgroundColor:(id)color {
    let color: id = if color == nil { msg_class![env; UIColor whiteColor] } else { color };
    msg_super![env; this setBackgroundColor:color]
}

- (())setShadowColor:(id)color { todo_objc_setter!(this, color); }
- (())setShadowOffset:(CGSize)value { todo_objc_setter!(this, value); }
- (())setOpaque:(bool)_opaque { }

- (UITextAlignment)textAlignment { env.objc.borrow::<UILabelHostObject>(this).text_alignment }
- (())setTextAlignment:(UITextAlignment)text_alignment {
    env.objc.borrow_mut::<UILabelHostObject>(this).text_alignment = text_alignment;
    () = msg![env; this setNeedsDisplay];
}

- (UILineBreakMode)lineBreakMode { env.objc.borrow::<UILabelHostObject>(this).line_break_mode }
- (())setLineBreakMode:(UILineBreakMode)line_break_mode {
    env.objc.borrow_mut::<UILabelHostObject>(this).line_break_mode = line_break_mode;
    () = msg![env; this setNeedsDisplay];
}

- (NSInteger)numberOfLines { env.objc.borrow::<UILabelHostObject>(this).number_of_lines }
- (())setNumberOfLines:(NSInteger)number {
    env.objc.borrow_mut::<UILabelHostObject>(this).number_of_lines = number;
    () = msg![env; this setNeedsDisplay];
}

- (())setBaselineAdjustment:(UIBaselineAdjustment)value {
    log!("TODO: [(UILabel*) {:?} setBaselineAdjustment:{}]", this, value);
}

- (())sizeToFit {
    let size: CGSize = msg![env; this sizeThatFits:(CGSize { width: CGFloat::MAX, height: CGFloat::MAX })];
    let origin: CGPoint = {
        let frame: CGRect = msg![env; this frame];
        frame.origin
    };
    let new_frame = CGRect { origin, size };
    () = msg![env; this setFrame:new_frame];
}

- (CGSize)sizeThatFits:(CGSize)size {
    let &UILabelHostObject { text, font, line_break_mode, number_of_lines, .. } =
        env.objc.borrow(this);

    if text == nil || font == nil {
        return CGSize { width: 0.0, height: 0.0 };
    }

    let len: NSUInteger = msg![env; text length];
    if len == 0 {
        // Even with empty text, UILabel reports one line height
        let line_height: CGFloat = msg![env; font lineHeight];
        return CGSize { width: 0.0, height: line_height };
    }

    let single_line = number_of_lines == 1;

    if single_line {
        let text_size: CGSize = msg![env; text sizeWithFont:font];
        text_size
    } else {
        // Constrain to the proposed width (or infinite if unconstrained),
        // with effectively unlimited height for multi-line labels.
        let max_height = if number_of_lines == 0 {
            // 0 means unlimited lines
            CGFloat::MAX
        } else {
            // number_of_lines > 1: allow that many lines worth of height
            let line_height: CGFloat = msg![env; font lineHeight];
            line_height * (number_of_lines as CGFloat) + 1.0
        };
        let constraint = CGSize { width: size.width, height: max_height };
        let text_size: CGSize = msg![env; text sizeWithFont:font
                  constrainedToSize:constraint
                      lineBreakMode:line_break_mode];
        text_size
    }
}

- (())drawRect:(CGRect)_rect {
    let bounds: CGRect = msg![env; this bounds];
    let context = UIGraphicsGetCurrentContext(env);

    let &mut UILabelHostObject {
        text, font, text_color, text_alignment, line_break_mode, number_of_lines, ..
    } = env.objc.borrow_mut(this);

    if text == nil || font == nil || text_color == nil { return; }

    let len: NSUInteger = msg![env; text length];
    if len == 0 { return; }

    let (r, g, b, a) = ui_color::get_rgba(&env.objc, text_color);
    CGContextSetRGBFillColor(env, context, r, g, b, a);

    let single_line = number_of_lines == 1;

    let calculated_size: CGSize;
    if single_line {
        calculated_size = msg![env; text sizeWithFont:font];
    } else {
        calculated_size = msg![env; text sizeWithFont:font
                  constrainedToSize:(bounds.size)
                      lineBreakMode:line_break_mode];
    }

    let rect = CGRect {
        origin: CGPoint {
            x: bounds.origin.x,
            y: bounds.origin.y + (bounds.size.height - calculated_size.height) / 2.0,
        },
        size: CGSize { width: bounds.size.width, height: calculated_size.height },
    };

    if single_line {
        let x_offset = match text_alignment {
            UITextAlignmentLeft => 0.0,
            UITextAlignmentCenter => 0.5,
            UITextAlignmentRight => 1.0,
            _ => 0.0,
        };
        let point = CGPoint {
            x: rect.origin.x + x_offset * (bounds.size.width - calculated_size.width),
            y: rect.origin.y
        };
        let _size: CGSize = msg![env; text drawAtPoint:point withFont:font];
    } else {
        let _size: CGSize = msg![env; text drawInRect:rect
                         withFont:font
                    lineBreakMode:line_break_mode
                        alignment:text_alignment];
    }
}

@end

};

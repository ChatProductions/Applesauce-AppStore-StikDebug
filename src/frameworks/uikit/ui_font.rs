/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIFont`.

use super::ui_graphics::UIGraphicsGetCurrentContext;
use crate::font::{Font, TextAlignment, WrapMode};
use crate::frameworks::core_graphics::cg_bitmap_context::CGBitmapContextDrawer;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{autorelease, id, msg, objc_classes, ClassExports, HostObject, nil};
use crate::Environment;
use std::collections::HashMap;
use std::ops::Range;

#[derive(Default)]
pub(super) struct State {
    fonts: HashMap<FontKind, Font>,
    sans_regular_ja: Option<Font>,
    sans_bold_ja: Option<Font>,
}
impl State {
    fn get_font_by_kind(&mut self, font_kind: FontKind) -> &Font {
        self.fonts
            .entry(font_kind)
            .or_insert_with(|| match font_kind {
                FontKind::MonoRegular => Font::mono_regular(),
                FontKind::MonoBold => Font::mono_bold(),
                FontKind::MonoBoldItalic => Font::mono_bold_italic(),
                FontKind::MonoItalic => Font::mono_italic(),
                FontKind::SansRegular => Font::sans_regular(),
                FontKind::SansBold => Font::sans_bold(),
                FontKind::SansBoldItalic => Font::sans_bold_italic(),
                FontKind::SansItalic => Font::sans_italic(),
                FontKind::SerifRegular => Font::serif_regular(),
                FontKind::SerifBold => Font::serif_bold(),
                FontKind::SerifBoldItalic => Font::serif_bold_italic(),
                FontKind::SerifItalic => Font::serif_italic(),
            })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum FontKind {
    MonoRegular, MonoBold, MonoBoldItalic, MonoItalic,
    SansRegular, SansBold, SansBoldItalic, SansItalic,
    SerifRegular, SerifBold, SerifBoldItalic, SerifItalic,
}

struct UIFontHostObject {
    size: CGFloat,
    kind: FontKind,
}
impl HostObject for UIFontHostObject {}

pub type UILineBreakMode = NSInteger;
pub const UILineBreakModeWordWrap: UILineBreakMode = 0;
pub const UILineBreakModeCharacterWrap: UILineBreakMode = 1;
pub const UILineBreakModeClip: UILineBreakMode = 2;
pub const UILineBreakModeHeadTruncation: UILineBreakMode = 3;
pub const UILineBreakModeTailTruncation: UILineBreakMode = 4;
pub const UILineBreakModeMiddleTruncation: UILineBreakMode = 5;

pub type UITextAlignment = NSInteger;
pub const UITextAlignmentLeft: UITextAlignment = 0;
pub const UITextAlignmentCenter: UITextAlignment = 1;
pub const UITextAlignmentRight: UITextAlignment = 2;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIFont: NSObject

+ (CGFloat)systemFontSize { 14.0 }
+ (CGFloat)smallSystemFontSize { 12.0 }
+ (CGFloat)labelFontSize { 17.0 }
+ (CGFloat)buttonFontSize { 18.0 }

+ (id)systemFontOfSize:(CGFloat)size {
    let host_object = UIFontHostObject { size, kind: FontKind::SansRegular };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}
+ (id)boldSystemFontOfSize:(CGFloat)size {
    let host_object = UIFontHostObject { size, kind: FontKind::SansBold };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}
+ (id)italicSystemFontOfSize:(CGFloat)size {
    let host_object = UIFontHostObject { size, kind: FontKind::SansItalic };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}

+ (id)fontWithName:(id)fontName size:(CGFloat)fontSize {
    if fontName == nil {
        let host_object = UIFontHostObject { kind: FontKind::SansRegular, size: fontSize };
        let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
        return autorelease(env, new);
    }
    let font_name = to_rust_string(env, fontName).to_string();
    let host_object = UIFontHostObject {
        kind: get_equivalent_font(&font_name).unwrap_or_else(|| FontKind::SansRegular),
        size: fontSize,
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}

- (CGFloat)pointSize { env.objc.borrow::<UIFontHostObject>(this).size }

- (CGFloat)ascender {
    let host_object = env.objc.borrow::<UIFontHostObject>(this);
    let font = env.framework_state.uikit.ui_font.get_font_by_kind(host_object.kind);
    font.ascent(host_object.size)
}
- (CGFloat)descender {
    let host_object = env.objc.borrow::<UIFontHostObject>(this);
    let font = env.framework_state.uikit.ui_font.get_font_by_kind(host_object.kind);
    font.descent(host_object.size)
}
- (CGFloat)leading {
    let host_object = env.objc.borrow::<UIFontHostObject>(this);
    let font = env.framework_state.uikit.ui_font.get_font_by_kind(host_object.kind);
    font.line_gap(host_object.size)
}

- (CGFloat)lineHeight {
    let ascender: CGFloat = msg![env; this ascender];
    let descender: CGFloat = msg![env; this descender];
    let leading: CGFloat = msg![env; this leading];
    ascender + leading - descender
}

- (id)fontWithSize:(CGFloat)size {
    let kind = env.objc.borrow::<UIFontHostObject>(this).kind;
    let host_object = UIFontHostObject { size, kind };
    let class_ptr = env.objc.get_known_class("UIFont", &mut env.mem);
    let new_font = env.objc.alloc_object(class_ptr, Box::new(host_object), &mut env.mem);
    autorelease(env, new_font)
}

@end

};

fn convert_line_break_mode(ui_mode: UILineBreakMode) -> WrapMode {
    match ui_mode {
        UILineBreakModeWordWrap => WrapMode::Word,
        UILineBreakModeCharacterWrap => WrapMode::Char,
        UILineBreakModeTailTruncation => WrapMode::Word,
        _ => WrapMode::Word,
    }
}

fn get_font<'a>(state: &'a mut State, kind: FontKind, text: &str) -> &'a Font {
    for c in text.chars() {
        let c = c as u32;
        if (0x3000..=0x30FF).contains(&c) || (0xFF00..=0xFFEF).contains(&c) ||
           (0x4e00..=0x9FA0).contains(&c) || (0x3400..=0x4DBF).contains(&c) {
            match kind {
                FontKind::MonoRegular | FontKind::MonoItalic | FontKind::SansRegular | FontKind::SansItalic | FontKind::SerifRegular | FontKind::SerifItalic => {
                    if state.sans_regular_ja.is_none() { state.sans_regular_ja = Some(Font::sans_regular_ja()); }
                    return state.sans_regular_ja.as_ref().unwrap();
                },
                FontKind::MonoBold | FontKind::MonoBoldItalic | FontKind::SansBold | FontKind::SansBoldItalic | FontKind::SerifBold | FontKind::SerifBoldItalic => {
                    if state.sans_bold_ja.is_none() { state.sans_bold_ja = Some(Font::sans_bold_ja()); }
                    return state.sans_bold_ja.as_ref().unwrap();
                },
            }
        }
    }
    state.get_font_by_kind(kind)
}

pub fn size_with_font(
    env: &mut Environment,
    font: id,
    text: &str,
    constrained: Option<(CGSize, UILineBreakMode)>,
) -> CGSize {
    if font == nil { return CGSize { width: 0.0, height: 0.0 }; }
    
    let host_object = env.objc.borrow::<UIFontHostObject>(font);
    let font_obj = get_font(&mut env.framework_state.uikit.ui_font, host_object.kind, text);
    let wrap = constrained.map(|(size, ui_mode)| (size.width, convert_line_break_mode(ui_mode)));

    let (width, height) = font_obj.calculate_text_size(host_object.size, text, wrap);
    CGSize { width, height }
}

pub fn break_lines_with_font<'a>(
    env: &mut Environment,
    font: id,
    text: &'a str,
    constrained: Option<(CGSize, UILineBreakMode)>,
) -> Vec<(f32, &'a str)> {
    if font == nil { return vec![]; }
    
    let host_object = env.objc.borrow::<UIFontHostObject>(font);
    let font_obj = get_font(&mut env.framework_state.uikit.ui_font, host_object.kind, text);
    let wrap = constrained.map(|(size, ui_mode)| (size.width, convert_line_break_mode(ui_mode)));

    font_obj.break_lines(host_object.size, text, wrap)
}

#[inline(always)]
fn draw_font_glyph(
    drawer: &mut CGBitmapContextDrawer,
    raster_glyph: crate::font::RasterGlyph,
    fill_color: (f32, f32, f32, f32),
    clip_x: Option<Range<f32>>,
    clip_y: Option<Range<f32>>,
) {
    let mut glyph_rect = {
        let (x, y) = raster_glyph.origin();
        let (width, height) = raster_glyph.dimensions();
        CGRect {
            origin: CGPoint { x, y },
            size: CGSize { width: width as f32, height: height as f32 },
        }
    };
    if let Some(clip_x) = clip_x {
        if glyph_rect.origin.x >= clip_x.end { return; }
        if glyph_rect.origin.x + glyph_rect.size.width > clip_x.end {
            glyph_rect.size.width = clip_x.end - glyph_rect.origin.x;
        }
    }
    if let Some(clip_y) = clip_y {
        if glyph_rect.origin.y >= clip_y.end { return; }
        if glyph_rect.origin.y + glyph_rect.size.height > clip_y.end {
            glyph_rect.size.height = clip_y.end - glyph_rect.origin.y;
        }
    }

    for ((x, y), (tex_x, tex_y)) in drawer.iter_transformed_pixels(glyph_rect) {
        let coverage = raster_glyph.pixel_at((
            (tex_x * glyph_rect.size.width - 0.5).round() as i32,
            (tex_y * glyph_rect.size.height - 0.5).round() as i32,
        ));
        let (r, g, b, a) = fill_color;
        let (r, g, b, a) = (r * coverage, g * coverage, b * coverage, a * coverage);
        drawer.put_pixel((x, y), (r, g, b, a), true);
    }
}

pub fn draw_at_point(
    env: &mut Environment,
    font: id,
    text: &str,
    point: CGPoint,
    width_and_line_break_mode: Option<(CGFloat, UILineBreakMode)>,
) -> CGSize {
    if font == nil { return CGSize { width: 0.0, height: 0.0 }; }
    
    let context = UIGraphicsGetCurrentContext(env);
    let host_object = env.objc.borrow::<UIFontHostObject>(font);
    let font_obj = get_font(&mut env.framework_state.uikit.ui_font, host_object.kind, text);
    
    let width_and_line_break_mode = width_and_line_break_mode.map(|(width, ui_mode)| (width, convert_line_break_mode(ui_mode)));
    let clip_x = width_and_line_break_mode.map(|(width, _)| point.x..(point.x + width));
    let (width, height) = font_obj.calculate_text_size(host_object.size, text, width_and_line_break_mode);
    
    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);
    let fill_color = drawer.rgb_fill_color();
    
    font_obj.draw(
        host_object.size, text, (point.x, point.y), width_and_line_break_mode, TextAlignment::Left,
        |raster_glyph| {
            draw_font_glyph(&mut drawer, raster_glyph, fill_color, clip_x.clone(), None)
        },
    );
    CGSize { width, height }
}

pub fn draw_in_rect(
    env: &mut Environment,
    font: id,
    text: &str,
    rect: CGRect,
    line_break_mode: UILineBreakMode,
    alignment: UITextAlignment,
) -> CGSize {
    if font == nil { return CGSize { width: 0.0, height: 0.0 }; }
    
    let context = UIGraphicsGetCurrentContext(env);
    let text_size = size_with_font(env, font, text, Some((rect.size, line_break_mode)));

    let host_object = env.objc.borrow::<UIFontHostObject>(font);
    let font_obj = get_font(&mut env.framework_state.uikit.ui_font, host_object.kind, text);
    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);
    let fill_color = drawer.rgb_fill_color();
    
    let (origin_x_offset, text_alignment) = match alignment {
        UITextAlignmentLeft => (0.0, TextAlignment::Left),
        UITextAlignmentCenter => (rect.size.width / 2.0, TextAlignment::Center),
        UITextAlignmentRight => (rect.size.width, TextAlignment::Right),
        _ => (0.0, TextAlignment::Left),
    };
    
    font_obj.draw(
        host_object.size, text, (rect.origin.x + origin_x_offset, rect.origin.y),
        Some((rect.size.width, convert_line_break_mode(line_break_mode))), text_alignment,
        |raster_glyph| {
            draw_font_glyph(
                &mut drawer, raster_glyph, fill_color,
                Some(rect.origin.x..(rect.origin.x + rect.size.width)),
                Some(rect.origin.y..(rect.origin.y + rect.size.height)),
            )
        },
    );
    text_size
}

fn get_equivalent_font(system_font: &str) -> Option<FontKind> {
    match system_font {
        "Courier" | "CourierNewPSMT" | "AmericanTypewriter" | "AmericanTypewriter-Condensed" | "AmericanTypewriter-CondensedLight" | "AmericanTypewriter-Light" => Some(FontKind::MonoRegular),
        "Courier-Bold" | "CourierNewPS-BoldMT" | "AmericanTypewriter-Bold" | "AmericanTypewriter-CondensedBold" | "DBLCDTempBlack" => Some(FontKind::MonoBold),
        "Courier-Oblique" | "CourierNewPS-ItalicMT" => Some(FontKind::MonoItalic),
        "Courier-BoldOblique" | "CourierNewPS-BoldItalicMT" => Some(FontKind::MonoBoldItalic),
        "ArialMT" | "Helvetica" | "Helvetica-Light" | "Helvetica-Narrow" | "HelveticaNeue" | "HelveticaNeue-Light" | "HelveticaNeue-UltraLight" | "HelveticaNeue-Thin" | "Verdana" | "TrebuchetMS" | "Futura-Medium" | "Futura-CondensedMedium" | "GillSans" | "GillSans-Light" | "Optima-Regular" | "MarkerFelt-Thin" | "ChalkboardSE-Regular" | "ChalkboardSE-Light" | "Chalkduster" | "EuphemiaUCAS" => Some(FontKind::SansRegular),
        "Arial-BoldMT" | "ArialRoundedMTBold" | "Helvetica-Bold" | "Helvetica-Narrow-Bold" | "HelveticaNeue-Bold" | "HelveticaNeue-Medium" | "HelveticaNeue-CondensedBold" | "HelveticaNeue-CondensedBlack" | "Verdana-Bold" | "TrebuchetMS-Bold" | "Futura-CondensedExtraBold" | "GillSans-Bold" | "Optima-Bold" | "Optima-ExtraBlack" | "MarkerFelt-Wide" | "ChalkboardSE-Bold" | "BradleyHandITCTT-Bold" | "EuphemiaUCAS-Bold" => Some(FontKind::SansBold),
        "Arial-ItalicMT" | "Helvetica-Oblique" | "Helvetica-LightOblique" | "Helvetica-Narrow-Oblique" | "HelveticaNeue-Italic" | "HelveticaNeue-LightItalic" | "HelveticaNeue-UltraLightItalic" | "HelveticaNeue-ThinItalic" | "Verdana-Italic" | "TrebuchetMS-Italic" | "Futura-MediumItalic" | "GillSans-Italic" | "GillSans-LightItalic" | "Optima-Italic" | "EuphemiaUCAS-Italic" => Some(FontKind::SansItalic),
        "Arial-BoldItalicMT" | "Helvetica-BoldOblique" | "Helvetica-Narrow-BoldOblique" | "HelveticaNeue-BoldItalic" | "Verdana-BoldItalic" | "TrebuchetMS-BoldItalic" | "GillSans-BoldItalic" | "Optima-BoldItalic" => Some(FontKind::SansBoldItalic),
        "TimesNewRomanPSMT" | "Georgia" | "Palatino-Roman" | "Baskerville" | "Didot" | "Cochin" => Some(FontKind::SerifRegular),
        "TimesNewRomanPS-BoldMT" | "Georgia-Bold" | "Palatino-Bold" | "Baskerville-Bold" | "Baskerville-SemiBold" | "Didot-Bold" | "Cochin-Bold" => Some(FontKind::SerifBold),
        "TimesNewRomanPS-ItalicMT" | "Georgia-Italic" | "Palatino-Italic" | "Baskerville-Italic" | "Didot-Italic" | "Cochin-Italic" => Some(FontKind::SerifItalic),
        "TimesNewRomanPS-BoldItalicMT" | "Georgia-BoldItalic" | "Palatino-BoldItalic" | "Baskerville-BoldItalic" | "Baskerville-SemiBoldItalic" | "Cochin-BoldItalic" => Some(FontKind::SerifBoldItalic),
        _ => None,
    }
    }

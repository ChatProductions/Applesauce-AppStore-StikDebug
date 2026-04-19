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
use crate::objc::{autorelease, id, msg, objc_classes, ClassExports, HostObject};
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
    MonoRegular,
    MonoBold,
    MonoBoldItalic,
    MonoItalic,
    SansRegular,
    SansBold,
    SansBoldItalic,
    SansItalic,
    SerifRegular,
    SerifBold,
    SerifBoldItalic,
    SerifItalic,
}

struct UIFontHostObject {
    size: CGFloat,
    kind: FontKind,
}
impl HostObject for UIFontHostObject {}

/// Line break mode.
///
/// This is put here for convenience since it's font-related.
/// Apple puts it in its own header, also in UIKit.
pub type UILineBreakMode = NSInteger;
pub const UILineBreakModeWordWrap: UILineBreakMode = 0;
pub const UILineBreakModeCharacterWrap: UILineBreakMode = 1;
#[allow(dead_code)]
pub const UILineBreakModeClip: UILineBreakMode = 2;
#[allow(dead_code)]
pub const UILineBreakModeHeadTruncation: UILineBreakMode = 3;
pub const UILineBreakModeTailTruncation: UILineBreakMode = 4;
#[allow(dead_code)]
pub const UILineBreakModeMiddleTruncation: UILineBreakMode = 5;

/// Text alignment.
///
/// This is put here for convenience since it's font-related.
/// Apple puts it in its own header, also in UIKit.
pub type UITextAlignment = NSInteger;
pub const UITextAlignmentLeft: UITextAlignment = 0;
pub const UITextAlignmentCenter: UITextAlignment = 1;
pub const UITextAlignmentRight: UITextAlignment = 2;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIFont: NSObject

+ (CGFloat)systemFontSize {
    14.0
}

+ (CGFloat)smallSystemFontSize {
    12.0
}

+ (CGFloat)labelFontSize {
    17.0
}

+ (CGFloat)buttonFontSize {
    18.0
}

+ (id)systemFontOfSize:(CGFloat)size {
    let host_object = UIFontHostObject {
        size,
        kind: FontKind::SansRegular,
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}
+ (id)boldSystemFontOfSize:(CGFloat)size {
    let host_object = UIFontHostObject {
        size,
        kind: FontKind::SansBold,
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}
+ (id)italicSystemFontOfSize:(CGFloat)size {
    let host_object = UIFontHostObject {
        size,
        kind: FontKind::SansItalic,
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}

+ (id)fontWithName:(id)fontName // NSString*
            size:(CGFloat)fontSize {
    // FIX: Guard against nil fontName — на реальном iOS возвращает nil,
    // но мы возвращаем системный шрифт чтобы не упасть.
    // Без этой проверки to_rust_string читает по адресу 0x0 → NULL-PAGE READ.
    if fontName.is_null() {
        log_dbg!("UIFont fontWithName:size: called with nil fontName, returning system font");
        let host_object = UIFontHostObject {
            kind: FontKind::SansRegular,
            size: fontSize,
        };
        let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
        return autorelease(env, new);
    }
    let font_name = to_rust_string(env, fontName).to_string();
    let host_object = UIFontHostObject {
        kind: get_equivalent_font(&font_name).unwrap_or_else(|| {
            log!("No replacement found for font {}. Using system font instead.", font_name);
            FontKind::SansRegular
        }),
        size: fontSize,
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}

- (CGFloat)pointSize {
    let host_object = env.objc.borrow::<UIFontHostObject>(this);
    host_object.size
}

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
    // https://developer.apple.com/library/archive/documentation/TextFonts/Conceptual/CocoaTextArchitecture/FontHandling/FontHandling.html
    let ascender: CGFloat = msg![env; this ascender];
    let descender: CGFloat = msg![env; this descender];
    let leading: CGFloat = msg![env; this leading];
    assert!(descender <= 0.0);
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
        // TODO: support this properly; fake support is so that UILabel works,
        // which has this as its default line break mode
        UILineBreakModeTailTruncation => WrapMode::Word,
        _ => unimplemented!("TODO: line break mode {}", ui_mode),
    }
}

#[rustfmt::skip]
fn get_font<'a>(state: &'a mut State, kind: FontKind, text: &str) -> &'a Font {
    // The default fonts (see font.rs) are the Liberation family, which are a
    // good substitute for Helvetica, the iPhone OS system font. Unfortunately,
    // there is no CJK support in these fonts. To support Super Monkey Ball in
    // Japanese, let's fall back to Noto Sans JP when necessary.
    // FIXME: This heuristic is incomplete and a proper font fallback system
    // should be used instead.
    for c in text.chars() {
        let c = c as u32;
        if (0x3000..=0x30FF).contains(&c) || // JA punctuation, kana
           (0xFF00..=0xFFEF).contains(&c) || // full-width/half-width chars
           (0x4e00..=0x9FA0).contains(&c) || // various kanji
           (0x3400..=0x4DBF).contains(&c) { // more kanji
            match kind {
                // CJK has no italic equivalent
                FontKind::MonoRegular | FontKind::MonoItalic | FontKind::SansRegular | FontKind::SansItalic | FontKind::SerifRegular | FontKind::SerifItalic => {
                    if state.sans_regular_ja.is_none() {
                        state.sans_regular_ja = Some(Font::sans_regular_ja());
                    }
                    return state.sans_regular_ja.as_ref().unwrap();
                },
                FontKind::MonoBold | FontKind::MonoBoldItalic | FontKind::SansBold | FontKind::SansBoldItalic | FontKind::SerifBold | FontKind::SerifBoldItalic => {
                    if state.sans_bold_ja.is_none() {
                        state.sans_bold_ja = Some(Font::sans_bold_ja());
                    }
                    return state.sans_bold_ja.as_ref().unwrap();
                },
            }
        }
    }

    state.get_font_by_kind(kind)
}

/// Called by the `sizeWithFont:` method family on `NSString`.
pub fn size_with_font(
    env: &mut Environment,
    font: id,
    text: &str,
    constrained: Option<(CGSize, UILineBreakMode)>,
) -> CGSize {
    let host_object = env.objc.borrow::<UIFontHostObject>(font);
    let font = get_font(
        &mut env.framework_state.uikit.ui_font,
        host_object.kind,
        text,
    );
    let wrap = constrained.map(|(size, ui_mode)| (size.width, convert_line_break_mode(ui_mode)));

    let (width, height) = font.calculate_text_size(host_object.size, text, wrap);
    CGSize { width, height }
}

/// Determine how the text lines will be rendered given a constraint
pub fn break_lines_with_font<'a>(
    env: &mut Environment,
    font: id,
    text: &'a str,
    constrained: Option<(CGSize, UILineBreakMode)>,
) -> Vec<(f32, &'a str)> {
    let host_object = env.objc.borrow::<UIFontHostObject>(font);
    let font = get_font(
        &mut env.framework_state.uikit.ui_font,
        host_object.kind,
        text,
    );
    let wrap = constrained.map(|(size, ui_mode)| (size.width, convert_line_break_mode(ui_mode)));

    font.break_lines(host_object.size, text, wrap)
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
            size: CGSize {
                width: width as f32,
                height: height as f32,
            },
        }
    };
    // The code in font.rs won't and can't clip glyphs hanging over the right
    // and bottom sides of the rect, so it has to be done here. Bear in mind
    // that this must not incorrectly affect the texture co-ordinates, otherwise
    // the glyphs become squashed instead.
    // Note that there isn't clipping for the other sides currently because it
    // doesn't seem to be needed.
    if let Some(clip_x) = clip_x {
        if glyph_rect.origin.x >= clip_x.end {
            return;
        }
        if glyph_rect.origin.x + glyph_rect.size.width > clip_x.end {
            glyph_rect.size.width = clip_x.end - glyph_rect.origin.x;
        }
    }
    if let Some(clip_y) = clip_y {
        if glyph_rect.origin.y >= clip_y.end {
            return;
        }
        if glyph_rect.origin.y + glyph_rect.size.height > clip_y.end {
            glyph_rect.size.height = clip_y.end - glyph_rect.origin.y;
        }
    }

    for ((x, y), (tex_x, tex_y)) in drawer.iter_transformed_pixels(glyph_rect) {
        // TODO: bilinear sampling
        let coverage = raster_glyph.pixel_at((
            (tex_x * glyph_rect.size.width - 0.5).round() as i32,
            (tex_y * glyph_rect.size.height - 0.5).round() as i32,
        ));
        let (r, g, b, a) = fill_color;
        let (r, g, b, a) = (r * coverage, g * coverage, b * coverage, a * coverage);
        drawer.put_pixel((x, y), (r, g, b, a), /* blend: */ true);
    }
}

/// Called by the `drawAtPoint:` method family on `NSString`.
pub fn draw_at_point(
    env: &mut Environment,
    font: id,
    text: &str,
    point: CGPoint,
    width_and_line_break_mode: Option<(CGFloat, UILineBreakMode)>,
) -> CGSize {
    let context = UIGraphicsGetCurrentContext(env);
    let host_object = env.objc.borrow::<UIFontHostObject>(font);

    let font = get_font(
        &mut env.framework_state.uikit.ui_font,
        host_object.kind,
        text,
    );
    let width_and_line_break_mode =
        width_and_line_break_mode.map(|(width, ui_mode)| (width, convert_line_break_mode(ui_mode)));
    let clip_x = width_and_line_break_mode.map(|(width, _)| point.x..(point.x + width));
    let (width, height) =
        font.calculate_text_size(host_object.size, text, width_and_line_break_mode);
    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);
    let fill_color = drawer.rgb_fill_color();
    font.draw(
        host_object.size,
        text,
        (point.x, point.y),
        width_and_line_break_mode,
        TextAlignment::Left,
        |raster_glyph| {
            draw_font_glyph(
                &mut drawer,
                raster_glyph,
                fill_color,
                clip_x.clone(),
                /* clip_y: */ None,
            )
        },
    );
    CGSize { width, height }
}

/// Called by the `drawInRect:` method family on `NSString`.
pub fn draw_in_rect(
    env: &mut Environment,
    font: id,
    text: &str,
    rect: CGRect,
    line_break_mode: UILineBreakMode,
    alignment: UITextAlignment,
) -> CGSize {
    let context = UIGraphicsGetCurrentContext(env);
    let text_size = size_with_font(env, font, text, Some((rect.size, line_break_mode)));

    let host_object = env.objc.borrow::<UIFontHostObject>(font);
    let font = get_font(
        &mut env.framework_state.uikit.ui_font,
        host_object.kind,
        text,
    );
    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);
    let fill_color = drawer.rgb_fill_color();
    let (origin_x_offset, alignment) = match alignment {
        UITextAlignmentLeft => (0.0, TextAlignment::Left),
        UITextAlignmentCenter => (rect.size.width / 2.0, TextAlignment::Center),
        UITextAlignmentRight => (rect.size.width, TextAlignment::Right),
        _ => unimplemented!(),
    };
    font.draw(
        host_object.size,
        text,
        (rect.origin.x + origin_x_offset, rect.origin.y),
        Some((rect.size.width, convert_line_break_mode(line_break_mode))),
        alignment,
        |raster_glyph| {
            draw_font_glyph(
                &mut drawer,
                raster_glyph,
                fill_color,
                /* clip_x: */ Some(rect.origin.x..(rect.origin.x + rect.size.width)),
                /* clip_y: */ Some(rect.origin.y..(rect.origin.y + rect.size.height)),
            )
        },
    );
    text_size
}

/// Maps iOS font PostScript names to the closest available FontKind.
///
/// Covers fonts from iOS 2.0 through iOS 4.3.5 (Simulator + device lists).
/// Returns None only for fonts with no usable Latin substitute (CJK-only, etc.)
/// — the caller will log a warning and fall back to SansRegular.
#[rustfmt::skip]
fn get_equivalent_font(system_font: &str) -> Option<FontKind> {
    match system_font {
        // ── Courier ────────────────────────────────────────────────────────────
        "Courier"                          => Some(FontKind::MonoRegular),
        "Courier-Bold"                     => Some(FontKind::MonoBold),
        "Courier-Oblique"                  => Some(FontKind::MonoItalic),
        "Courier-BoldOblique"              => Some(FontKind::MonoBoldItalic),

        // ── Courier New ────────────────────────────────────────────────────────
        "CourierNewPSMT"                   => Some(FontKind::MonoRegular),
        "CourierNewPS-BoldMT"              => Some(FontKind::MonoBold),
        "CourierNewPS-ItalicMT"            => Some(FontKind::MonoItalic),
        "CourierNewPS-BoldItalicMT"        => Some(FontKind::MonoBoldItalic),

        // ── Arial ──────────────────────────────────────────────────────────────
        "ArialMT"                          => Some(FontKind::SansRegular),
        "Arial-BoldMT"                     => Some(FontKind::SansBold),
        "Arial-ItalicMT"                   => Some(FontKind::SansItalic),
        "Arial-BoldItalicMT"               => Some(FontKind::SansBoldItalic),

        // ── Arial Rounded MT Bold ──────────────────────────────────────────────
        // FIX: was missing → caused "No replacement found" warning + potential
        // NULL-PAGE READ if fontName was nil; now mapped to SansBold.
        "ArialRoundedMTBold"               => Some(FontKind::SansBold),

        // ── Arial Unicode MS ───────────────────────────────────────────────────
        // No good single-weight Latin substitute; keep as None so the warning
        // fires but we fall back gracefully instead of crashing.
        "ArialUnicodeMS"                   => None,

        // ── Helvetica ──────────────────────────────────────────────────────────
        "Helvetica"                        => Some(FontKind::SansRegular),
        "Helvetica-Bold"                   => Some(FontKind::SansBold),
        "Helvetica-Oblique"                => Some(FontKind::SansItalic),
        "Helvetica-BoldOblique"            => Some(FontKind::SansBoldItalic),
        "Helvetica-Light"                  => Some(FontKind::SansRegular),
        "Helvetica-LightOblique"           => Some(FontKind::SansItalic),
        "Helvetica-Narrow"                 => Some(FontKind::SansRegular),
        "Helvetica-Narrow-Bold"            => Some(FontKind::SansBold),
        "Helvetica-Narrow-Oblique"         => Some(FontKind::SansItalic),
        "Helvetica-Narrow-BoldOblique"     => Some(FontKind::SansBoldItalic),

        // ── Helvetica Neue (iOS 3+) ────────────────────────────────────────────
        "HelveticaNeue"                    => Some(FontKind::SansRegular),
        "HelveticaNeue-Bold"               => Some(FontKind::SansBold),
        "HelveticaNeue-Italic"             => Some(FontKind::SansItalic),
        "HelveticaNeue-BoldItalic"         => Some(FontKind::SansBoldItalic),
        "HelveticaNeue-Light"              => Some(FontKind::SansRegular),
        "HelveticaNeue-LightItalic"        => Some(FontKind::SansItalic),
        "HelveticaNeue-Medium"             => Some(FontKind::SansBold),
        "HelveticaNeue-UltraLight"         => Some(FontKind::SansRegular),
        "HelveticaNeue-UltraLightItalic"   => Some(FontKind::SansItalic),
        "HelveticaNeue-CondensedBold"      => Some(FontKind::SansBold),
        "HelveticaNeue-CondensedBlack"     => Some(FontKind::SansBold),
        "HelveticaNeue-Thin"               => Some(FontKind::SansRegular),
        "HelveticaNeue-ThinItalic"         => Some(FontKind::SansItalic),

        // ── Verdana ────────────────────────────────────────────────────────────
        "Verdana"                          => Some(FontKind::SansRegular),
        "Verdana-Bold"                     => Some(FontKind::SansBold),
        "Verdana-Italic"                   => Some(FontKind::SansItalic),
        "Verdana-BoldItalic"               => Some(FontKind::SansBoldItalic),

        // ── Trebuchet MS (iOS 3+) ─────────────────────────────────────────────
        "TrebuchetMS"                      => Some(FontKind::SansRegular),
        "TrebuchetMS-Bold"                 => Some(FontKind::SansBold),
        "TrebuchetMS-Italic"               => Some(FontKind::SansItalic),
        "TrebuchetMS-BoldItalic"           => Some(FontKind::SansBoldItalic),

        // ── Futura (iOS 3+) ───────────────────────────────────────────────────
        "Futura-Medium"                    => Some(FontKind::SansRegular),
        "Futura-MediumItalic"              => Some(FontKind::SansItalic),
        "Futura-CondensedMedium"           => Some(FontKind::SansRegular),
        "Futura-CondensedExtraBold"        => Some(FontKind::SansBold),

        // ── Gill Sans (iOS 3+) ────────────────────────────────────────────────
        "GillSans"                         => Some(FontKind::SansRegular),
        "GillSans-Bold"                    => Some(FontKind::SansBold),
        "GillSans-Italic"                  => Some(FontKind::SansItalic),
        "GillSans-BoldItalic"              => Some(FontKind::SansBoldItalic),
        "GillSans-Light"                   => Some(FontKind::SansRegular),
        "GillSans-LightItalic"             => Some(FontKind::SansItalic),

        // ── Optima (iOS 3+) ───────────────────────────────────────────────────
        "Optima-Regular"                   => Some(FontKind::SansRegular),
        "Optima-Bold"                      => Some(FontKind::SansBold),
        "Optima-Italic"                    => Some(FontKind::SansItalic),
        "Optima-BoldItalic"                => Some(FontKind::SansBoldItalic),
        "Optima-ExtraBlack"                => Some(FontKind::SansBold),

        // ── Times New Roman ───────────────────────────────────────────────────
        "TimesNewRomanPSMT"                => Some(FontKind::SerifRegular),
        "TimesNewRomanPS-BoldMT"           => Some(FontKind::SerifBold),
        "TimesNewRomanPS-ItalicMT"         => Some(FontKind::SerifItalic),
        "TimesNewRomanPS-BoldItalicMT"     => Some(FontKind::SerifBoldItalic),

        // ── Georgia ───────────────────────────────────────────────────────────
        "Georgia"                          => Some(FontKind::SerifRegular),
        "Georgia-Bold"                     => Some(FontKind::SerifBold),
        "Georgia-Italic"                   => Some(FontKind::SerifItalic),
        "Georgia-BoldItalic"               => Some(FontKind::SerifBoldItalic),

        // ── Palatino (iOS 3+) ─────────────────────────────────────────────────
        "Palatino-Roman"                   => Some(FontKind::SerifRegular),
        "Palatino-Bold"                    => Some(FontKind::SerifBold),
        "Palatino-Italic"                  => Some(FontKind::SerifItalic),
        "Palatino-BoldItalic"              => Some(FontKind::SerifBoldItalic),

        // ── Baskerville (iOS 4+) ──────────────────────────────────────────────
        "Baskerville"                      => Some(FontKind::SerifRegular),
        "Baskerville-Bold"                 => Some(FontKind::SerifBold),
        "Baskerville-Italic"               => Some(FontKind::SerifItalic),
        "Baskerville-BoldItalic"           => Some(FontKind::SerifBoldItalic),
        "Baskerville-SemiBold"             => Some(FontKind::SerifBold),
        "Baskerville-SemiBoldItalic"       => Some(FontKind::SerifBoldItalic),

        // ── Didot (iOS 4+) ────────────────────────────────────────────────────
        "Didot"                            => Some(FontKind::SerifRegular),
        "Didot-Bold"                       => Some(FontKind::SerifBold),
        "Didot-Italic"                     => Some(FontKind::SerifItalic),

        // ── Cochin (iOS 3+) ───────────────────────────────────────────────────
        "Cochin"                           => Some(FontKind::SerifRegular),
        "Cochin-Bold"                      => Some(FontKind::SerifBold),
        "Cochin-Italic"                    => Some(FontKind::SerifItalic),
        "Cochin-BoldItalic"                => Some(FontKind::SerifBoldItalic),

        // ── American Typewriter ───────────────────────────────────────────────
        "AmericanTypewriter"               => Some(FontKind::MonoRegular),
        "AmericanTypewriter-Bold"          => Some(FontKind::MonoBold),
        "AmericanTypewriter-Condensed"     => Some(FontKind::MonoRegular),
        "AmericanTypewriter-CondensedBold" => Some(FontKind::MonoBold),
        "AmericanTypewriter-CondensedLight"=> Some(FontKind::MonoRegular),
        "AmericanTypewriter-Light"         => Some(FontKind::MonoRegular),

        // ── Marker Felt ───────────────────────────────────────────────────────
        "MarkerFelt-Thin"                  => Some(FontKind::SansRegular),
        "MarkerFelt-Wide"                  => Some(FontKind::SansBold),

        // ── Chalkboard SE (iOS 3+) ────────────────────────────────────────────
        "ChalkboardSE-Regular"             => Some(FontKind::SansRegular),
        "ChalkboardSE-Bold"                => Some(FontKind::SansBold),
        "ChalkboardSE-Light"               => Some(FontKind::SansRegular),

        // ── Chalkduster (iOS 3+) ──────────────────────────────────────────────
        "Chalkduster"                      => Some(FontKind::SansRegular),

        // ── Bradley Hand ──────────────────────────────────────────────────────
        "BradleyHandITCTT-Bold"            => Some(FontKind::SansBold),

        // ── Euphemia UCAS (iOS 4+) ────────────────────────────────────────────
        "EuphemiaUCAS"                     => Some(FontKind::SansRegular),
        "EuphemiaUCAS-Bold"                => Some(FontKind::SansBold),
        "EuphemiaUCAS-Italic"              => Some(FontKind::SansItalic),

        // ── DB LCD Temp ───────────────────────────────────────────────────────
        "DBLCDTempBlack"                   => Some(FontKind::MonoBold),

        // ── CJK / special fonts — no usable Latin substitute ──────────────────
        "AppleGothic"                      => None, // Korean
        "STHeitiSC-Light"                  => None, // Simplified Chinese
        "STHeitiSC-Medium"                 => None,
        "STHeitiSC-Thin"                   => None,
        "STHeitiTC-Light"                  => None, // Traditional Chinese
        "STHeitiTC-Medium"                 => None,
        "STHeitiTC-Thin"                   => None,
        "HiraKakuProN-W3"                  => None, // Japanese
        "HiraKakuProN-W6"                  => None,
        "Zapfino"                          => None, // decorative, no substitute

        // ── Unknown font ─────────────────────────────────────────────────────
        _ => None,
    }
}


/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//!
//! `CALayer`.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_animation::ca_transform3d::{CATransform3D, CATransform3DIdentity};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::core_graphics::cg_affine_transform::{
    CGAffineTransform, CGAffineTransformIdentity,
};
use crate::frameworks::core_graphics::cg_bitmap_context::{
    CGBitmapContextCreate, CGBitmapContextGetHeight, CGBitmapContextGetWidth,
};
use crate::frameworks::core_graphics::cg_color::{CGColorHostObject, CGColorRef};
use crate::frameworks::core_graphics::cg_color_space::CGColorSpaceCreateDeviceRGB;
use crate::frameworks::core_graphics::cg_context::{
    CGContextClearRect, CGContextRef, CGContextRelease, CGContextTranslateCTM,
};
use crate::frameworks::core_graphics::cg_image::{
    kCGImageAlphaPremultipliedLast, kCGImageByteOrder32Big,
};
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::{self, to_rust_string};
use crate::mem::{GuestUSize, Ptr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, todo_objc_setter,
    ClassExports, HostObject, ObjC,
};
use crate::Environment;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub(super) struct CALayerHostObject {
    delegate: id,
    pub(super) sublayers: Vec<id>,
    superlayer: id,
    pub(super) bounds: CGRect,
    pub(super) position: CGPoint,
    pub(super) z_position: CGFloat, // <-- ДОБАВЛЕНО СВОЙСТВО Z-POSITION
    pub(super) anchor_point: CGPoint,
    pub(super) affine_transform: CGAffineTransform,
    /// Full 3D transform set via `-[CALayer setTransform:]`. touchHLE's
    /// renderer is 2D-only, so we extract the 2x3 affine submatrix from
    /// the assigned `CATransform3D` and store it in `affine_transform`
    /// (used by the existing `frame`/`bounds` machinery). The full 4x4
    /// matrix is kept here so `-[CALayer transform]` can roundtrip the
    /// value the app assigned.
    pub(super) transform_3d: CATransform3D,
    /// `CALayer.sublayerTransform` — a transform applied to the layer's
    /// sublayers when they are rendered. Defaults to the identity matrix.
    /// Stored verbatim so reads round-trip; touchHLE's 2D renderer doesn't
    /// currently apply this when compositing sublayers, but apps that set
    /// and read it back observe the right values.
    pub(super) sublayer_transform: CATransform3D,
    pub(super) hidden: bool,
    pub(super) opaque: bool,
    pub(super) opacity: f32,
    pub(super) background_color: Option<CGColorHostObject>,
    /// CGImageRef for pattern backgrounds (set via colorWithPatternImage:)
    pub(super) background_pattern_cg_image: id,
    pub(super) background_pattern_gles_texture: Option<crate::gles::gles11_raw::types::GLuint>,
    pub(super) corner_radius: CGFloat,
    pub(super) border_width: CGFloat,
    pub(super) border_color: Option<CGColorHostObject>,
    pub(super) needs_display: bool,
    pub(super) needs_display_on_bounds_change: bool,
    pub(super) contents: id,
    pub(super) drawable_properties: id,
    pub(super) presented_pixels: Option<(Vec<u8>, u32, u32)>,
    pub(super) cg_context: Option<CGContextRef>,
    pub(super) gles_texture: Option<crate::gles::gles11_raw::types::GLuint>,
    pub(super) gles_texture_is_up_to_date: bool,
    pub(super) animations: HashMap<String, id>,
    pub(super) anonymous_animations: HashSet<id>,
    pub(super) name: Option<String>,
    pub(super) mask: id,
}
impl HostObject for CALayerHostObject {}

impl CALayerHostObject {
    pub(super) fn superlayer_to_layer_transform(&self) -> CGAffineTransform {
        CGAffineTransform::make_translation(-self.bounds.origin.x, -self.bounds.origin.y)
            .concat(CGAffineTransform::make_translation(
                -self.bounds.size.width * self.anchor_point.x,
                -self.bounds.size.height * self.anchor_point.y,
            ))
            .concat(self.affine_transform)
            .concat(CGAffineTransform::make_translation(
                self.position.x,
                self.position.y,
            ))
    }
}

/// Set a CGImage as the tiled background pattern for this layer.
/// Called from UIView when a pattern-based UIColor is set as backgroundColor.
pub fn set_background_pattern_cg_image(env: &mut Environment, layer: id, cg_image: id) {
    use crate::objc::{release, retain};
    retain(env, cg_image);
    let old = env
        .objc
        .borrow::<CALayerHostObject>(layer)
        .background_pattern_cg_image;
    release(env, old);
    env.objc
        .borrow_mut::<CALayerHostObject>(layer)
        .background_pattern_cg_image = cg_image;
}

pub const kCAFilterLinear: &str = "kCAFilterLinear";
pub const kCAFilterNearest: &str = "kCAFilterNearest";
pub const kCAFilterTrilinear: &str = "kCAFilterTrilinear";
pub const kCAGravityCenter: &str = "center";
// Apple QuartzCore framework — `CALayer.h` declares these as
// `CA_EXTERN NSString * const kCAGravity*` strings. The literal values
// match what real Core Animation uses internally and what `isEqual:`
// comparisons on the contentsGravity property test against.
pub const kCAGravityResize: &str = "resize";
pub const kCAGravityResizeAspect: &str = "resizeAspect";
pub const kCAGravityResizeAspectFill: &str = "resizeAspectFill";
pub const kCAGravityTop: &str = "top";
pub const kCAGravityBottom: &str = "bottom";
pub const kCAGravityLeft: &str = "left";
pub const kCAGravityRight: &str = "right";
pub const kCAGravityTopLeft: &str = "topLeft";
pub const kCAGravityTopRight: &str = "topRight";
pub const kCAGravityBottomLeft: &str = "bottomLeft";
pub const kCAGravityBottomRight: &str = "bottomRight";

pub const CONSTANTS: ConstantExports = &[
    ("_kCAFilterLinear", HostConstant::NSString(kCAFilterLinear)),
    (
        "_kCAFilterNearest",
        HostConstant::NSString(kCAFilterNearest),
    ),
    (
        "_kCAFilterTrilinear",
        HostConstant::NSString(kCAFilterTrilinear),
    ),
    (
        "_kCAGravityCenter",
        HostConstant::NSString(kCAGravityCenter),
    ),
    (
        "_kCAGravityResize",
        HostConstant::NSString(kCAGravityResize),
    ),
    (
        "_kCAGravityResizeAspect",
        HostConstant::NSString(kCAGravityResizeAspect),
    ),
    (
        "_kCAGravityResizeAspectFill",
        HostConstant::NSString(kCAGravityResizeAspectFill),
    ),
    ("_kCAGravityTop", HostConstant::NSString(kCAGravityTop)),
    (
        "_kCAGravityBottom",
        HostConstant::NSString(kCAGravityBottom),
    ),
    ("_kCAGravityLeft", HostConstant::NSString(kCAGravityLeft)),
    (
        "_kCAGravityRight",
        HostConstant::NSString(kCAGravityRight),
    ),
    (
        "_kCAGravityTopLeft",
        HostConstant::NSString(kCAGravityTopLeft),
    ),
    (
        "_kCAGravityTopRight",
        HostConstant::NSString(kCAGravityTopRight),
    ),
    (
        "_kCAGravityBottomLeft",
        HostConstant::NSString(kCAGravityBottomLeft),
    ),
    (
        "_kCAGravityBottomRight",
        HostConstant::NSString(kCAGravityBottomRight),
    ),
];
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CALayer: NSObject

+ (id)alloc {
    let host_object = Box::new(CALayerHostObject {
        delegate: nil,
        sublayers: Vec::new(),
        superlayer: nil,
        bounds: CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize { width: 0.0, height: 0.0 }
        },
        position: CGPoint { x: 0.0, y: 0.0 },
        z_position: 0.0, // <-- ИНИЦИАЛИЗАЦИЯ Z-POSITION
        anchor_point: CGPoint { x: 0.5, y: 0.5 },
        affine_transform: CGAffineTransformIdentity,
        transform_3d: CATransform3DIdentity,
        sublayer_transform: CATransform3DIdentity,
        hidden: false,
        opaque: false,
        opacity: 1.0,
        background_color: None,
        background_pattern_cg_image: nil,
        background_pattern_gles_texture: None,
        corner_radius: 0.0,
        border_width: 0.0,
        border_color: None,
        needs_display: false,
        needs_display_on_bounds_change: false,
        contents: nil,
        drawable_properties: nil,
        presented_pixels: None,
        cg_context: None,
        gles_texture: None,
        gles_texture_is_up_to_date: false,
        animations: HashMap::new(),
        anonymous_animations: HashSet::new(),
        name: None,
        mask: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)layer {
    let new_layer: id = msg![env; this alloc];
    msg![env; new_layer init]
}

- (())dealloc {
    let &mut CALayerHostObject {
        drawable_properties,
        contents,
        superlayer,
        cg_context,
        mask,
        ref mut sublayers,
        ..
    } = env.objc.borrow_mut(this);
    let sublayers = std::mem::take(sublayers);

    if drawable_properties != nil { release(env, drawable_properties); }
    if contents != nil { release(env, contents); }
    if mask != nil { release(env, mask); }
    if let Some(cg_context) = cg_context { CGContextRelease(env, cg_context); }

    // On real iOS a layer being deallocated cannot have a superlayer,
    // because the superlayer's `sublayers` array holds a strong reference
    // and would keep the retain count above zero. In touchHLE the
    // retain/release accounting is occasionally off for games that mix
    // direct -release with cached `id` references (e.g. Chuzzle's alert-
    // view init path which produced HyperHLE log #3 — the dealloc fires
    // while the layer is still installed in the alert view hierarchy).
    // Panicking the whole emulator over a reference-counting glitch in
    // the guest is worse than the alternative, so we instead detach
    // ourselves from the superlayer gracefully. This matches what
    // CoreAnimation's own internal teardown does when a layer is force-
    // released through CFRelease while still parented.
    if superlayer != nil {
        log!(
            "Warning: CALayer {:?} is being deallocated while still attached \
             to superlayer {:?}; detaching to avoid a dangling sublayer \
             reference.",
            this,
            superlayer
        );
        let CALayerHostObject { sublayers: ref mut super_sublayers, .. } =
            env.objc.borrow_mut(superlayer);
        super_sublayers.retain(|&sublayer| sublayer != this);
        // Clear our own back-pointer so the recursive cleanup below sees a
        // clean state if something unexpected re-enters.
        env.objc.borrow_mut::<CALayerHostObject>(this).superlayer = nil;
    }

    for sublayer in sublayers {
        env.objc.borrow_mut::<CALayerHostObject>(sublayer).superlayer = nil;
        release(env, sublayer);
    }

    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)delegate { env.objc.borrow::<CALayerHostObject>(this).delegate }
- (())setDelegate:(id)delegate { env.objc.borrow_mut::<CALayerHostObject>(this).delegate = delegate; }

- (id)superlayer { env.objc.borrow::<CALayerHostObject>(this).superlayer }

- (())addSublayer:(id)layer {
    if layer == nil { return; }
    if env.objc.borrow::<CALayerHostObject>(layer).superlayer == this {
        () = msg![env; this bringSublayerToFront:layer];
    } else {
        retain(env, layer);
        () = msg![env; layer removeFromSuperlayer];
        env.objc.borrow_mut::<CALayerHostObject>(layer).superlayer = this;
        env.objc.borrow_mut::<CALayerHostObject>(this).sublayers.push(layer);
    }
}

- (())insertSublayer:(id)layer atIndex:(u32)idx {
    if layer == nil { return; }
    retain(env, layer);
    () = msg![env; layer removeFromSuperlayer];
    env.objc.borrow_mut::<CALayerHostObject>(layer).superlayer = this;
    let CALayerHostObject { ref mut sublayers, .. } = env.objc.borrow_mut(this);
    sublayers.insert(idx.try_into().unwrap(), layer);
}

- (())insertSublayer:(id)layer below:(id)sibling {
    if layer == nil { return; }
    retain(env, layer);
    () = msg![env; layer removeFromSuperlayer];
    env.objc.borrow_mut::<CALayerHostObject>(layer).superlayer = this;
    let CALayerHostObject { ref mut sublayers, .. } = env.objc.borrow_mut(this);
    let idx = sublayers.iter().position(|&sublayer| sublayer == sibling).unwrap_or(0);
    sublayers.insert(idx, layer);
}

- (())insertSublayer:(id)layer above:(id)sibling {
    if layer == nil { return; }
    retain(env, layer);
    () = msg![env; layer removeFromSuperlayer];
    env.objc.borrow_mut::<CALayerHostObject>(layer).superlayer = this;
    let CALayerHostObject { ref mut sublayers, .. } = env.objc.borrow_mut(this);
    let idx = sublayers
        .iter()
        .position(|&sublayer| sublayer == sibling)
        .map(|i| i + 1)
        .unwrap_or(sublayers.len());
    sublayers.insert(idx, layer);
}

- (())replaceSublayer:(id)old_layer with:(id)new_layer {
    if old_layer == nil || new_layer == nil || old_layer == new_layer { return; }
    let old_idx = {
        let host = env.objc.borrow::<CALayerHostObject>(this);
        host.sublayers.iter().position(|&x| x == old_layer)
    };
    if old_idx.is_some() {
        retain(env, new_layer);
        () = msg![env; new_layer removeFromSuperlayer];
        let host = env.objc.borrow_mut::<CALayerHostObject>(this);
        if let Some(actual_idx) = host.sublayers.iter().position(|&x| x == old_layer) {
            host.sublayers[actual_idx] = new_layer;
            env.objc.borrow_mut::<CALayerHostObject>(new_layer).superlayer = this;
            env.objc.borrow_mut::<CALayerHostObject>(old_layer).superlayer = nil;
            release(env, old_layer);
        } else {
            release(env, new_layer);
        }
    }
}

- (())removeFromSuperlayer {
    let CALayerHostObject { ref mut superlayer, .. } = env.objc.borrow_mut(this);
    let superlayer = std::mem::take(superlayer);
    if superlayer == nil { return; }
    let CALayerHostObject { ref mut sublayers, .. } = env.objc.borrow_mut(superlayer);
    let idx = sublayers.iter().position(|&sublayer| sublayer == this).unwrap();
    let sublayer = sublayers.remove(idx);
    assert!(sublayer == this);
    release(env, this);
}

- (CGRect)bounds { env.objc.borrow::<CALayerHostObject>(this).bounds }
- (())setBounds:(CGRect)bounds {
    let host_object = env.objc.borrow_mut::<CALayerHostObject>(this);
    host_object.bounds = bounds;
    if host_object.needs_display_on_bounds_change {
        () = msg![env; this setNeedsDisplay];
    }
}

- (CGPoint)position { env.objc.borrow::<CALayerHostObject>(this).position }
- (())setPosition:(CGPoint)position { env.objc.borrow_mut::<CALayerHostObject>(this).position = position; }

// --- ДОБАВЛЕНЫ МЕТОДЫ ДЛЯ Z-POSITION ---
- (CGFloat)zPosition { env.objc.borrow::<CALayerHostObject>(this).z_position }
- (())setZPosition:(CGFloat)z_position { env.objc.borrow_mut::<CALayerHostObject>(this).z_position = z_position; }
// ---------------------------------------

- (CGPoint)anchorPoint { env.objc.borrow::<CALayerHostObject>(this).anchor_point }
- (())setAnchorPoint:(CGPoint)anchor_point { env.objc.borrow_mut::<CALayerHostObject>(this).anchor_point = anchor_point; }

- (CGAffineTransform)affineTransform { env.objc.borrow::<CALayerHostObject>(this).affine_transform }
- (())setAffineTransform:(CGAffineTransform)affine_transform {
    let host_obj = env.objc.borrow_mut::<CALayerHostObject>(this);
    host_obj.affine_transform = affine_transform;
    // Keep transform_3d in sync so a subsequent -transform read returns
    // the equivalent CATransform3D, matching iOS behaviour.
    host_obj.transform_3d = affine_transform_to_catransform3d(affine_transform);
}

// `-[CALayer transform]` is a CATransform3D (4x4 matrix). touchHLE's
// renderer is 2D, so a CATransform3D assigned here is collapsed to its 2x3
// affine submatrix for the existing frame/bounds pipeline; the full 4x4
// is kept for roundtrip reads. iMilk (HyperHLE appdb report #70) was the
// motivating case — without these the app crashed with "CALayer does not
// respond to setTransform:".
- (CATransform3D)transform { env.objc.borrow::<CALayerHostObject>(this).transform_3d }
- (())setTransform:(CATransform3D)transform {
    let host_obj = env.objc.borrow_mut::<CALayerHostObject>(this);
    host_obj.transform_3d = transform;
    host_obj.affine_transform = catransform3d_to_affine(transform);
}

// `-[CALayer sublayerTransform]` / `-setSublayerTransform:` — the
// `CATransform3D` applied to this layer's sublayers when rendering.
- (CATransform3D)sublayerTransform {
    env.objc.borrow::<CALayerHostObject>(this).sublayer_transform
}
- (())setSublayerTransform:(CATransform3D)transform {
    env.objc.borrow_mut::<CALayerHostObject>(this).sublayer_transform = transform;
}

- (CGRect)frame {
    let host_obj @ &CALayerHostObject { bounds, .. } = env.objc.borrow(this);
    host_obj.superlayer_to_layer_transform().apply_to_rect(CGRect {
        origin: CGPoint { x: bounds.origin.x, y: bounds.origin.y },
        size: bounds.size,
    })
}
- (())setFrame:(CGRect)frame {
    let CALayerHostObject { anchor_point, affine_transform, .. } = env.objc.borrow_mut(this);
    let inverse_transform = CGAffineTransform::make_translation(
        -frame.size.width * anchor_point.x,
        -frame.size.height * anchor_point.y,
    ).concat(*affine_transform).invert();
    let transformed_size = inverse_transform.apply_to_rect(CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: frame.size
    }).size;
    let transformed_offset = inverse_transform.apply_to_point(CGPoint { x: 0.0, y: 0.0 });
    let new_position = CGPoint {
        x: frame.origin.x + transformed_offset.x,
        y: frame.origin.y + transformed_offset.y,
    };
    () = msg![env; this setPosition:new_position];
    let new_bounds = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: transformed_size,
    };
    () = msg![env; this setBounds:new_bounds];
}

- (())renderInContext {

}

- (bool)isHidden { env.objc.borrow::<CALayerHostObject>(this).hidden }
- (())setHidden:(bool)hidden { env.objc.borrow_mut::<CALayerHostObject>(this).hidden = hidden; }

- (bool)isOpaque { env.objc.borrow::<CALayerHostObject>(this).opaque }
- (())setOpaque:(bool)opaque { env.objc.borrow_mut::<CALayerHostObject>(this).opaque = opaque; }

- (f32)opacity { env.objc.borrow::<CALayerHostObject>(this).opacity }
- (())setOpacity:(f32)opacity { env.objc.borrow_mut::<CALayerHostObject>(this).opacity = opacity; }

- (CGColorRef)backgroundColor {
    if let Some(bg_color) = env.objc.borrow::<CALayerHostObject>(this).background_color {
        let class = env.objc.get_known_class("_touchHLE_CGColor", &mut env.mem);
        let obj = env.objc.alloc_object(class, Box::new(bg_color), &mut env.mem);
        autorelease(env, obj)
    } else { nil }
}
- (())setBackgroundColor:(CGColorRef)new_color {
    let new_color = if new_color == nil { None } else { Some(*env.objc.borrow::<CGColorHostObject>(new_color)) };
    env.objc.borrow_mut::<CALayerHostObject>(this).background_color = new_color;
}

- (CGFloat)cornerRadius { env.objc.borrow::<CALayerHostObject>(this).corner_radius }
- (())setCornerRadius:(CGFloat)corner_radius { env.objc.borrow_mut::<CALayerHostObject>(this).corner_radius = corner_radius; }

- (CGFloat)borderWidth { env.objc.borrow::<CALayerHostObject>(this).border_width }
- (())setBorderWidth:(CGFloat)border_width { env.objc.borrow_mut::<CALayerHostObject>(this).border_width = border_width; }

- (CGColorRef)borderColor {
    if let Some(border_color) = env.objc.borrow::<CALayerHostObject>(this).border_color {
        let class = env.objc.get_known_class("_touchHLE_CGColor", &mut env.mem);
        let obj = env.objc.alloc_object(class, Box::new(border_color), &mut env.mem);
        autorelease(env, obj)
    } else { nil }
}
- (())setBorderColor:(CGColorRef)new_color {
    let new_color = if new_color == nil { None } else { Some(*env.objc.borrow::<CGColorHostObject>(new_color)) };
    env.objc.borrow_mut::<CALayerHostObject>(this).border_color = new_color;
}

- (bool)needsDisplay { env.objc.borrow::<CALayerHostObject>(this).needs_display }
- (())setNeedsDisplay { env.objc.borrow_mut::<CALayerHostObject>(this).needs_display = true; }

- (bool)needsDisplayOnBoundsChange { env.objc.borrow::<CALayerHostObject>(this).needs_display_on_bounds_change }
- (())setNeedsDisplayOnBoundsChange:(bool)value { env.objc.borrow_mut::<CALayerHostObject>(this).needs_display_on_bounds_change = value; }

- (())displayIfNeeded {
    let &mut CALayerHostObject {
        ref mut needs_display,
        delegate,
        ..
    } = env.objc.borrow_mut(this);
    if !std::mem::take(needs_display) { return; }
    if delegate == nil { return; }

    let delegate_class = ObjC::read_isa(delegate, &env.mem);
    if env.objc.class_has_method_named(delegate_class, "displayLayer:") {
        () = msg![env; delegate displayLayer:this];
        return;
    }

    let &mut CALayerHostObject {
        cg_context,
        ref mut gles_texture_is_up_to_date,
        bounds: CGRect { origin, size },
        ..
    } = env.objc.borrow_mut(this);
    *gles_texture_is_up_to_date = false;

    let int_width = size.width.round() as GuestUSize;
    let int_height = size.height.round() as GuestUSize;
    // --- ФИКС КРАША 0x0 ---
    if int_width == 0 || int_height == 0 {
        return;
    }

    let need_new_context = cg_context.is_none_or(|existing|
            CGBitmapContextGetWidth(env, existing) != int_width ||
            CGBitmapContextGetHeight(env, existing) != int_height
    );
    let cg_context = if need_new_context {
        if let Some(old_context) = cg_context { CGContextRelease(env, old_context); }
        let color_space = CGColorSpaceCreateDeviceRGB(env);
        let cg_context = CGBitmapContextCreate(
            env, Ptr::null(), int_width, int_height, 8,
            int_width.checked_mul(4).unwrap(), color_space,
            kCGImageByteOrder32Big | kCGImageAlphaPremultipliedLast
        );
        env.objc.borrow_mut::<CALayerHostObject>(this).cg_context = Some(cg_context);
        cg_context
    } else {
        cg_context.unwrap()
    };
    CGContextTranslateCTM(env, cg_context, -origin.x, -origin.y);
    CGContextClearRect(env, cg_context, CGRect { origin, size });
    () = msg![env; delegate drawLayer:this inContext:cg_context];
    CGContextTranslateCTM(env, cg_context, origin.x, origin.y);
}

- (id)contents { env.objc.borrow::<CALayerHostObject>(this).contents }
- (())setContents:(id)new_contents {
    let host_obj = env.objc.borrow_mut::<CALayerHostObject>(this);
    host_obj.gles_texture_is_up_to_date = false;
    let old_contents = std::mem::replace(&mut host_obj.contents, new_contents);
    retain(env, new_contents);
    release(env, old_contents);
}

- (id)name {
    if let Some(ref name) = env.objc.borrow::<CALayerHostObject>(this).name {
        let string_id = ns_string::from_rust_string(env, name.clone());
        autorelease(env, string_id)
    } else { nil }
}

- (())setName:(id)name {
    let name_str = if name != nil { Some(ns_string::to_rust_string(env, name).into_owned()) } else { None };
    env.objc.borrow_mut::<CALayerHostObject>(this).name = name_str;
}

- (id)mask { env.objc.borrow::<CALayerHostObject>(this).mask }

- (())setMask:(id)mask {
    let old_mask = env.objc.borrow::<CALayerHostObject>(this).mask;
    if mask != old_mask {
        if mask != nil { retain(env, mask); }
        env.objc.borrow_mut::<CALayerHostObject>(this).mask = mask;
        if old_mask != nil { release(env, old_mask); }
    }
}

- (())setEdgeAntialiasingMask:(u32)mask { todo_objc_setter!(this, mask); }
- (())setMagnificationFilter:(id)filter { todo_objc_setter!(this, ns_string::to_rust_string(env, filter)); }
- (())setMinificationFilter:(id)filter { todo_objc_setter!(this, ns_string::to_rust_string(env, filter)); }

- (bool)containsPoint:(CGPoint)point {
    let bounds: CGRect = msg![env; this bounds];
    let x_range = bounds.origin.x..(bounds.origin.x + bounds.size.width);
    let y_range = bounds.origin.y..(bounds.origin.y + bounds.size.height);
    let CGPoint {x, y} = point;
    x_range.contains(&x) && y_range.contains(&y)
}

- (CGPoint)convertPoint:(CGPoint)point fromLayer:(id)other {
    if this == other { return point; }
    transform_for_conversion(env, this, other).apply_to_point(point)
}
- (CGPoint)convertPoint:(CGPoint)point toLayer:(id)other {
    if this == other { return point; }
    transform_for_conversion(env, other, this).apply_to_point(point)
}
- (CGRect)convertRect:(CGRect)rect fromLayer:(id)other {
    if this == other { return rect; }
    transform_for_conversion(env, this, other).apply_to_rect(rect)
}
- (CGRect)convertRect:(CGRect)rect toLayer:(id)other {
    if this == other { return rect; }
    transform_for_conversion(env, other, this).apply_to_rect(rect)
}

- (())addAnimation:(id)anim forKey:(id)key {
    let duration: CFTimeInterval = msg![env; anim duration];
    if duration == 0.0 {
        let duration: CFTimeInterval = msg_class![env; CATransaction animationDuration];
        () = msg![env; anim setDuration:duration];
    }
    if key == nil {
        let inserted = env.objc.borrow_mut::<CALayerHostObject>(this).anonymous_animations.insert(anim);
        assert!(inserted);
    } else {
        let key_string = to_rust_string(env, key);
        env.objc.borrow_mut::<CALayerHostObject>(this).animations.insert(key_string.to_string(), anim);
    }
    retain(env, anim);
}

- (())removeAnimationForKey:(id)key {
    let key_string = to_rust_string(env, key);
    if let Some(anim) = env.objc.borrow_mut::<CALayerHostObject>(this).animations.remove(&*key_string) {
        release(env, anim);
    };
}

// --- ДОБАВЛЕННЫЙ МЕТОД: removeAllAnimations ---
- (())removeAllAnimations {
    let host = env.objc.borrow_mut::<CALayerHostObject>(this);

    // Забираем коллекции, оставляя пустые на их месте
    let named_animations = std::mem::take(&mut host.animations);
    let anonymous_animations = std::mem::take(&mut host.anonymous_animations);

    // Освобождаем память (release) для каждой именованной анимации
    for (_, anim) in named_animations {
        release(env, anim);
    }

    // Освобождаем память (release) для каждой анонимной анимации
    for anim in anonymous_animations {
        release(env, anim);
    }
}

@end

};

/// Project a `CGAffineTransform` (2x3 matrix used by `setAffineTransform:`)
/// up into the equivalent `CATransform3D` used by `setTransform:`. This is
/// the documented `CATransform3DMakeAffineTransform(t)` mapping.
fn affine_transform_to_catransform3d(t: CGAffineTransform) -> CATransform3D {
    CATransform3D::from_affine(t)
}

/// Collapse a `CATransform3D` to its 2x3 affine submatrix, the way the
/// system's `CATransform3DGetAffineTransform` does. The 3D-only entries
/// (m13/m14/m23/m24/m31..m34/m43/m44) are dropped — touchHLE's renderer
/// is 2D so layers with non-trivial 3D content just get their projected
/// 2D shadow.
fn catransform3d_to_affine(t: CATransform3D) -> CGAffineTransform {
    t.to_affine()
}

pub fn remove_anonymous_animation(env: &mut Environment, layer: id, animation: id) {
    let removed = env
        .objc
        .borrow_mut::<CALayerHostObject>(layer)
        .anonymous_animations
        .remove(&animation);
    assert!(removed);
    release(env, animation);
}

fn transform_for_conversion(env: &mut Environment, this: id, other: id) -> CGAffineTransform {
    let need_common_ancestor = this != nil && other != nil;
    assert!(!(this == nil && other == nil));

    let mut this_map = HashMap::from([(this, CGAffineTransformIdentity)]);
    let mut other_map = HashMap::from([(other, CGAffineTransformIdentity)]);
    let mut this_superlayer = this;
    let mut this_transform = CGAffineTransformIdentity;
    let mut other_superlayer = other;
    let mut other_transform = CGAffineTransformIdentity;
    let (common_ancestor, this_transform, other_transform) = loop {
        if this_superlayer != nil {
            let this_hostobj: &CALayerHostObject = env.objc.borrow(this_superlayer);
            let next = this_hostobj.superlayer;
            let next_transform =
                this_transform.concat(this_hostobj.superlayer_to_layer_transform());
            if need_common_ancestor && next != nil {
                if let Some(&other_transform) = other_map.get(&next) {
                    break (next, next_transform, other_transform);
                }
                this_map.insert(next, next_transform);
            }
            this_superlayer = next;
            this_transform = next_transform;
        }

        if other_superlayer != nil {
            let other_hostobj: &CALayerHostObject = env.objc.borrow(other_superlayer);
            let next = other_hostobj.superlayer;
            let next_transform =
                other_transform.concat(other_hostobj.superlayer_to_layer_transform());
            if need_common_ancestor && next != nil {
                if let Some(&this_transform) = this_map.get(&next) {
                    break (next, this_transform, next_transform);
                }
                other_map.insert(next, next_transform);
            }
            other_superlayer = next;
            other_transform = next_transform;
        }

        if this_superlayer == nil && other_superlayer == nil {
            if need_common_ancestor {
                // Disconnected layers (e.g. one was removed from its
                // superview, or a CATransition snapshot layer is being
                // queried after it was detached) have no path between
                // them. Real Core Animation tolerates this and returns
                // the identity transform from the partial walk; mirror
                // that instead of panicking.
                log!(
                    "Warning: Layers {:?} and {:?} have no common ancestor; \
                     falling back to identity transform.",
                    this,
                    other
                );
                break (nil, this_transform, other_transform);
            } else {
                break (nil, this_transform, other_transform);
            }
        }
    };

    let _ = common_ancestor;
    other_transform.concat(this_transform.invert())
}

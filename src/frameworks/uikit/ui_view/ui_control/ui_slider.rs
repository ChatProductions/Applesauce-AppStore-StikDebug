/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISlider`.

use crate::frameworks::core_graphics::CGRect;
// Цепочка: UISlider -> UIControl -> UIView
use crate::frameworks::uikit::ui_view::ui_control::UIControlHostObject;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg_super, objc_classes,
    ClassExports, NSZonePtr,
};

#[derive(Default)]
pub(super) struct UISliderHostObject {
    pub(super) superclass: UIControlHostObject,
    pub(super) value: f32,
    pub(super) minimum_value: f32,
    pub(super) maximum_value: f32,
    pub(super) continuous: bool,
}

// Позволяет borrow() заглядывать в superclass
impl_HostObject_with_superclass!(UISliderHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISlider: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UISliderHostObject {
        superclass: UIControlHostObject::default(),
        value: 0.5,
        minimum_value: 0.0,
        maximum_value: 1.0,
        continuous: true, // По умолчанию в UIKit это свойство равно YES
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    msg_super![env; this initWithFrame:frame]
}

- (id)initWithCoder:(id)coder {
    msg_super![env; this initWithCoder:coder]
}

- (f32)value {
    env.objc.borrow::<UISliderHostObject>(this).value
}

- (())setValue:(f32)value {
    env.objc.borrow_mut::<UISliderHostObject>(this).value = value;
}

- (f32)minimumValue {
    env.objc.borrow::<UISliderHostObject>(this).minimum_value
}

- (())setMinimumValue:(f32)value {
    env.objc.borrow_mut::<UISliderHostObject>(this).minimum_value = value;
}

- (f32)maximumValue {
    env.objc.borrow::<UISliderHostObject>(this).maximum_value
}

- (())setMaximumValue:(f32)value {
    env.objc.borrow_mut::<UISliderHostObject>(this).maximum_value = value;
}

- (bool)isContinuous {
    env.objc.borrow::<UISliderHostObject>(this).continuous
}

- (())setContinuous:(bool)value {
    env.objc.borrow_mut::<UISliderHostObject>(this).continuous = value;
}

- (())setMinimumValueImage:(id)_img {
    // Stub
}

- (())setMaximumValueImage:(id)_img {
    // Stub
}

- (())setThumbImage:(id)_image forState:(u32)_state {
    // Stub: custom thumb images are not rendered
}

- (())setMinimumTrackImage:(id)_image forState:(u32)_state {
    // Stub: custom minimum track images are not rendered
}

- (())setMaximumTrackImage:(id)_image forState:(u32)_state {
    // Stub: custom maximum track images are not rendered
}

@end

};

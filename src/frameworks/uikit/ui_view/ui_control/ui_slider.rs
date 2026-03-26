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
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    log_dbg!("[(UISlider*){:?} initWithFrame:{:?}] TODO: Implement.",
        this, frame);
    msg_super![env; this initWithFrame:frame]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log_dbg!("[(UISlider*){:?} initWithCoder:{:?}] TODO: Implement.",
        this, coder);
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

- (())setMinimumValueImage:(id)_img { 
    // Stub
}

- (())setMaximumValueImage:(id)_img { 
    // Stub
}

@end

};

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISlider`.

use crate::frameworks::core_graphics::CGRect;
// \u0426\u0435\u043f\u043e\u0447\u043a\u0430: UISlider -> UIControl -> UIView
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

// \u041f\u043e\u0437\u0432\u043e\u043b\u044f\u0435\u0442 borrow() \u0437\u0430\u0433\u043b\u044f\u0434\u044b\u0432\u0430\u0442\u044c \u0432 superclass
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

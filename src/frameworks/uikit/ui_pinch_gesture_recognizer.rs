/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPinchGestureRecognizer`.

use crate::frameworks::core_graphics::CGFloat;
use crate::objc::{
    id, objc_classes, ClassExports, HostObject, NSZonePtr,
};

// MARK: - UIPinchGestureRecognizer host object

struct UIPinchGestureRecognizerHostObject {
    /// Масштаб жеста (начальное значение 1.0)
    scale: CGFloat,
    /// Скорость изменения (начальное значение 0.0)
    velocity: CGFloat,
}
impl HostObject for UIPinchGestureRecognizerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - UIPinchGestureRecognizer
// =========================================================================

@implementation UIPinchGestureRecognizer: UIGestureRecognizer

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIPinchGestureRecognizerHostObject {
        scale: 1.0,
        velocity: 0.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Свойства UIPinchGestureRecognizer

- (CGFloat)scale {
    env.objc.borrow::<UIPinchGestureRecognizerHostObject>(this).scale
}

- (())setScale:(CGFloat)scale {
    env.objc.borrow_mut::<UIPinchGestureRecognizerHostObject>(this).scale = scale;
}

- (CGFloat)velocity {
    env.objc.borrow::<UIPinchGestureRecognizerHostObject>(this).velocity
}

// Примечание: свойство velocity в iOS предназначено только для чтения (read-only),
// поэтому сеттер setVelocity: мы не реализуем, как это и работает в реальном Foundation.

@end

};

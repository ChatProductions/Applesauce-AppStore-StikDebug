/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Core Motion framework.

use crate::dyld::HostDylib;
use crate::objc::{id, msg_class, nil, objc_classes, ClassExports, HostObject, NSZonePtr};

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/CoreMotion.framework/CoreMotion",
    aliases: &[],
    class_exports: &[CLASSES],
    constant_exports: &[],
    function_exports: &[],
};

struct CMMotionManagerHostObject {
    accelerometer_update_interval: f64,
    gyro_update_interval: f64,
    device_motion_update_interval: f64,
    accelerometer_active: bool,
    gyro_active: bool,
    device_motion_active: bool,
}
impl HostObject for CMMotionManagerHostObject {}

struct CMAccelerometerDataHostObject {
    x: f64,
    y: f64,
    z: f64,
    timestamp: f64,
}
impl HostObject for CMAccelerometerDataHostObject {}

const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =============================================================================
// CMAccelerometerData — returned by [CMMotionManager accelerometerData]
// =============================================================================

@implementation CMAccelerometerData: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CMAccelerometerDataHostObject {
        x: 0.0,
        y: 0.0,
        z: -1.0, // Default: phone flat on table, gravity pointing down
        timestamp: 0.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (f64)timestamp {
    env.objc.borrow::<CMAccelerometerDataHostObject>(this).timestamp
}

@end

// =============================================================================
// CMMotionManager
// =============================================================================

@implementation CMMotionManager: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CMMotionManagerHostObject {
        accelerometer_update_interval: 0.01, // 100 Hz default
        gyro_update_interval: 0.01,
        device_motion_update_interval: 0.01,
        accelerometer_active: false,
        gyro_active: false,
        device_motion_active: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (bool)isGyroAvailable {
    false
}
- (bool)isDeviceMotionAvailable {
    false
}
- (bool)isAccelerometerAvailable {
    true
}
- (bool)isMagnetometerAvailable {
    false
}

- (())setAccelerometerUpdateInterval:(f64)interval {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).accelerometer_update_interval = interval;
}

- (f64)accelerometerUpdateInterval {
    env.objc.borrow::<CMMotionManagerHostObject>(this).accelerometer_update_interval
}

- (())startAccelerometerUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).accelerometer_active = true;
}

- (())stopAccelerometerUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).accelerometer_active = false;
}

- (())setGyroUpdateInterval:(f64)interval {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).gyro_update_interval = interval;
}

- (f64)gyroUpdateInterval {
    env.objc.borrow::<CMMotionManagerHostObject>(this).gyro_update_interval
}

- (())startGyroUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).gyro_active = true;
}

- (())stopGyroUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).gyro_active = false;
}

- (())setDeviceMotionUpdateInterval:(f64)interval {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).device_motion_update_interval = interval;
}

- (f64)deviceMotionUpdateInterval {
    env.objc.borrow::<CMMotionManagerHostObject>(this).device_motion_update_interval
}

- (())startDeviceMotionUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).device_motion_active = true;
}

- (())stopDeviceMotionUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).device_motion_active = false;
}

- (bool)isDeviceMotionActive {
    env.objc.borrow::<CMMotionManagerHostObject>(this).device_motion_active
}

- (bool)isAccelerometerActive {
    env.objc.borrow::<CMMotionManagerHostObject>(this).accelerometer_active
}

- (bool)isGyroActive {
    env.objc.borrow::<CMMotionManagerHostObject>(this).gyro_active
}

- (id)deviceMotion {
    nil
}

- (id)accelerometerData {
    let active = env.objc.borrow::<CMMotionManagerHostObject>(this).accelerometer_active;
    if !active {
        return nil;
    }
    // Return a fresh CMAccelerometerData with default gravity values.
    // Real accelerometer data (if available from SDL sensor) would be
    // integrated here via the window's accelerometer reading.
    let data: id = msg_class![env; CMAccelerometerData new];
    data
}

- (id)gyroData {
    nil
}

- (())startAccelerometerUpdatesToQueue:(id)_queue withHandler:(id)_handler {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).accelerometer_active = true;
    log_dbg!("CMMotionManager startAccelerometerUpdatesToQueue:withHandler: — handler will not be called (stub)");
}

- (())startGyroUpdatesToQueue:(id)_queue withHandler:(id)_handler {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).gyro_active = true;
    log_dbg!("CMMotionManager startGyroUpdatesToQueue:withHandler: — handler will not be called (stub)");
}

- (())startDeviceMotionUpdatesToQueue:(id)_queue withHandler:(id)_handler {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).device_motion_active = true;
    log_dbg!("CMMotionManager startDeviceMotionUpdatesToQueue:withHandler: — handler will not be called (stub)");
}

@end

};

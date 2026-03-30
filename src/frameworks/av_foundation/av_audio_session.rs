/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AVAudioSession`

use crate::mem::MutPtr;
use crate::objc::{id, nil, ClassExports, HostObject, NSZonePtr};
use crate::objc_classes;
use crate::Environment;

struct AVAudioSessionHostObject;
impl HostObject for AVAudioSessionHostObject {}

/// Allocate a bare AVAudioSession host object.
/// Called from +allocWithZone: and +sharedInstance.
/// Each call returns a fresh allocation; pointer identity is not
/// guaranteed across sharedInstance calls (acceptable for stubs).
fn new_av_audio_session(env: &mut Environment, class: id) -> id {
    let host_object = Box::new(AVAudioSessionHostObject);
    env.objc.alloc_object(class, host_object, &mut env.mem)
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation AVAudioSession: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    new_av_audio_session(env, this)
}

+ (id)sharedInstance {
    log!("TODO: AVAudioSession sharedInstance");
    new_av_audio_session(env, this)
}

- (bool)setCategory:(id)_category
              error:(MutPtr<id>)_error {
    log!("TODO: AVAudioSession setCategory:error:");
    true
}

- (bool)setCategory:(id)_category
       withOptions:(i32)_options
              error:(MutPtr<id>)_error {
    log!("TODO: AVAudioSession setCategory:withOptions:error:");
    true
}

- (bool)setActive:(bool)_active
            error:(MutPtr<id>)_error {
    log!("TODO: AVAudioSession setActive:error:");
    true
}

- (bool)setActive:(bool)_active
      withOptions:(i32)_options
            error:(MutPtr<id>)_error {
    log!("TODO: AVAudioSession setActive:withOptions:error:");
    true
}

- (id)category {
    nil
}

- (())setDelegate:(id)_delegate {
    log!("TODO: AVAudioSession setDelegate:");
}

- (f64)currentHardwareSampleRate {
    44100.0
}

- (u32)currentHardwareOutputNumberChannels {
    2
}

- (bool)inputIsAvailable {
    false
}

- (f32)outputVolume {
    1.0
}

@end

};

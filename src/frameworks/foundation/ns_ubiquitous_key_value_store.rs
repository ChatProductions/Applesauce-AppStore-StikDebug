/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 */
//! `NSUbiquitousKeyValueStore`

use crate::objc::{id, msg, msg_class, objc_classes, ClassExports};
use crate::Environment;

#[derive(Default)]
pub struct State {
    pub default_store: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSUbiquitousKeyValueStore: NSObject

+ (id)defaultStore {
    // Паттерн синглтона по аналогии с NSFileManager
    if let Some(existing) = env.framework_state.foundation.ns_ubiquitous_key_value_store.default_store {
        existing
    } else {
        let new: id = msg![env; this new];
        env.framework_state.foundation.ns_ubiquitous_key_value_store.default_store = Some(new);
        new
    }
}

- (bool)synchronize {
    // Перенаправляем синхронизацию в локальный NSUserDefaults
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults synchronize]
}

- (id)dictionaryRepresentation {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults dictionaryRepresentation]
}

- (id)objectForKey:(id)key {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults objectForKey:key]
}

- (())setObject:(id)obj forKey:(id)key {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults setObject:obj forKey:key]
}

- (bool)boolForKey:(id)key {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults boolForKey:key]
}

- (())setBool:(bool)value forKey:(id)key {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults setBool:value forKey:key]
}

- (id)stringForKey:(id)key {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults stringForKey:key]
}

- (())setString:(id)value forKey:(id)key {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults setObject:value forKey:key]
}

// Cut the Rope и другие игры часто используют 64-битные числа для очков
- (i64)longLongForKey:(id)key {
    let obj: id = msg![env; this objectForKey:key];
    if obj != crate::objc::nil {
        msg![env; obj longLongValue]
    } else {
        0
    }
}

- (())setLongLong:(i64)value forKey:(id)key {
    let num: id = msg_class![env; NSNumber numberWithLongLong:value];
    msg![env; this setObject:num forKey:key]
}

@end

};

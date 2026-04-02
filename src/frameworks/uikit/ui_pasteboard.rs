/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPasteboard`.

use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// MARK: - UIPasteboard host object

struct UIPasteboardHostObject {
    /// Имя буфера обмена (NSString*)
    name: id,
    /// Содержимое буфера в виде строки (NSString*)
    string: id,
}

impl HostObject for UIPasteboardHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - UIPasteboard
// =========================================================================

@implementation UIPasteboard: NSObject

// =========================================================================
// МЕТОДЫ КЛАССА (+)
// =========================================================================

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIPasteboardHostObject {
        name: nil,
        string: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)generalPasteboard {
    // В большинстве игр используется для быстрого копирования текста. 
    // Создаем базовый инстанс.
    let instance: id = msg_class![env; UIPasteboard alloc];
    msg![env; instance init]
}

+ (id)pasteboardWithName:(id)name create:(bool)_create {
    let instance: id = msg_class![env; UIPasteboard alloc];
    let instance: id = msg![env; instance init];

    // 1. Сначала делаем retain, пока env свободен
    retain(env, name);

    // 2. Затем заимствуем и сразу записываем (без сохранения переменной host)
    env.objc.borrow_mut::<UIPasteboardHostObject>(instance).name = name;

    instance
}

// =========================================================================
// МЕТОДЫ ЭКЗЕМПЛЯРА (-)
// =========================================================================

- (id)init {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<UIPasteboardHostObject>(this);
    let name = host.name;
    let string = host.string;
    
    release(env, name);
    release(env, string);
    
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)name { // NSString*
    env.objc.borrow::<UIPasteboardHostObject>(this).name
}

- (id)string { // NSString*
    env.objc.borrow::<UIPasteboardHostObject>(this).string
}

- (())setString:(id)string { // NSString*
    // Достаем старое значение, чтобы освободить его
    let old = env.objc.borrow::<UIPasteboardHostObject>(this).string;
    
    // Выполняем операции управления памятью
    release(env, old);
    retain(env, string);
    
    // Записываем новое значение
    env.objc.borrow_mut::<UIPasteboardHostObject>(this).string = string;
}

@end

};

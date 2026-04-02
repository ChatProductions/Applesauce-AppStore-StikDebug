/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::mem::MutPtr;
use crate::objc::{
    id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr, SEL,
};
use crate::Environment;

#[derive(Debug, Default)]
struct NSInvocationOperationHostObject {
    target: id,
    selector: Option<SEL>, // Используем Option, так как у SEL нет Default
    arg: id,
}
impl HostObject for NSInvocationOperationHostObject {}

#[derive(Debug, Default)]
struct NSOperationQueueHostObject {}
impl HostObject for NSOperationQueueHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSOperation: NSObject

- (id)init {
    this
}

- (())start {
    // В HLE реализации мы просто вызываем main() сразу
    () = msg![env; this main];
}

- (())main {
    // Базовая реализация ничего не делает
}

- (())cancel {
}

@end

@implementation NSInvocationOperation: NSOperation

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSInvocationOperationHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithTarget:(id)target selector:(SEL)sel object:(id)arg {
    let this: id = msg![env; this init];
    if this != nil {
        retain(env, target);
        retain(env, arg);
        
        let host_object = env.objc.borrow_mut::<NSInvocationOperationHostObject>(this);
        host_object.target = target;
        host_object.selector = Some(sel);
        host_object.arg = arg;
    }
    this
}

- (())dealloc {
    // Используем временный блок { }, чтобы заимствование host_object завершилось
    // до того, как мы вызовем release(env, ...), который требует монопольный доступ к env.
    let (target, arg) = {
        let host_object = env.objc.borrow::<NSInvocationOperationHostObject>(this);
        (host_object.target, host_object.arg)
    };
    
    release(env, target);
    release(env, arg);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (())main {
    // Извлекаем данные во временные переменные, чтобы освободить env для макроса msg!
    let (target, sel_opt, arg) = {
        let host_object = env.objc.borrow::<NSInvocationOperationHostObject>(this);
        (host_object.target, host_object.selector, host_object.arg)
    };

    if target != nil {
        if let Some(sel) = sel_opt {
            if arg != nil {
                let _: id = msg![env; target performSelector:sel withObject:arg];
            } else {
                let _: id = msg![env; target performSelector:sel];
            }
        }
    }
}

@end

@implementation NSOperationQueue: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSOperationQueueHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (())addOperation:(id)op { 
    retain(env, op);
    // Для простоты выполняем операцию синхронно
    () = msg![env; op start];
    release(env, op);
}

- (())setMaxConcurrentOperationCount:(i32)_count {
    // Игнорируем в данной реализации
}

@end

};

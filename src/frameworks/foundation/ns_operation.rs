/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr, SEL,
};

#[derive(Debug, Default)]
struct NSOperationHostObject {
    dependencies: id,
    // Поля для дочернего NSInvocationOperation хранятся здесь же
    target: id,
    selector: Option<SEL>,
    arg: id,
}
impl HostObject for NSOperationHostObject {}

#[derive(Debug, Default)]
struct NSOperationQueueHostObject {}
impl HostObject for NSOperationQueueHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSOperation: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSOperationHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (())dealloc {
    // Извлекаем все свойства во временные переменные, чтобы не конфликтовать с borrow checker
    let (deps, target, arg) = {
        let host_object = env.objc.borrow::<NSOperationHostObject>(this);
        (host_object.dependencies, host_object.target, host_object.arg)
    };
    
    release(env, deps);
    release(env, target);
    release(env, arg);
    env.objc.dealloc_object(this, &mut env.mem)
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

- (())addDependency:(id)op {
    if op == nil {
        return;
    }
    let deps = env.objc.borrow::<NSOperationHostObject>(this).dependencies;
    
    let deps_arr = if deps == nil {
        let new_arr: id = msg_class![env; NSMutableArray alloc];
        let new_arr: id = msg![env; new_arr init];
        env.objc.borrow_mut::<NSOperationHostObject>(this).dependencies = new_arr;
        new_arr
    } else {
        deps
    };
    
    let _: () = msg![env; deps_arr addObject:op];
}

- (())removeDependency:(id)op {
    if op == nil {
        return;
    }
    let deps = env.objc.borrow::<NSOperationHostObject>(this).dependencies;
    if deps != nil {
        let _: () = msg![env; deps removeObject:op];
    }
}

- (id)dependencies {
    let deps = env.objc.borrow::<NSOperationHostObject>(this).dependencies;
    if deps == nil {
        msg_class![env; NSArray array]
    } else {
        let copy: id = msg![env; deps copy];
        autorelease(env, copy)
    }
}

@end

@implementation NSInvocationOperation: NSOperation

// allocWithZone: и dealloc теперь наследуются от NSOperation

- (id)initWithTarget:(id)target selector:(SEL)sel object:(id)arg {
    let this: id = msg![env; this init];
    if this != nil {
        retain(env, target);
        retain(env, arg);
        
        let host_object = env.objc.borrow_mut::<NSOperationHostObject>(this);
        host_object.target = target;
        host_object.selector = Some(sel);
        host_object.arg = arg;
    }
    this
}

- (())main {
    let (target, sel_opt, arg) = {
        let host_object = env.objc.borrow::<NSOperationHostObject>(this);
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

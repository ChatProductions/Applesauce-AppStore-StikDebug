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
    selector: SEL,
    arg: id,
}
impl HostObject for NSInvocationOperationHostObject {}

#[derive(Debug, Default)]
struct NSOperationQueueHostObject {
    // В полноценном многопоточном режиме здесь бы хранилась очередь (Vec<id>).
    // Но для HLE синхронного выполнения состояние можно оставить пустым.
}
impl HostObject for NSOperationQueueHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// Базовый класс операции
@implementation NSOperation: NSObject

- (id)init {
    this
}

- (())start {
    // По дефолту start вызывает main
    () = msg![env; this main];
}

- (())main {
    // Должен переопределяться сабклассами
}

- (())cancel {
    // Заглушка, так как мы выполняем операции сразу при добавлении
}

@end

// Наш недостающий класс
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
        host_object.selector = sel;
        host_object.arg = arg;
    }
    this
}

- (())dealloc {
    let host_object = env.objc.borrow::<NSInvocationOperationHostObject>(this);
    release(env, host_object.target);
    release(env, host_object.arg);
    env.objc.dealloc_object(this, &mut env.mem)
}

// Фактическое выполнение селектора с переданными аргументами
- (())main {
    let host_object = env.objc.borrow::<NSInvocationOperationHostObject>(this);
    let target = host_object.target;
    let sel = host_object.selector;
    let arg = host_object.arg;

    if target != nil {
        if arg != nil {
            let _: id = msg![env; target performSelector:sel withObject:arg];
        } else {
            let _: id = msg![env; target performSelector:sel];
        }
    }
}

@end

// Класс очереди, через который игра будет пытаться эти операции запускать
@implementation NSOperationQueue: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSOperationQueueHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (())addOperation:(id)op { // NSOperation*
    retain(env, op);
    
    // В оригинале операции уходят в асинхронный пул потоков.
    // В HLE (без сложного шедулера) мы просто "пробиваем" операцию синхронно.
    // Это безопасно для подавляющего большинства старых игр.
    () = msg![env; op start];
    
    release(env, op);
}

- (())setMaxConcurrentOperationCount:(i32)_count {
    // Метод настройки пула. При нашей синхронной реализации он ни на что не влияет.
}

@end

};
  

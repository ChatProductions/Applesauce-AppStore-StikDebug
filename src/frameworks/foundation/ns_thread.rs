/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSThread`.

use super::NSTimeInterval;
use crate::dyld::HostFunction;
use crate::frameworks::core_foundation::CFTypeRef;
use crate::frameworks::foundation::NSUInteger;
use crate::libc::pthread::thread::{
    pthread_attr_init, pthread_attr_setdetachstate, pthread_attr_setstacksize, pthread_attr_t,
    pthread_create, pthread_self, pthread_t, PTHREAD_CREATE_DETACHED,
};
use crate::mem::{guest_size_of, Mem, MutPtr};
use crate::objc::{
    id, msg_send, msg_send_no_type_checking, nil, objc_classes, release, retain, todo_objc_setter,
    Class, ClassExports, HostObject, NSZonePtr, SEL,
};
use crate::Environment;
use crate::{msg, msg_class};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Default)]
pub struct State {
    is_multi_threaded: bool,
    ns_threads: HashMap<pthread_t, id>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.foundation.ns_thread
    }
}

struct NSThreadHostObject {
    target: id,
    selector: Option<SEL>,
    object: id,
    /// `NSMutableDictionary*`
    thread_dictionary: id,
    owned: bool,
    finished: bool,
    executing: bool, // Добавлено для отслеживания состояния
    stack_size: NSUInteger,
    tolerate_type_mismatch: bool,
}
impl HostObject for NSThreadHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSThread: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSThreadHostObject {
        target: nil,
        selector: None,
        object: nil,
        thread_dictionary: nil,
        owned: false,
        finished: false,
        executing: false, // Инициализация
        stack_size: Mem::SECONDARY_THREAD_DEFAULT_STACK_SIZE,
        tolerate_type_mismatch: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (bool)isMultiThreaded {
    env.framework_state.foundation.ns_thread.is_multi_threaded
}

+ (f64)threadPriority {
    let thread: id = msg![env; this currentThread];
    msg![env; thread threadPriority]
}
+ (bool)setThreadPriority:(f64)priority {
    let thread: id = msg![env; this currentThread];
    msg![env; thread setThreadPriority:priority]
}

+ (id)currentThread {
    let pthread = pthread_self(env);
    #[allow(clippy::map_entry)]
    if !State::get(env).ns_threads.contains_key(&pthread) {
        let ns_thread: id = msg_class![env; NSThread alloc];
        let ns_thread: id = msg![env; ns_thread init];
        State::get(env).ns_threads.insert(pthread, ns_thread);
    }
    *State::get(env).ns_threads.get(&pthread).unwrap()
}

+ (id)callStackReturnAddresses {
    log!("WARNING: [NSThread callStackReturnAddresses] is called, returning an empty array!");
    msg_class![env; NSArray new]
}

+ (())sleepForTimeInterval:(NSTimeInterval)ti {
    log_dbg!("[NSThread sleepForTimeInterval:{:?}]", ti);
    env.sleep(Duration::from_secs_f64(ti));
}

+ (())detachNewThreadSelector:(SEL)selector
                     toTarget:(id)target
                   withObject:(id)object {
    detach_new_thread_inner(env, selector, target, object, /* tolerate_type_mismatch: */ false)
}

- (id)initWithTarget:(id)target
            selector:(SEL)selector
              object:(id)object {
    env.objc.borrow_mut::<NSThreadHostObject>(this).target = target;
    retain(env, target);
    env.objc.borrow_mut::<NSThreadHostObject>(this).selector = Some(selector);
    env.objc.borrow_mut::<NSThreadHostObject>(this).object = object;
    retain(env, object);

    this
}

- (())start {
    let symb = "__touchHLE_NSThreadInvocationHelper";
    let hf: HostFunction = &(_touchHLE_NSThreadInvocationHelper as fn(&mut Environment, _) -> _);
    let gf = env
        .dyld
        .create_guest_function(&mut env.mem, symb, hf);

    let attr: MutPtr<pthread_attr_t> = env.mem.alloc(guest_size_of::<pthread_attr_t>()).cast();
    pthread_attr_init(env, attr);

    let stack_size = env.objc.borrow::<NSThreadHostObject>(this).stack_size;
    pthread_attr_setstacksize(env, attr, stack_size);
    pthread_attr_setdetachstate(env, attr, PTHREAD_CREATE_DETACHED);
    let thread_ptr: MutPtr<pthread_t> = env.mem.alloc(guest_size_of::<pthread_t>()).cast();

    pthread_create(env, thread_ptr, attr.cast_const(), gf, this.cast());

    let pthread = env.mem.read(thread_ptr);
    assert!(!State::get(env).ns_threads.contains_key(&pthread));
    State::get(env).ns_threads.insert(pthread, this);

    env.framework_state.foundation.ns_thread.is_multi_threaded = true;
}

- (())main {
    let &NSThreadHostObject {
        target,
        selector,
        object,
        tolerate_type_mismatch,
        ..
    } = env.objc.borrow(this);
    if tolerate_type_mismatch {
        () = msg_send_no_type_checking(env, (target, selector.unwrap(), object));
    } else {
        () = msg_send(env, (target, selector.unwrap(), object));
    }
}

- (id)threadDictionary {
    let thread_dictionary = env.objc.borrow::<NSThreadHostObject>(this).thread_dictionary;
    if thread_dictionary == nil {
        let thread_dictionary = msg_class![env; NSMutableDictionary new];
        env.objc.borrow_mut::<NSThreadHostObject>(this).thread_dictionary = thread_dictionary;
        thread_dictionary
    } else {
        thread_dictionary
    }
}

- (f64)threadPriority {
    log!("TODO: [(NSThread *){:?} threadPriority] (not implemented yet)", this);
    1.0
}
- (bool)setThreadPriority:(f64)priority {
    todo_objc_setter!(this, priority);
    true
}

- (NSUInteger)stackSize {
    env.objc.borrow::<NSThreadHostObject>(this).stack_size
}
- (())setStackSize:(NSUInteger)size {
    env.objc.borrow_mut::<NSThreadHostObject>(this).stack_size = size;
}

- (bool)isFinished {
    env.objc.borrow::<NSThreadHostObject>(this).finished
}

- (bool)isExecuting {
    env.objc.borrow::<NSThreadHostObject>(this).executing
}

- (bool)isCancelled {
    log!("TODO: [(NSThread *){:?} isCancelled]", this);
    false
}

- (())dealloc {
    log_dbg!("[(NSThread*){:?} dealloc]", this);
    let host_object = env.objc.borrow::<NSThreadHostObject>(this);
    release(env, host_object.thread_dictionary);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

type NSThreadRef = CFTypeRef;

pub fn _touchHLE_NSThreadInvocationHelper(env: &mut Environment, ns_thread_obj: NSThreadRef) {
    let class: Class = msg![env; ns_thread_obj class];
    let thread_class = env.objc.get_known_class("NSThread", &mut env.mem);
        if !env.objc.class_is_subclass_of(class, thread_class) {
        log!("Warning: _touchHLE_NSThreadInvocationHelper called with unexpected class, skipping");
        return;
    }

    // Устанавливаем статус выполнения
    env.objc.borrow_mut::<NSThreadHostObject>(ns_thread_obj).executing = true;

    () = msg![env; ns_thread_obj main];

    let host_obj = env.objc.borrow_mut::<NSThreadHostObject>(ns_thread_obj);
    host_obj.finished = true;
    host_obj.executing = false; // Поток завершен

    let &NSThreadHostObject {
        target,
        object,
        owned,
        ..
    } = env.objc.borrow(ns_thread_obj);
    
    release(env, object);
    release(env, target);

    let pthread = pthread_self(env);
    let res = State::get(env).ns_threads.remove(&pthread);
    if res.is_none() {
        log!("Warning: _touchHLE_NSThreadInvocationHelper: pthread {:?} not found in ns_threads map", pthread);
    }

    if owned {
        release(env, ns_thread_obj);
    }
}

pub fn detach_new_thread_inner(
    env: &mut Environment,
    selector: SEL,
    target: id,
    object: id,
    tolerate_type_mismatch: bool,
) {
    let new: id = msg_class![env; NSThread alloc];
    let new: id = msg![env; new initWithTarget:target
                                      selector:selector
                                        object:object];

    env.objc.borrow_mut::<NSThreadHostObject>(new).owned = true;
    env.objc.borrow_mut::<NSThreadHostObject>(new).tolerate_type_mismatch = tolerate_type_mismatch;
    env.framework_state.foundation.ns_thread.is_multi_threaded = true;

    msg![env; new start]
}


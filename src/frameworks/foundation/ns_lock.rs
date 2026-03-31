/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSLock family`.
//!
//! TODO: There is probably an opportunity to refactor common methods.
//! (Need to find a good way to do so! Common super class wouldn't
//! work as it breaks expected inheritance chain.)

use crate::environment::MutexType::PTHREAD_MUTEX_RECURSIVE;
use crate::environment::{MutexId, PTHREAD_MUTEX_DEFAULT};
use crate::msg;
use crate::objc::{id, nil, objc_classes, ClassExports, HostObject};
use std::sync::{Arc, Condvar, Mutex};

// NSInteger is i32 on 32-bit ARM.
type NSInteger = i32;

struct NSLockHostObject {
    mutex_id: MutexId,
    name: id,
}
impl HostObject for NSLockHostObject {}

// ---------------------------------------------------------------------------
// NSConditionLock
// ---------------------------------------------------------------------------

struct NSConditionLockState {
    locked: bool,
    condition: NSInteger,
}

struct NSConditionLockHostObject {
    /// Arc lets lock methods clone the reference, drop the ObjC borrow,
    /// and then block on the condvar without holding a borrow on env.objc.
    inner: Arc<(Mutex<NSConditionLockState>, Condvar)>,
}
impl HostObject for NSConditionLockHostObject {}

// ---------------------------------------------------------------------------

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSLock: NSObject

+ (id)alloc {
    log_dbg!("[NSLock alloc]");
    let mutex_id = env.mutex_state.init_mutex(PTHREAD_MUTEX_DEFAULT);
    let host_object = NSLockHostObject { mutex_id, name: nil };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

// NSLocking protocol implementation
- (())lock {
    log_dbg!("[(NSLock *){:?} lock]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    env.lock_mutex(host_object.mutex_id).unwrap();
}
- (())unlock {
    log_dbg!("[(NSLock *){:?} unlock]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    if !env.mutex_state.mutex_is_locked(host_object.mutex_id) {
        echo!("*** -[NSLock unlock]: lock (<NSLock: {:?}> '{:?}') unlocked when not locked", this, host_object.name);
    }
    env.unlock_mutex(host_object.mutex_id).unwrap();
}

- (bool)tryLock {
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    if env.mutex_state.mutex_is_locked(host_object.mutex_id) {
        false
    } else {
        env.lock_mutex(host_object.mutex_id).is_ok()
    }
}

- (())setName:(id)name { // NSString *
    // @property(copy), name has to be copied
    env.objc.borrow_mut::<NSLockHostObject>(this).name = msg![env; name copy];
}
- (id)name {
    env.objc.borrow::<NSLockHostObject>(this).name
}

- (())dealloc {
    log_dbg!("[(NSLock *){:?} dealloc]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    env.mutex_state.destroy_mutex(host_object.mutex_id).unwrap();
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

@implementation NSRecursiveLock: NSObject

+ (id)alloc {
    log_dbg!("[NSRecursiveLock alloc]");
    let mutex_id = env.mutex_state.init_mutex(PTHREAD_MUTEX_RECURSIVE);
    let host_object = NSLockHostObject { mutex_id, name: nil };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

// NSLocking protocol implementation
- (())lock {
    log_dbg!("[(NSRecursiveLock *){:?} lock]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    env.lock_mutex(host_object.mutex_id).unwrap();
}
- (())unlock {
    log_dbg!("[(NSRecursiveLock *){:?} unlock]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    if !env.mutex_state.mutex_is_locked(host_object.mutex_id) {
        echo!("*** -[NSRecursiveLock unlock]: lock (<NSRecursiveLock: {:?}> '{:?}') unlocked when not locked", this, host_object.name);
    }
    env.unlock_mutex(host_object.mutex_id).unwrap();
}

- (bool)tryLock {
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    if env.mutex_state.mutex_is_locked(host_object.mutex_id) {
        false
    } else {
        env.lock_mutex(host_object.mutex_id).is_ok()
    }
}

- (())setName:(id)name { // NSString *
    // @property(copy), name has to be copied
    env.objc.borrow_mut::<NSLockHostObject>(this).name = msg![env; name copy];
}
- (id)name {
    env.objc.borrow::<NSLockHostObject>(this).name
}

- (())dealloc {
    log_dbg!("[(NSRecursiveLock *){:?} dealloc]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    env.mutex_state.destroy_mutex(host_object.mutex_id).unwrap();
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// ---------------------------------------------------------------------------
// NSConditionLock
// ---------------------------------------------------------------------------

@implementation NSConditionLock: NSObject

+ (id)alloc {
    log_dbg!("[NSConditionLock alloc]");
    let inner = Arc::new((
        Mutex::new(NSConditionLockState {
            locked: false,
            condition: 0,
        }),
        Condvar::new(),
    ));
    let host_obj = NSConditionLockHostObject { inner };
    env.objc.alloc_object(this, Box::new(host_obj), &mut env.mem)
}

- (id)init {
    this
}

- (id)initWithCondition:(NSInteger)condition {
    let inner = env.objc
        .borrow::<NSConditionLockHostObject>(this)
        .inner
        .clone();
    inner.0.lock().unwrap().condition = condition;
    this
}

- (NSInteger)condition {
    let inner = env.objc
        .borrow::<NSConditionLockHostObject>(this)
        .inner
        .clone();
    // Фикс: сохраняем в переменную, чтобы замок (guard) освободился ДО того,
    // как переменная 'inner' выйдет из области видимости.
    let current_cond = inner.0.lock().unwrap().condition;
    current_cond
}

// Acquires the lock unconditionally, blocking until available.
- (())lock {
    log_dbg!("[(NSConditionLock *){:?} lock]", this);
    let inner = env.objc
        .borrow::<NSConditionLockHostObject>(this)
        .inner
        .clone();
    let (mutex, condvar) = &*inner;
    let mut state = condvar
        .wait_while(mutex.lock().unwrap(), |s| s.locked)
        .unwrap();
    state.locked = true;
}

// Acquires the lock without blocking. Returns true on success.
- (bool)tryLock {
    let inner = env.objc
        .borrow::<NSConditionLockHostObject>(this)
        .inner
        .clone();
    let mut state = inner.0.lock().unwrap();
    if state.locked {
        false
    } else {
        state.locked = true;
        true
    }
}

// Blocks until the lock is free AND condition matches, then acquires.
- (())lockWhenCondition:(NSInteger)condition {
    log_dbg!(
        "[(NSConditionLock *){:?} lockWhenCondition:{}]",
        this,
        condition
    );
    let inner = env.objc
        .borrow::<NSConditionLockHostObject>(this)
        .inner
        .clone();
    let (mutex, condvar) = &*inner;
    let mut state = condvar
        .wait_while(mutex.lock().unwrap(), |s| {
            s.locked || s.condition != condition
        })
        .unwrap();
    state.locked = true;
}

// Non-blocking conditional acquire. Returns true on success.
- (bool)tryLockWhenCondition:(NSInteger)condition {
    let inner = env.objc
        .borrow::<NSConditionLockHostObject>(this)
        .inner
        .clone();
    let mut state = inner.0.lock().unwrap();
    if !state.locked && state.condition == condition {
        state.locked = true;
        true
    } else {
        false
    }
}

// Releases the lock without changing the condition.
- (())unlock {
    log_dbg!("[(NSConditionLock *){:?} unlock]", this);
    let inner = env.objc
        .borrow::<NSConditionLockHostObject>(this)
        .inner
        .clone();
    let (mutex, condvar) = &*inner;
    mutex.lock().unwrap().locked = false;
    condvar.notify_all();
}

// Releases the lock and sets the condition to a new value.
- (())unlockWithCondition:(NSInteger)condition {
    log_dbg!(
        "[(NSConditionLock *){:?} unlockWithCondition:{}]",
        this,
        condition
    );
    let inner = env.objc
        .borrow::<NSConditionLockHostObject>(this)
        .inner
        .clone();
    let (mutex, condvar) = &*inner;
    {
        let mut state = mutex.lock().unwrap();
        state.locked = false;
        state.condition = condition;
    }
    condvar.notify_all();
}

// Timed variants: NSDate* limit is ignored; we wait unconditionally
// and always return true. Sufficient for most game use-cases.
- (bool)lockBeforeDate:(id)_limit {
    let inner = env.objc
        .borrow::<NSConditionLockHostObject>(this)
        .inner
        .clone();
    let (mutex, condvar) = &*inner;
    // Фикс: используем блок, чтобы MutexGuard удалился до того, как 'inner' выйдет из области видимости.
    {
        let mut state = condvar
            .wait_while(mutex.lock().unwrap(), |s| s.locked)
            .unwrap();
        state.locked = true;
    }
    true
}

- (bool)lockWhenCondition:(NSInteger)condition
         beforeDate:(id)_limit {
    let inner = env.objc
        .borrow::<NSConditionLockHostObject>(this)
        .inner
        .clone();
    let (mutex, condvar) = &*inner;
    // Фикс: используем блок для своевременного освобождения замка.
    {
        let mut state = condvar
            .wait_while(mutex.lock().unwrap(), |s| {
                s.locked || s.condition != condition
            })
            .unwrap();
        state.locked = true;
    }
    true
}

- (())dealloc {
    log_dbg!("[(NSConditionLock *){:?} dealloc]", this);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

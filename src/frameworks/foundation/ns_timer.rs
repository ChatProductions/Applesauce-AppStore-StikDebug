/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSTimer`.

use super::ns_run_loop::NSDefaultRunLoopMode;
use super::NSTimeInterval;
use super::{ns_run_loop, ns_string};
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, nil, objc_classes, release, retain, ClassExports,
    HostObject, SEL, NSZonePtr,
};
use crate::Environment;
use std::time::{Duration, Instant};

struct NSTimerHostObject {
    ns_interval: NSTimeInterval,
    /// Copy of `ns_interval` in Rust's type for time intervals. Keep in sync!
    rust_interval: Duration,
    /// Strong reference
    target: id,
    selector: SEL,
    /// Strong reference
    user_info: id,
    repeats: bool,
    due_by: Option<Instant>,
    /// If the timer is currently running its callback, this is set so that the
    /// re-entering the run loop from inside the callback doesn't cause an
    /// infinite loop.
    is_running_callback: bool,
    /// Weak reference
    run_loop: id,
}
impl HostObject for NSTimerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSTimer doesn't seem to be an abstract class?
@implementation NSTimer: NSObject

+ (id)timerWithTimeInterval:(NSTimeInterval)ns_interval
                     target:(id)target
                   selector:(SEL)selector
                   userInfo:(id)user_info
                    repeats:(bool)repeats {
    let ns_interval = ns_interval.max(0.0001);
    let rust_interval = Duration::from_secs_f64(ns_interval);

    retain(env, target);
    retain(env, user_info);

    let host_object = Box::new(NSTimerHostObject {
        ns_interval,
        rust_interval,
        target,
        selector,
        user_info,
        repeats,
        due_by: Some(Instant::now().checked_add(rust_interval).unwrap()),
        run_loop: nil,
        is_running_callback: false
    });
    let new = env.objc.alloc_object(this, host_object, &mut env.mem);

    log_dbg!(
        "New {} timer {:?}, interval {}s, target [{:?} {}], user info {:?}",
        if repeats { "repeating" } else { "single-use" },
        new,
        ns_interval,
        target,
        selector.as_str(&env.mem),
        user_info,
    );
    autorelease(env, new)
}

+ (id)allocWithZone:(NSZonePtr)_zone {
    // Безопасно зануляем структуру, если Default вдруг не реализован вручную
    let host_object = Box::new(unsafe { std::mem::zeroed::<NSTimerHostObject>() });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}
    
+ (id)scheduledTimerWithTimeInterval:(f64)ti target:(id)t selector:(SEL)s userInfo:(id)ui repeats:(bool)rep {
    let timer: id = msg_class![env; NSTimer alloc];
    let timer: id = msg![env; timer initWithFireDate:nil interval:ti target:t selector:s userInfo:ui repeats:rep];
    
    let run_loop: id = msg_class![env; NSRunLoop currentRunLoop];
    
    // Получаем реальный гостевой NSString из системного пула эмулятора
    let mode_str = crate::frameworks::foundation::ns_string::get_static_str(env, "NSDefaultRunLoopMode");
    let _: () = msg![env; run_loop addTimer:timer forMode:mode_str];
    
    // ИСПРАВЛЕНИЕ: Обязательно возвращаем объект в пуле autorelease, иначе будет утечка или краш
    autorelease(env, timer)
}

- (())dealloc {
    let _: () = msg![env; this invalidate];
    
    let host = env.objc.borrow::<NSTimerHostObject>(this);
    let (target, user_info) = (host.target, host.user_info);
    release(env, target);
    release(env, user_info);
    env.objc.dealloc_object(this, &mut env.mem)
}
    
- (NSTimeInterval)timeInterval {
    let host_object = env.objc.borrow::<NSTimerHostObject>(this);
    if host_object.repeats {
        host_object.ns_interval
    } else {
        0.0 // this is the documented behaviour!
    }
}

- (id)userInfo {
    env.objc.borrow::<NSTimerHostObject>(this).user_info
}

- (bool)isValid {
    env.objc.borrow::<NSTimerHostObject>(this).due_by.is_some()
}

- (())invalidate {
    // Использован блок {}, чтобы ограничить жизнь переменной host
    let run_loop_to_remove = {
        let mut host = env.objc.borrow_mut::<NSTimerHostObject>(this);
        host.due_by = None;
        let rl = host.run_loop;
        host.run_loop = crate::objc::nil; // Гарантируем очистку
        rl
    };
    
    if run_loop_to_remove != crate::objc::nil {
        crate::frameworks::foundation::ns_run_loop::remove_timer(env, run_loop_to_remove, this);
    }
}

- (())fire {
    let &NSTimerHostObject {
        target,
        selector,
        repeats,
        ..
    } = env.objc.borrow(this);
    let pool: id = msg_class![env; NSAutoreleasePool new];

    // Signature should be `- (void)timerDidFire:(NSTimer *)which`.
    let _: () = msg_send(env, (target, selector, this));

    release(env, pool);
    if !repeats {
        () = msg![env; this invalidate];
    }
}

// MARK: - Fire Date

- (())setFireDate:(id)date {
    let time_interval: NSTimeInterval = msg![env; date timeIntervalSinceNow];
        
    let mut timer = env.objc.borrow_mut::<NSTimerHostObject>(this);
    // Согласно документации Apple, если таймер уже остановлен (invalidated), изменение fireDate игнорируется.
    if timer.due_by.is_some() {
        if time_interval.is_nan() || time_interval <= 0.0 {
            timer.due_by = Some(Instant::now());
        } else {
            // Ограничиваем ~100 годами для защиты от паники
            let safe_interval = time_interval.min(100.0 * 365.0 * 24.0 * 3600.0);
            timer.due_by = Some(Instant::now() + Duration::from_secs_f64(safe_interval));
        }
    }
}

- (id)fireDate {
    let timer = env.objc.borrow::<NSTimerHostObject>(this);
    if let Some(due) = timer.due_by {
        let now = Instant::now();
        let time_interval: NSTimeInterval = if due > now {
            due.duration_since(now).as_secs_f64()
        } else {
            -now.duration_since(due).as_secs_f64()
        };
        msg_class![env; NSDate dateWithTimeIntervalSinceNow:time_interval]
    } else {
        nil
    }
}

- (id)initWithFireDate:(id)_date interval:(f64)ti target:(id)t selector:(SEL)s userInfo:(id)ui repeats:(bool)rep {
    let this: id = crate::objc::msg_super![env; this init];
    
    let retained_target = retain(env, t);
    let retained_user_info = retain(env, ui);
    
    let mut host = env.objc.borrow_mut::<NSTimerHostObject>(this);
    
    // ИСПРАВЛЕНИЕ: Защита от <= 0 для предотвращения паники в Rust Duration
    let safe_ti = ti.max(0.0001);
    host.ns_interval = safe_ti;
    host.rust_interval = std::time::Duration::from_secs_f64(safe_ti);
    
    host.target = retained_target;
    host.selector = s;
    host.user_info = retained_user_info;
    host.repeats = rep;
    
    // ИСПРАВЛЕНИЕ: Обрабатываем реальную передачу _date
    let fire_time = if _date != crate::objc::nil {
        let time_interval: NSTimeInterval = msg![env; _date timeIntervalSinceNow];
        if time_interval.is_nan() || time_interval <= 0.0 {
            std::time::Instant::now()
        } else {
            let safe_interval = time_interval.min(100.0 * 365.0 * 24.0 * 3600.0);
            std::time::Instant::now() + std::time::Duration::from_secs_f64(safe_interval)
        }
    } else {
        std::time::Instant::now() + host.rust_interval
    };
    
    host.due_by = Some(fire_time);
    this
}

// =========================================================================
// MARK: - Description
// =========================================================================

- (id)description {
    let host = env.objc.borrow::<NSTimerHostObject>(this);
    let validity = if host.due_by.is_some() { "valid" } else { "invalid" };
    let repeats_str = if host.repeats { "repeats" } else { "one-shot" };
    let selector_str = host.selector.as_str(&env.mem);
    let s = format!(
        "<NSTimer: {:?}; {} {}; interval={:.4}s; target={:?}; selector={}>",
        this,
        validity,
        repeats_str,
        host.ns_interval,
        host.target,
        selector_str
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};

/// For use by `CADisplayLink`
pub fn set_time_interval(env: &mut Environment, timer: id, interval: NSTimeInterval) {
    let host_object = env.objc.borrow_mut::<NSTimerHostObject>(timer);
    // ИСПРАВЛЕНИЕ: Защита от паники
    let safe_interval = interval.max(0.0001);
    host_object.ns_interval = safe_interval;
    host_object.rust_interval = Duration::from_secs_f64(safe_interval);
}

/// For use by `NSRunLoop`
pub(super) fn set_run_loop(env: &mut Environment, timer: id, run_loop: id) {
    let host_object = env.objc.borrow_mut::<NSTimerHostObject>(timer);
    host_object.run_loop = run_loop;
}

/// For use by `NSRunLoop`: check if a timer is due to fire and fire it if necessary.
/// Returns the next firing time, if any.
pub(super) fn handle_timer(env: &mut Environment, timer: id) -> Option<Instant> {
    let &NSTimerHostObject {
        ns_interval,
        rust_interval,
        target,
        selector,
        repeats,
        due_by,
        is_running_callback,
        ..
    } = env.objc.borrow(timer);

    if is_running_callback {
        return None;
    }

    let due_by = due_by?;
    let now = Instant::now();

    if due_by > now {
        return Some(due_by);
    }

    let overdue_by = now.duration_since(due_by);

    // Retain гарантирует, что объект выживет во время исполнения callback'а
    retain(env, timer);

    // Если таймер повторяющийся — пересчитываем следующее время срабатывания
    if repeats {
        let advance_by = (overdue_by.as_secs_f64() / ns_interval).max(1.0).ceil() as u32;
        let advance_by_dur = rust_interval.checked_mul(advance_by).unwrap();
        let next_time = due_by.checked_add(advance_by_dur).unwrap();
        env.objc.borrow_mut::<NSTimerHostObject>(timer).due_by = Some(next_time);
    }

    env.objc.borrow_mut::<NSTimerHostObject>(timer).is_running_callback = true;

    log_dbg!(
        "Timer {:?} fired, sending {:?} message to {:?}",
        timer,
        selector.as_str(&env.mem),
        target
    );

    let pool: id = msg_class![env; NSAutoreleasePool new];

    // Вызываем коллбэк игры/приложения
    let _: () = msg_send(env, (target, selector, timer));

    release(env, pool);

    env.objc.borrow_mut::<NSTimerHostObject>(timer).is_running_callback = false;

    // ИСПРАВЛЕНИЕ: Инвалидируем нецикличный таймер только ПОСЛЕ коллбэка,
    // точно так, как указывает официальная документация Apple.
    // Метод invalidate корректно очистит связи и вызовет ns_run_loop::remove_timer.
    if !repeats {
        let _: () = msg![env; timer invalidate];
    }

    // Заново считываем due_by, так как игра могла вызвать [timer invalidate] внутри коллбэка
    let final_due_by = env.objc.borrow::<NSTimerHostObject>(timer).due_by;
    
    release(env, timer);

    final_due_by
}


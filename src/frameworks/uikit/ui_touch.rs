/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITouch`.

use super::ui_event;
use crate::frameworks::core_graphics::{CGPoint, CGRect};
use crate::frameworks::foundation::{NSInteger, NSTimeInterval, NSUInteger};
use crate::mem::MutVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::window::{Coords, Event, FingerId};
use crate::Environment;
use std::collections::hash_map::{Entry, HashMap};
use std::collections::HashSet;

pub type UITouchPhase = NSInteger;
pub const UITouchPhaseBegan: UITouchPhase = 0;
pub const UITouchPhaseMoved: UITouchPhase = 1;
pub const UITouchPhaseStationary: UITouchPhase = 2;
pub const UITouchPhaseEnded: UITouchPhase = 3;

#[derive(Default)]
pub struct State {
    current_touches: HashMap<FingerId, id>,
}

pub(super) struct UITouchHostObject {
    /// Strong reference to the `UIView`
    pub(super) view: id,
    /// Strong reference to the `UIWindow`, used as a reference for co-ordinate
    /// space conversion
    pub(super) window: id,
    /// Relative to the screen
    location: CGPoint,
    /// Relative to the screen
    previous_location: CGPoint,
    timestamp: NSTimeInterval,
    phase: UITouchPhase,
}
impl HostObject for UITouchHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITouch: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UITouchHostObject {
        view: nil,
        window: nil,
        location: CGPoint { x: 0.0, y: 0.0 },
        previous_location: CGPoint { x: 0.0, y: 0.0 },
        timestamp: 0.0,
        phase: UITouchPhaseBegan,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let &mut UITouchHostObject { view, window, .. } = env.objc.borrow_mut(this);
    release(env, view);
    release(env, window);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (CGPoint)locationInView:(id)that_view { // UIView*
    let &UITouchHostObject { location, window, .. } = env.objc.borrow(this);
    let location_in_window: CGPoint = msg![env; window convertPoint:location fromWindow:nil];
    if that_view == nil {
        location_in_window
    } else {
        msg![env; that_view convertPoint:location_in_window fromView:window]
    }
}
- (CGPoint)previousLocationInView:(id)that_view { // UIView*
    let &UITouchHostObject { previous_location, window, .. } = env.objc.borrow(this);
    let location_in_window: CGPoint = msg![env; window convertPoint:previous_location fromWindow:nil];
    if that_view == nil {
        location_in_window
    } else {
        msg![env; that_view convertPoint:location_in_window fromView:window]
    }
}

- (id)view {
    env.objc.borrow::<UITouchHostObject>(this).view
}

- (NSTimeInterval)timestamp {
    env.objc.borrow::<UITouchHostObject>(this).timestamp
}

- (NSUInteger)tapCount {
    1 // TODO: support double-taps etc
}

- (UITouchPhase)phase {
    env.objc.borrow::<UITouchHostObject>(this).phase
}

@end

};

/// [super::handle_events] will forward touch events to this function.
pub fn handle_event(env: &mut Environment, event: Event) {
    // before processing anything, we mark all current touches as stationary
    let current_touches = &env.framework_state.uikit.ui_touch.current_touches;
    for &touch in (*current_touches).values() {
        env.objc.borrow_mut::<UITouchHostObject>(touch).phase = UITouchPhaseStationary;
    }
    match event {
        Event::TouchesDown(map) => handle_touches_down(env, map),
        Event::TouchesMove(map) => handle_touches_move(env, map),
        Event::TouchesUp(map) => handle_touches_up(env, map),
        _ => unreachable!(),
    }
}

fn handle_touches_down(env: &mut Environment, map: HashMap<FingerId, Coords>) {
    let pool: id = msg_class![env; NSAutoreleasePool new];

    let timestamp: NSTimeInterval = {
        let process_info = msg_class![env; NSProcessInfo processInfo];
        msg![env; process_info systemUptime]
    };

    let touches: id = msg_class![env; NSMutableSet allocWithZone:(MutVoidPtr::null())];

    for (finger_id, coords) in map {
        let current_touches = &mut env.framework_state.uikit.ui_touch.current_touches;

        if current_touches.contains_key(&finger_id) {
            assert_eq!(current_touches.len(), 1);
            log!(
                "Warning: New touch {:?} initiated but current touch did not end yet, treating as movement.",
                finger_id
            );
            return handle_touches_move(env, HashMap::from([(finger_id, coords)]));
        }

        log_dbg!("Finger {:?} touch down: {:?}", finger_id, coords);

        let location = CGPoint {
            x: coords.0,
            y: coords.1,
        };

        let new_touch: id = msg_class![env; UITouch alloc];
        *env.objc.borrow_mut(new_touch) = UITouchHostObject {
            view: nil,
            window: nil,
            location,
            previous_location: location,
            timestamp,
            phase: UITouchPhaseBegan,
        };
        autorelease(env, new_touch);

        let _: () = msg![env; touches addObject:new_touch];

        env.framework_state.uikit.ui_touch.current_touches.insert(finger_id, new_touch);
        retain(env, new_touch);
    }

    let all_touches: id = msg_class![env; NSMutableSet allocWithZone:(MutVoidPtr::null())];
    for &touch in env.framework_state.uikit.ui_touch.current_touches.clone().values() {
        let _: () = msg![env; all_touches addObject:touch];
    }

    let event = ui_event::new_event(env, all_touches);
    autorelease(env, event);

    let views_with_existing_touches: HashSet<id> = env
        .framework_state
        .uikit
        .ui_touch
        .current_touches
        .values()
        .map(|&touch| env.objc.borrow::<UITouchHostObject>(touch).view)
        .collect();

    let mut view_touches: HashMap<id, id> = HashMap::new();

    let touches_arr: id = msg![env; touches allObjects];
    let touches_count: NSUInteger = msg![env; touches_arr count];
    for i in 0..touches_count {
        let touch: id = msg![env; touches_arr objectAtIndex:i];
        let &UITouchHostObject { location, .. } = env.objc.borrow(touch);

        let windows = env.framework_state.uikit.ui_view.ui_window.windows.clone();
        let Some((window, location_in_window)) = windows.into_iter().rev().find_map(|window| {
            let location_in_window: CGPoint =
                msg![env; window convertPoint:location fromWindow:nil];
            if msg![env; window pointInside:location_in_window withEvent:event] {
                Some((window, location_in_window))
            } else {
                None
            }
        }) else {
            log!("Couldn't find a window for touch at {:?}, discarding", location);
            continue;
        };

        let view: id = msg![env; window hitTest:location_in_window withEvent:event];
        if view == nil {
            log!("Couldn't find a view for touch at {:?} in window {:?}, discarding", location_in_window, window);
            continue;
        }

        // --- Исправление для застрявших касаний ---
        let is_multi_touch_enabled: bool = msg![env; view isMultipleTouchEnabled];
        if !is_multi_touch_enabled {
            let view_has_other_new_touches = view_touches.contains_key(&view);
            if view_has_other_new_touches {
                log!("Ignoring new touch {:?} for view {:?}, !isMultipleTouchEnabled (other new touch)", touch, view);
                continue;
            }

            if views_with_existing_touches.contains(&view) {
                // Ищем "призрачные" нажатия, которые не завершились
                let stuck_finger_ids: Vec<FingerId> = env
                    .framework_state
                    .uikit
                    .ui_touch
                    .current_touches
                    .iter()
                    .filter(|(_, &t)| {
                        // Важно: не удаляем текущее обрабатываемое нажатие
                        env.objc.borrow::<UITouchHostObject>(t).view == view && t != touch
                    })
                    .map(|(&fid, _)| fid)
                    .collect();

                if !stuck_finger_ids.is_empty() {
                    log!("Force-clearing {} stuck touch(es) for view {:?} to unblock", stuck_finger_ids.len(), view);
                    for stuck_fid in stuck_finger_ids {
                        if let Some(stuck_touch) = env.framework_state.uikit.ui_touch.current_touches.remove(&stuck_fid) {
                            release(env, stuck_touch);
                        }
                    }
                } else {
                    log!("Ignoring new touch {:?} for view {:?}, !isMultipleTouchEnabled", touch, view);
                    continue;
                }
            }
        }
        // --- Конец исправления ---

        if let Entry::Vacant(e) = view_touches.entry(view) {
            let touches: id = msg_class![env; NSMutableSet allocWithZone:(MutVoidPtr::null())];
            e.insert(touches);
        }
        let touches_set: id = *view_touches.get(&view).unwrap();
        let _: () = msg![env; touches_set addObject:touch];

        retain(env, view);
        retain(env, window);
        {
            let touch_obj = env.objc.borrow_mut::<UITouchHostObject>(touch);
            touch_obj.view = view;
            touch_obj.window = window;
        }
    }

    for (view, touches_set) in view_touches {
        let _: () = msg![env; view touchesBegan:touches_set withEvent:event];
    }

    release(env, pool);
}

fn handle_touches_move(env: &mut Environment, map: HashMap<FingerId, Coords>) {
    let pool: id = msg_class![env; NSAutoreleasePool new];

    let timestamp: NSTimeInterval = {
        let process_info = msg_class![env; NSProcessInfo processInfo];
        msg![env; process_info systemUptime]
    };

    let touches: id = msg_class![env; NSMutableSet allocWithZone:(MutVoidPtr::null())];
    let mut view_touches: HashMap<id, id> = HashMap::new();

    for (finger_id, coords) in map {
        let Some(&touch) = env.framework_state.uikit.ui_touch.current_touches.get(&finger_id) else {
            log!("Warning: Finger {:?} touch move event received but no current touch, ignoring.", finger_id);
            continue;
        };

        let location = CGPoint { x: coords.0, y: coords.1 };
        let view = env.objc.borrow::<UITouchHostObject>(touch).view;
        let host_object = env.objc.borrow_mut::<UITouchHostObject>(touch);

        if host_object.location == location {
            continue;
        }

        log_dbg!("Finger {:?} touch move: {:?}", finger_id, coords);

        host_object.previous_location = host_object.location;
        host_object.location = location;
        host_object.timestamp = timestamp;
        host_object.phase = UITouchPhaseMoved;

        let _: () = msg![env; touches addObject:touch];

        if let Entry::Vacant(e) = view_touches.entry(view) {
            let touches: id = msg_class![env; NSMutableSet allocWithZone:(MutVoidPtr::null())];
            e.insert(touches);
        }
        let touches_set: id = *view_touches.get(&view).unwrap();
        let _: () = msg![env; touches_set addObject:touch];
    }

    let all_touches: id = msg_class![env; NSMutableSet allocWithZone:(MutVoidPtr::null())];
    for &touch in env.framework_state.uikit.ui_touch.current_touches.values() {
        let _: () = msg![env; all_touches addObject:touch];
    }

    let event = ui_event::new_event(env, all_touches);
    autorelease(env, event);

    for (view, touches_set) in view_touches {
        let _: () = msg![env; view touchesMoved:touches_set withEvent:event];
    }

    release(env, pool);
}

fn handle_touches_up(env: &mut Environment, map: HashMap<FingerId, Coords>) {
    let pool: id = msg_class![env; NSAutoreleasePool new];

    let timestamp: NSTimeInterval = {
        let process_info = msg_class![env; NSProcessInfo processInfo];
        msg![env; process_info systemUptime]
    };

    let touches: id = msg_class![env; NSMutableSet allocWithZone:(MutVoidPtr::null())];
    let all_touches: id = msg_class![env; NSMutableSet allocWithZone:(MutVoidPtr::null())];
    for &touch in env.framework_state.uikit.ui_touch.current_touches.values() {
        let _: () = msg![env; all_touches addObject:touch];
    }

    let mut view_touches: HashMap<id, id> = HashMap::new();

    for (finger_id, coords) in map {
        let Some(&touch) = env.framework_state.uikit.ui_touch.current_touches.get(&finger_id) else {
            log!("Warning: Finger {:?} touch up event received but no current touch, ignoring.", finger_id);
            continue;
        };

        log_dbg!("Finger {:?} touch up: {:?}", finger_id, coords);

        let location = CGPoint { x: coords.0, y: coords.1 };
        let view = env.objc.borrow::<UITouchHostObject>(touch).view;
        let host_object = env.objc.borrow_mut::<UITouchHostObject>(touch);
        host_object.previous_location = host_object.location;
        host_object.location = location;
        host_object.timestamp = timestamp;
        host_object.phase = UITouchPhaseEnded;

        let _: () = msg![env; touches addObject:touch];

        if let Entry::Vacant(e) = view_touches.entry(view) {
            let touches: id = msg_class![env; NSMutableSet allocWithZone:(MutVoidPtr::null())];
            e.insert(touches);
        }
        let touches_set: id = *view_touches.get(&view).unwrap();
        let _: () = msg![env; touches_set addObject:touch];

        env.framework_state.uikit.ui_touch.current_touches.remove(&finger_id);
        release(env, touch);
    }

    let event = ui_event::new_event(env, all_touches);
    autorelease(env, event);

    for (view, touches_set) in view_touches {
        let _: () = msg![env; view touchesEnded:touches_set withEvent:event];
    }

    release(env, pool);
}


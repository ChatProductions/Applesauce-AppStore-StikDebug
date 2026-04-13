/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The UIKit framework.
//!
//! For the time being the focus of this project is on running games, which are
//! likely to use UIKit in very simple and limited ways, so this implementation
//! will probably take a lot of shortcuts.

use crate::{msg, Environment};
use std::time::Instant;

use crate::dyld::HostConstant;
use crate::mem::{MutPtr, ConstVoidPtr};
use crate::objc::{id, msg_class, nil, ClassExports, HostObject, NSZonePtr, SEL};
use crate::frameworks::foundation::NSUInteger;
use crate::objc_classes;

pub mod ui_accelerometer;
pub mod ui_action_sheet;
pub mod ui_activity_indicator_view;
pub mod ui_application;
pub mod ui_color;
pub mod ui_device;
pub mod ui_document;
pub mod ui_event;
pub mod ui_font;
pub mod ui_geometry;
pub mod ui_graphics;
pub mod ui_image;
pub mod ui_image_picker_controller;
pub mod ui_keyboard;
pub mod ui_navigation_bar;
pub mod ui_nib;
pub mod ui_pasteboard;
pub mod ui_popover_controller;
pub mod ui_responder;
pub mod ui_screen;
pub mod ui_screen_mode;
pub mod ui_split_view_controller;
pub mod ui_tab_bar_item;
pub mod ui_tab_bar_controller;
pub mod ui_touch;
pub mod ui_view;
pub mod ui_view_controller;

fn ui_background_task_invalid(env: &mut Environment) -> ConstVoidPtr {
    // UIBackgroundTaskInvalid == NSUIntegerMax == 0xFFFF_FFFF
    let ptr: MutPtr<u32> = env.mem.alloc(4).cast();
    env.mem.write(ptr, 0xFFFF_FFFFu32);
    ptr.cast().cast_const()
}

pub const CONSTANTS: &[(&str, HostConstant)] = &[
    ("_UIBackgroundTaskInvalid", HostConstant::Custom(ui_background_task_invalid)),
    ("_UIScreenDidConnectNotification", HostConstant::NSString("UIScreenDidConnectNotification")),
];

// NSUndoManager implementation
#[derive(Default)]
pub struct NSUndoManagerState {
    pub undo_stack: Vec<id>,
    pub redo_stack: Vec<id>,
    pub is_undoing: bool,
    pub is_redoing: bool,
}

struct NSUndoManagerHostObject {
    delegate: id,
    levels: usize,
    groups_by_age: usize,
}
impl HostObject for NSUndoManagerHostObject {}

const NSUNDOMANAGER_CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSUndoManager: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSUndoManagerHostObject {
        delegate: nil,
        levels: 1,
        groups_by_age: 1,
    });
    env.objc.alloc_static_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (id)retain { this }
- (id)autorelease { this }
- (())release {}

- (bool)isUndoing {
    env.framework_state.uikit.undo_manager.is_undoing
}

- (bool)isRedoing {
    env.framework_state.uikit.undo_manager.is_redoing
}

- (bool)canUndo {
    !env.framework_state.uikit.undo_manager.undo_stack.is_empty()
}

- (bool)canRedo {
    !env.framework_state.uikit.undo_manager.redo_stack.is_empty()
}

- (())undo {
    let state = &mut env.framework_state.uikit.undo_manager;
    if let Some(invocation) = state.undo_stack.pop() {
        state.is_undoing = true;
        let _: () = msg![env; invocation invoke];
        state.is_undoing = false;
        state.redo_stack.push(invocation);
    }
}

- (())redo {
    let state = &mut env.framework_state.uikit.undo_manager;
    if let Some(invocation) = state.redo_stack.pop() {
        state.is_redoing = true;
        let _: () = msg![env; invocation invoke];
        state.is_redoing = false;
        state.undo_stack.push(invocation);
    }
}

- (())removeAllActions {
    let state = &mut env.framework_state.uikit.undo_manager;
    state.undo_stack.clear();
    state.redo_stack.clear();
}

- (())removeAllActionsWithTarget:(id)target {
    let state = &mut env.framework_state.uikit.undo_manager;
    state.undo_stack.retain(|inv| {
        let inv_target: id = msg![env; inv target];
        inv_target != target
    });
    state.redo_stack.retain(|inv| {
        let inv_target: id = msg![env; inv target];
        inv_target != target
    });
}

- (NSUInteger)undoCount {
    env.framework_state.uikit.undo_manager.undo_stack.len() as NSUInteger
}

- (NSUInteger)redoCount {
    env.framework_state.uikit.undo_manager.redo_stack.len() as NSUInteger
}

- (())registerUndoWithTarget:(id)target
                    selector:(SEL)selector
                      object:(id)object {
    let signature: id = msg_class![env; NSMethodSignature signatureWithObjCTypes:"v@:@"];

    let invocation: id = msg_class![env; NSInvocation invocationWithMethodSignature:signature];
    () = msg![env; invocation setTarget:target];
    () = msg![env; invocation setSelector:selector];
    if object != nil {
        () = msg![env; invocation setArgument:object atIndex:3];
    }
    
    let state = &mut env.framework_state.uikit.undo_manager;
    state.undo_stack.push(invocation);
    state.redo_stack.clear();
}

- (())forwardInvocation:(id)invocation {
    let target: id = msg![env; invocation target];
    if target != nil {
        () = msg![env; invocation invoke];
    }
}

- (id)methodSignatureForSelector:(SEL)_selector {
    msg_class![env; NSMethodSignature signatureWithObjCTypes:"v@:"]
}

- (id)delegate {
    env.objc.borrow::<NSUndoManagerHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    let host_object = env.objc.borrow_mut::<NSUndoManagerHostObject>(this);
    host_object.delegate = delegate;
}

- (NSInteger)levels {
    env.objc.borrow::<NSUndoManagerHostObject>(this).levels as NSInteger
}

- (())setLevels:(NSInteger)levels {
    let host_object = env.objc.borrow_mut::<NSUndoManagerHostObject>(this);
    host_object.levels = levels as usize;
}

- (())beginUndoGrouping {
    // Stub - grouping handled implicitly
}

- (())endUndoGrouping {
    // Stub - grouping handled implicitly
}

- (NSInteger)groupingLevel {
    0
}

- (())endAllGrouping {
    // Stub
}

- (())setGroupsByEvent:(bool)groupsByEvent {
    let host_object = env.objc.borrow_mut::<NSUndoManagerHostObject>(this);
    host_object.groups_by_age = if groupsByEvent { 1 } else { 0 };
}

- (bool)groupsByEvent {
    let host_object = env.objc.borrow::<NSUndoManagerHostObject>(this);
    host_object.groups_by_age != 0
}

- (())undoNestedGroup {
    () = msg![env; this undo]
}

- (id)undoActionName {
    if () != msg![env; this canUndo] {
        let pool: id = msg_class![env; NSAutoreleasePool new];
        let name = crate::frameworks::foundation::ns_string::from_rust_string(env, "Undo".to_string());
        let result = crate::objc::retain(env, name);
        () = msg![env; pool drain];
        result
    } else {
        nil
    }
}

- (id)redoActionName {
    if () != msg![env; this canRedo] {
        let pool: id = msg_class![env; NSAutoreleasePool new];
        let name = crate::frameworks::foundation::ns_string::from_rust_string(env, "Redo".to_string());
        let result = crate::objc::retain(env, name);
        () = msg![env; pool drain];
        result
    } else {
        nil
    }
}

- (id)undoMenuItemTitle {
    msg![env; this undoActionName]
}

- (id)redoMenuItemTitle {
    msg![env; this redoActionName]
}

- (())setUndoManager:(id)_manager {
    // Stub
}

@end

};

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/UIKit.framework/UIKit",
    aliases: &[],
    class_exports: &[
        ui_accelerometer::CLASSES,
        ui_action_sheet::CLASSES,
        ui_activity_indicator_view::CLASSES,
        ui_application::CLASSES,
        ui_color::CLASSES,
        ui_device::CLASSES,
        ui_document::CLASSES,
        ui_event::CLASSES,
        ui_font::CLASSES,
        ui_image::CLASSES,
        ui_image_picker_controller::CLASSES,
        ui_keyboard::CLASSES,
        ui_navigation_bar::CLASSES,
        ui_nib::CLASSES,
        ui_pasteboard::CLASSES,
        ui_popover_controller::CLASSES,
        ui_responder::CLASSES,
        ui_screen_mode::CLASSES,
        ui_screen::CLASSES,
        ui_split_view_controller::CLASSES,
        ui_tab_bar_item::CLASSES,
        ui_tab_bar_controller::CLASSES,
        ui_touch::CLASSES,
        ui_view::CLASSES,
        ui_view::ui_alert_view::CLASSES,
        ui_view::ui_control::CLASSES,
        ui_view::ui_control::ui_bar_button_item::CLASSES,
        ui_view::ui_control::ui_button::CLASSES,
        ui_view::ui_control::ui_segmented_control::CLASSES,
        ui_view::ui_control::ui_slider::CLASSES,
        ui_view::ui_control::ui_text_field::CLASSES,
        ui_view::ui_control::ui_switch::CLASSES,
        ui_view::ui_image_view::CLASSES,
        ui_view::ui_label::CLASSES,
        ui_view::ui_page_control::CLASSES,
        ui_view::ui_picker_view::CLASSES,
        ui_view::ui_scroll_view::CLASSES,
        ui_view::ui_scroll_view::ui_text_view::CLASSES,
        ui_view::ui_table_view::CLASSES,
        ui_view::ui_toolbar::CLASSES,
        ui_view::ui_web_view::CLASSES,
        ui_view::ui_window::CLASSES,
        ui_view_controller::CLASSES,
        ui_view_controller::ui_navigation_controller::CLASSES,
        NSUNDOMANAGER_CLASSES,
    ],
    constant_exports: &[
        ui_application::CONSTANTS,
        ui_device::CONSTANTS,
        ui_keyboard::CONSTANTS,
        ui_view::ui_control::ui_text_field::CONSTANTS,
        ui_view::ui_window::CONSTANTS,
        CONSTANTS,
    ],
    function_exports: &[
        ui_application::FUNCTIONS,
        ui_geometry::FUNCTIONS,
        ui_graphics::FUNCTIONS,
        ui_image::FUNCTIONS,
    ],
};

#[derive(Default)]
pub struct State {
    ui_accelerometer: ui_accelerometer::State,
    ui_application: ui_application::State,
    ui_color: ui_color::State,
    ui_device: ui_device::State,
    ui_font: ui_font::State,
    ui_geometry: ui_geometry::State,
    ui_graphics: ui_graphics::State,
    ui_image: ui_image::State,
    ui_screen: ui_screen::State,
    ui_touch: ui_touch::State,
    pub ui_view: ui_view::State,
    ui_responder: ui_responder::State,
    pub undo_manager: NSUndoManagerState,
}

/// For use by `NSRunLoop`: handles any events that have queued up.
///
/// Returns the next time this function must be called, if any, e.g. the next
/// time an accelerometer input is due.
pub fn handle_events(env: &mut Environment) -> Option<Instant> {
    use crate::window::Event;
    use crate::window::TextInputEvent;

    loop {
        // NSRunLoop will never call this function in headless mode.
        let Some(event) = env.window_mut().pop_event() else {
            break;
        };

        match event {
            Event::Quit => {
                echo!("User requested quit, exiting.");
                ui_application::exit(env);
            }
            Event::TouchesDown(..) | Event::TouchesMove(..) | Event::TouchesUp(..) => {
                ui_touch::handle_event(env, event)
            }
            Event::AppWillResignActive => {
                // Getting this event means touchHLE is becoming inactive, e.g.
                // due to switching apps. The obvious way to handle this would
                // be to just send `applicationWillResignActive:` to the
                // UIApplicationDelegate. However:
                // - touchHLE's event loop can't handle an inactive app well
                //   right now. For example, audio isn't paused.
                // - touchHLE's event loop can't handle the subsequent
                //   termination of an app right now: it doesn't manage to send
                //   the `applicationWillTerminate:` message in time. This can
                //   mean loss of data!
                // Therefore, for the moment we will simulate the early iOS
                // behavior where switching app usually resulted in termination.
                // We can usually handle this in time, so there won't be data
                // loss, nor problems with background resource usage or audio.
                // TODO: Handle this better.
                log!("Handling app-will-resign-active event: exiting.");
                ui_application::exit(env);
            }
            Event::AppWillTerminate => {
                log!("Handling app-will-terminate event.");
                ui_application::exit(env);
            }
            Event::EnterDebugger => {
                if env.is_debugging_enabled() {
                    log!("Handling EnterDebugger event: entering debugger.");
                    env.enter_debugger(/* reason: */ None);
                } else {
                    log!("Ignoring EnterDebugger event: no debugger connected.");
                }
            }
            Event::TextInput(text_event) => {
                let responder = env.framework_state.uikit.ui_responder.first_responder;
                let class = msg![env; responder class];
                let ui_text_field_class = env.objc.get_known_class("UITextField", &mut env.mem);

                if !responder.is_null() && env.objc.class_is_subclass_of(class, ui_text_field_class)
                {
                    match text_event {
                        TextInputEvent::Text(text) => {
                            ui_view::ui_control::ui_text_field::handle_text(env, responder, text)
                        }
                        TextInputEvent::Backspace => {
                            ui_view::ui_control::ui_text_field::handle_backspace(env, responder)
                        }
                        TextInputEvent::Return => {
                            ui_view::ui_control::ui_text_field::handle_return(env, responder)
                        }
                    }
                }
            }
        }
    }

    ui_accelerometer::handle_accelerometer(env)
}

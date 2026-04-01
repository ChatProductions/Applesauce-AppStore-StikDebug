/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITabBarController`.

use crate::frameworks::foundation::ns_string;
use crate::frameworks::foundation::NSUInteger;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

struct UITabBarControllerHostObject {
    /// `NSArray*` of `UIViewController*`
    view_controllers: id,
    selected_index: NSUInteger,
    delegate: id,
}
impl HostObject for UITabBarControllerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITabBarController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UITabBarControllerHostObject {
        view_controllers: nil,
        selected_index: 0,
        delegate: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    let view_controllers = msg_class![env; NSArray new];
    env.objc.borrow_mut::<UITabBarControllerHostObject>(this).view_controllers = view_controllers;
    this
}

- (())dealloc {
    let host = env.objc.borrow::<UITabBarControllerHostObject>(this);
    let view_controllers = host.view_controllers;
    let delegate = host.delegate;
    release(env, view_controllers);
    release(env, delegate);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - View controllers

- (id)viewControllers {
    env.objc.borrow::<UITabBarControllerHostObject>(this).view_controllers
}

- (())setViewControllers:(id)view_controllers { // NSArray*
    let old = env.objc.borrow::<UITabBarControllerHostObject>(this).view_controllers;
    release(env, old);
    retain(env, view_controllers);
    env.objc.borrow_mut::<UITabBarControllerHostObject>(this).view_controllers = view_controllers;

    // Reset selected index if out of bounds
    let count: NSUInteger = msg![env; view_controllers count];
    let idx = env.objc.borrow::<UITabBarControllerHostObject>(this).selected_index;
    if count == 0 {
        env.objc.borrow_mut::<UITabBarControllerHostObject>(this).selected_index = 0;
    } else if idx >= count {
        env.objc.borrow_mut::<UITabBarControllerHostObject>(this).selected_index = 0;
    }
}

- (())setViewControllers:(id)view_controllers animated:(bool)_animated {
    msg![env; this setViewControllers:view_controllers]
}

// MARK: - Selected index / controller

- (NSUInteger)selectedIndex {
    env.objc.borrow::<UITabBarControllerHostObject>(this).selected_index
}

- (())setSelectedIndex:(NSUInteger)index {
    let vcs = env.objc.borrow::<UITabBarControllerHostObject>(this).view_controllers;
    let count: NSUInteger = msg![env; vcs count];
    if index >= count {
        log!(
            "Warning: [UITabBarController setSelectedIndex:{}] out of bounds (count {})",
            index, count
        );
        return;
    }
    env.objc.borrow_mut::<UITabBarControllerHostObject>(this).selected_index = index;

    let delegate = env.objc.borrow::<UITabBarControllerHostObject>(this).delegate;
    if delegate != nil {
        let vc: id = msg![env; vcs objectAtIndex:index];
        msg![env; delegate tabBarController:this didSelectViewController:vc]
    }
}

- (id)selectedViewController {
    let vcs = env.objc.borrow::<UITabBarControllerHostObject>(this).view_controllers;
    let count: NSUInteger = msg![env; vcs count];
    if count == 0 {
        return nil;
    }
    let idx = env.objc.borrow::<UITabBarControllerHostObject>(this).selected_index;
    msg![env; vcs objectAtIndex:idx]
}

- (())setSelectedViewController:(id)view_controller {
    let vcs = env.objc.borrow::<UITabBarControllerHostObject>(this).view_controllers;
    let count: NSUInteger = msg![env; vcs count];
    let mut found_index: Option<NSUInteger> = None;
    let mut i: NSUInteger = 0;
    while i < count {
        let vc: id = msg![env; vcs objectAtIndex:i];
        if vc == view_controller {
            found_index = Some(i);
            break;
        }
        i += 1;
    }
    if let Some(index) = found_index {
        msg![env; this setSelectedIndex:index]
    } else {
        log!("Warning: [UITabBarController setSelectedViewController:] view controller not found in list");
    }
}

// MARK: - Delegate

- (id)delegate {
    env.objc.borrow::<UITabBarControllerHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    let old = env.objc.borrow::<UITabBarControllerHostObject>(this).delegate;
    release(env, old);
    retain(env, delegate);
    env.objc.borrow_mut::<UITabBarControllerHostObject>(this).delegate = delegate;
}

// MARK: - Tab bar (stub accessor)

- (id)tabBar {
    // Return self as a stand-in; apps typically just read this to configure
    // appearance properties we don't render anyway.
    log!("TODO: [UITabBarController tabBar] — returning nil");
    nil
}

// MARK: - UIViewController overrides

- (id)view {
    // Delegate to the currently selected child view controller.
    let vc: id = msg![env; this selectedViewController];
    if vc == nil {
        return nil;
    }
    msg![env; vc view]
}

- (())viewDidLoad {
    let vc: id = msg![env; this selectedViewController];
    if vc != nil {
        msg![env; vc viewDidLoad]
    }
}

- (())viewWillAppear:(bool)animated {
    let vc: id = msg![env; this selectedViewController];
    if vc != nil {
        msg![env; vc viewWillAppear:animated]
    }
}

- (())viewDidAppear:(bool)animated {
    let vc: id = msg![env; this selectedViewController];
    if vc != nil {
        msg![env; vc viewDidAppear:animated]
    }
}

- (())viewWillDisappear:(bool)animated {
    let vc: id = msg![env; this selectedViewController];
    if vc != nil {
        msg![env; vc viewWillDisappear:animated]
    }
}

- (())viewDidDisappear:(bool)animated {
    let vc: id = msg![env; this selectedViewController];
    if vc != nil {
        msg![env; vc viewDidDisappear:animated]
    }
}

@end

};

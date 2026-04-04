/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSInvocation` and `NSMethodSignature`.

use crate::frameworks::foundation::NSInteger;
use crate::mem::MutVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr, SEL,
};
use std::collections::HashMap;

// =========================================================================
// MARK: - NSMethodSignature Host Object
// =========================================================================

struct NSMethodSignatureHostObject {}
impl HostObject for NSMethodSignatureHostObject {}

// =========================================================================
// MARK: - NSInvocation Host Object
// =========================================================================

struct NSInvocationHostObject {
    signature: id,
    target: id,
    selector: Option<SEL>,
    arguments: HashMap<NSInteger, u32>,
    arguments_retained: bool,
}
impl HostObject for NSInvocationHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - NSMethodSignature
// =========================================================================

@implementation NSMethodSignature: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSMethodSignatureHostObject {});
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)signatureWithObjCTypes:(MutVoidPtr)_types {
    let sig: id = msg_class![env; NSMethodSignature alloc];
    let sig: id = msg![env; sig init];
    autorelease(env, sig)
}

- (id)init {
    this
}

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// =========================================================================
// MARK: - NSInvocation
// =========================================================================

@implementation NSInvocation: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSInvocationHostObject {
        signature: nil,
        target: nil,
        selector: None,
        arguments: HashMap::new(),
        arguments_retained: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)invocationWithMethodSignature:(id)sig {
    let inv: id = msg_class![env; NSInvocation alloc];
    let inv: id = msg![env; inv init];
    
    env.objc.borrow_mut::<NSInvocationHostObject>(inv).signature = sig;
    retain(env, sig);
    
    autorelease(env, inv)
}

- (id)init {
    this
}

- (())dealloc {
    let (target, sig, retained) = {
        let host = env.objc.borrow::<NSInvocationHostObject>(this);
        (host.target, host.signature, host.arguments_retained)
    };
    
    if retained && target != nil {
        release(env, target);
    }
    if sig != nil {
        release(env, sig);
    }
    
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)methodSignature {
    env.objc.borrow::<NSInvocationHostObject>(this).signature
}

- (())retainArguments {
    let (retained, target) = {
        let host = env.objc.borrow::<NSInvocationHostObject>(this);
        (host.arguments_retained, host.target)
    };
    
    if !retained {
        env.objc.borrow_mut::<NSInvocationHostObject>(this).arguments_retained = true;
        if target != nil {
            retain(env, target);
        }
    }
}

- (bool)argumentsRetained {
    env.objc.borrow::<NSInvocationHostObject>(this).arguments_retained
}

- (id)target {
    env.objc.borrow::<NSInvocationHostObject>(this).target
}

- (())setTarget:(id)target {
    env.objc.borrow_mut::<NSInvocationHostObject>(this).target = target;
}

- (SEL)selector {
    env.objc.borrow::<NSInvocationHostObject>(this).selector.unwrap_or(SEL::null())
}

- (())setSelector:(SEL)selector {
    env.objc.borrow_mut::<NSInvocationHostObject>(this).selector = Some(selector);
}

- (())getArgument:(MutVoidPtr)buffer atIndex:(NSInteger)index {
    let val = env.objc.borrow::<NSInvocationHostObject>(this).arguments.get(&index).copied().unwrap_or(0);
    env.mem.write(buffer.cast::<u32>(), val);
}

- (())setArgument:(MutVoidPtr)buffer atIndex:(NSInteger)index {
    let val = env.mem.read(buffer.cast::<u32>());
    env.objc.borrow_mut::<NSInvocationHostObject>(this).arguments.insert(index, val);
}

- (())invoke {
    let target = env.objc.borrow::<NSInvocationHostObject>(this).target;
    if target != nil {
        () = msg![env; this invokeWithTarget:target];
    }
}

- (())invokeWithTarget:(id)target {
    let sel = env.objc.borrow::<NSInvocationHostObject>(this).selector.expect("NSInvocation invoked without a selector");
    let args = env.objc.borrow::<NSInvocationHostObject>(this).arguments.clone();
    
    let sel_str = sel.as_str(&env.mem);
    let arg_count = sel_str.chars().filter(|&c| c == ':').count();

    // В Objective-C индексы 0 и 1 заняты под `self` и `_cmd`. Аргументы пользователя начинаются с индекса 2.
    if arg_count == 0 {
        let _: u32 = crate::objc::msg_send_no_type_checking(env, (target, sel));
    } else if arg_count == 1 {
        let arg2 = args.get(&2).copied().unwrap_or(0);
        let _: u32 = crate::objc::msg_send_no_type_checking(env, (target, sel, arg2));
    } else if arg_count == 2 {
        let arg2 = args.get(&2).copied().unwrap_or(0);
        let arg3 = args.get(&3).copied().unwrap_or(0);
        let _: u32 = crate::objc::msg_send_no_type_checking(env, (target, sel, arg2, arg3));
    } else if arg_count == 3 {
        let arg2 = args.get(&2).copied().unwrap_or(0);
        let arg3 = args.get(&3).copied().unwrap_or(0);
        let arg4 = args.get(&4).copied().unwrap_or(0);
        let _: u32 = crate::objc::msg_send_no_type_checking(env, (target, sel, arg2, arg3, arg4));
    } else {
        log!("TODO: NSInvocation invokeWithTarget: {} with > 3 arguments is not fully supported yet", sel_str);
    }
}

@end

};

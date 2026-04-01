use super::ns_string;
use super::NSInteger;
use crate::objc::{autorelease, id, objc_classes, ClassExports, HostObject};
use crate::msg_class;

struct NSAssertionHandlerHostObject {}
impl HostObject for NSAssertionHandlerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSAssertionHandler: NSObject

/// Returns the assertion handler for the current thread.
/// Real Cocoa stores one per NSThread; we use a fresh autoreleased instance
/// (sufficient for a single-threaded guest).
+ (id)currentHandler {
    let host = Box::new(NSAssertionHandlerHostObject {});
    let new = env.objc.alloc_object(this, host, &mut env.mem);
    autorelease(env, new)
}

/// Raised by NSAssert / NSParameterAssert (ObjC method context).
- (())handleFailureInMethod:(SEL)_selector
                    object:(id)_object
                      file:(id)file
                lineNumber:(NSInteger)line
               description:(id)description {
    let file_str = ns_string::to_rust_string(env, file);
    let desc_str = ns_string::to_rust_string(env, description);
    panic!(
        "NSAssert failed at {}:{} — {}",
        file_str, line, desc_str
    );
}

/// Raised by NSCAssert / NSCParameterAssert (C function context).
- (())handleFailureInFunction:(id)function_name
                         file:(id)file
                   lineNumber:(NSInteger)line
                  description:(id)description {
    let func_str = ns_string::to_rust_string(env, function_name);
    let file_str = ns_string::to_rust_string(env, file);
    let desc_str = ns_string::to_rust_string(env, description);
    panic!(
        "NSCAssert failed in {} at {}:{} — {}",
        func_str, file_str, line, desc_str
    );
}

@end

};

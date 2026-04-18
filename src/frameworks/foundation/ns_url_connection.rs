/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `NSURLConnection`.
//!
//! This is a stub implementation that does not perform real networking.
//!
//! Synchronous requests return empty NSData with a descriptive NSError.
//! Asynchronous connections immediately call `connection:didFailWithError:`
//! on the delegate (if it implements that method) so the app can handle the
//! failure gracefully instead of hanging or crashing.

use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr, SEL,
};

// NSError domain / code we use when reporting "no network in emulator".
const NS_URL_ERROR_DOMAIN: &str = "NSURLErrorDomain";
const NS_URL_ERROR_NOT_CONNECTED_TO_INTERNET: i32 = -1009;

// ---------------------------------------------------------------------------
// Host object — stores the delegate so we can call it back.
// ---------------------------------------------------------------------------

struct NSURLConnectionHostObject {
    /// `id<NSURLConnectionDelegate>` — weak-ish: we retain it while the
    /// connection is alive and release on dealloc / cancel.
    delegate: id,
    /// Whether the connection has already been cancelled / finished.
    cancelled: bool,
}
impl HostObject for NSURLConnectionHostObject {}

// ---------------------------------------------------------------------------
// Helper — build an NSError for "not connected to internet".
// ---------------------------------------------------------------------------
fn make_network_error(env: &mut crate::Environment) -> id {
    use crate::frameworks::foundation::ns_string::{from_rust_string, get_static_str};

    let domain = from_rust_string(env, NS_URL_ERROR_DOMAIN.to_string());
    autorelease(env, domain);

    let desc_key = get_static_str(env, "NSLocalizedDescription");
    let desc_val = from_rust_string(
        env,
        "The network connection was lost. \
         (touchHLE: networking not supported)"
            .to_string(),
    );
    autorelease(env, desc_val);

    let user_info: id = msg_class![env; NSMutableDictionary new];
    autorelease(env, user_info);
    () = msg![env; user_info setObject:desc_val forKey:desc_key];

    let error: id = msg_class![env; NSError alloc];
    let error: id = msg![env;
        error initWithDomain:domain
                        code:NS_URL_ERROR_NOT_CONNECTED_TO_INTERNET
                    userInfo:user_info];
    autorelease(env, error);
    error
}

// ---------------------------------------------------------------------------
// Helper — call `connection:didFailWithError:` on the delegate only if it
// actually implements that selector. Avoids panic on unimplemented methods.
// ---------------------------------------------------------------------------
fn notify_delegate_failure(env: &mut crate::Environment, connection: id, delegate: id) {
    if delegate == nil {
        return;
    }

    // Guard: only call the selector if the delegate responds to it.
    // This prevents a panic when the delegate class does not implement the
    // optional method (common for lightweight network-check callers).
    // Outside objc_classes! so {:?} is safe for id values.
    
    // Исправлено: используем экспортированную функцию selector.
    let sel: SEL = crate::objc::selector(env, "connection:didFailWithError:");
    let responds: bool = msg![env; delegate respondsToSelector:sel];

    if !responds {
        log_dbg!(
            "NSURLConnection: delegate {:?} does not respond to \
             connection:didFailWithError:, skipping",
            delegate
        );
        return;
    }

    let error = make_network_error(env);
    log_dbg!(
        "NSURLConnection: notifying delegate {:?} of failure on connection {:?}",
        delegate,
        connection
    );

    () = msg![env; delegate connection:connection didFailWithError:error];
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURLConnection: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(NSURLConnectionHostObject {
        delegate: nil,
        cancelled: false,
    });
    env.objc.alloc_object(this, host, &mut env.mem)
}

// MARK: - canHandleRequest: (class method)

+ (bool)canHandleRequest:(id)_request {
    // Advertise support so the app doesn't take a different code path;
    // failure is reported asynchronously via the delegate instead.
    true
}

// MARK: - Synchronous API

+ (id)sendSynchronousRequest:(id)request
           returningResponse:(MutPtr<id>)response_ptr
                       error:(MutPtr<id>)error_ptr {

    // No {:?} / {:#x} for id inside objc_classes! — log without pointer.
    log!("NSURLConnection sendSynchronousRequest: stub called");

    // When request is nil we still return empty NSData rather than nil,
    // because many callers do not nil-check the return value and will crash
    // on a null dereference otherwise.
    if request == nil {
        log!(
            "NSURLConnection sendSynchronousRequest: nil request — \
             returning empty NSData to prevent caller crash"
        );
    }

    // Write nil into *response (no HTTP response to report).
    if !response_ptr.is_null() {
        env.mem.write(response_ptr, nil);
    }

    // Build and write an NSError so the caller knows why it got empty data.
    if !error_ptr.is_null() {
        let error = make_network_error(env);
        // make_network_error already autoreleased; retain once more so the
        // caller owns a +1 reference through the out-pointer.
        retain(env, error);
        env.mem.write(error_ptr, error);
    }

    // Always return empty NSData (never nil) to avoid null-deref crashes in
    // callers that do not check the error out-pointer.
    let empty_data: id = msg_class![env; NSData data];
    empty_data
}

// MARK: - Asynchronous API

+ (id)connectionWithRequest:(id)request
                   delegate:(id)delegate {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRequest:request delegate:delegate];
    autorelease(env, new);
    new
}

- (id)initWithRequest:(id)request
             delegate:(id)delegate {
    msg![env;
        this initWithRequest:request
                    delegate:delegate
            startImmediately:true]
}

- (id)initWithRequest:(id)request
             delegate:(id)delegate
     startImmediately:(bool)start_immediately {

    if request == nil {
        log!("NSURLConnection initWithRequest: nil request — returning nil");
        release(env, this);
        return nil;
    }

    // No {:?} for id inside objc_classes! — log bool only.
    log!(
        "NSURLConnection initWithRequest:... delegate:... \
         startImmediately:{} (stub — failure via delegate)",
        start_immediately,
    );

    retain(env, delegate);
    {
        let mut host = env.objc.borrow_mut::<NSURLConnectionHostObject>(this);
        host.delegate  = delegate;
        host.cancelled = false;
    }

    if start_immediately {
        env.objc.borrow_mut::<NSURLConnectionHostObject>(this).cancelled = true;
        retain(env, this);
        notify_delegate_failure(env, this, delegate);
        autorelease(env, this);
    }

    this
}

// MARK: - Instance methods

- (())start {
    let host     = env.objc.borrow::<NSURLConnectionHostObject>(this);
    let already  = host.cancelled;
    let delegate = host.delegate;
    drop(host);

    if !already {
        env.objc.borrow_mut::<NSURLConnectionHostObject>(this).cancelled = true;
        retain(env, this);
        notify_delegate_failure(env, this, delegate);
        autorelease(env, this);
    }
}

- (())cancel {
    // No {:?}/{:#x} for id inside objc_classes!
    log_dbg!("NSURLConnection cancel");
    // Mark cancelled; do NOT call the delegate (Apple behaviour: cancelled
    // connections do not call connection:didFailWithError:).
    env.objc.borrow_mut::<NSURLConnectionHostObject>(this).cancelled = true;
}

// MARK: - Dealloc

- (())dealloc {
    log_dbg!("NSURLConnection dealloc");
    let delegate = env.objc.borrow::<NSURLConnectionHostObject>(this).delegate;
    release(env, delegate);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};


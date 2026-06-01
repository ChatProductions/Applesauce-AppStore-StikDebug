/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `NSURLConnection`.
//!
//! This implementation fakes successful network responses so that games
//! requiring server connectivity (e.g. Sonic Runners, The Simpsons) can
//! proceed past their network checks. Instead of reporting
//! NSURLErrorNotConnectedToInternet, we synthesize a 200 OK HTTP response
//! with an empty body, allowing the app to continue as if the server
//! returned an empty/valid response.

use crate::mem::{MutPtr, MutVoidPtr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

// ---------------------------------------------------------------------------
// Host object — stores the delegate so we can call it back.
// ---------------------------------------------------------------------------

struct NSURLConnectionHostObject {
    /// `id<NSURLConnectionDelegate>` — retained while the connection is
    /// alive, released on dealloc / cancel.
    delegate: id,
    /// Whether the connection has already been cancelled / finished.
    cancelled: bool,
}
impl HostObject for NSURLConnectionHostObject {}

// ---------------------------------------------------------------------------
// Helper — build a fake NSHTTPURLResponse with status 200.
// ---------------------------------------------------------------------------
fn make_fake_response(env: &mut crate::Environment, request: id) -> id {
    use crate::frameworks::foundation::ns_string::from_rust_string;

    // Get the URL from the request, or use a placeholder
    let url: id = if request != nil {
        msg![env; request URL]
    } else {
        nil
    };

    let url = if url == nil {
        // Create a dummy URL
        let url_str = from_rust_string(env, "http://localhost/".to_string());
        let dummy_url: id = msg_class![env; NSURL URLWithString:url_str];
        release(env, url_str);
        dummy_url
    } else {
        url
    };

    // Create NSHTTPURLResponse with status 200
    // NSHTTPURLResponse initWithURL:statusCode:HTTPVersion:headerFields:
    let http_version = from_rust_string(env, "HTTP/1.1".to_string());
    autorelease(env, http_version);

    // Create minimal headers dictionary
    let content_type_key = from_rust_string(env, "Content-Type".to_string());
    autorelease(env, content_type_key);
    let content_type_val = from_rust_string(env, "application/json".to_string());
    autorelease(env, content_type_val);
    let content_length_key = from_rust_string(env, "Content-Length".to_string());
    autorelease(env, content_length_key);
    let content_length_val = from_rust_string(env, "2".to_string());
    autorelease(env, content_length_val);

    let headers: id = msg_class![env; NSMutableDictionary new];
    autorelease(env, headers);
    () = msg![env; headers setObject:content_type_val forKey:content_type_key];
    () = msg![env; headers setObject:content_length_val forKey:content_length_key];

    let response: id = msg_class![env; NSHTTPURLResponse alloc];
    let response: id = msg![env; response initWithURL:url
                                           statusCode:200i32
                                          HTTPVersion:http_version
                                         headerFields:headers];
    autorelease(env, response);
    response
}

// ---------------------------------------------------------------------------
// Helper — build fake response data (empty JSON object).
// ---------------------------------------------------------------------------
fn make_fake_data(env: &mut crate::Environment) -> id {
    use crate::frameworks::foundation::ns_string::from_rust_string;

    // Return a minimal valid JSON response "{}" so JSON parsers don't crash
    let json_str = from_rust_string(env, "{}".to_string());
    let data: id = msg![env; json_str dataUsingEncoding:4u32]; // NSUTF8StringEncoding = 4
    release(env, json_str);
    autorelease(env, data)
}

// ---------------------------------------------------------------------------
// Helper — notify delegate of successful (faked) connection.
// Follows Apple's NSURLConnectionDataDelegate protocol:
// 1. connection:didReceiveResponse:
// 2. connection:didReceiveData:
// 3. connectionDidFinishLoading:
// ---------------------------------------------------------------------------
fn notify_delegate_success(env: &mut crate::Environment, connection: id, delegate: id, request: id) {
    if delegate == nil {
        return;
    }
    log_dbg!("NSURLConnection: notifying delegate of fake success");

    let response = make_fake_response(env, request);
    let data = make_fake_data(env);

    // connection:didReceiveResponse:
    () = msg![env; delegate connection:connection didReceiveResponse:response];

    // connection:didReceiveData:
    () = msg![env; delegate connection:connection didReceiveData:data];

    // connectionDidFinishLoading:
    () = msg![env; delegate connectionDidFinishLoading:connection];
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
    // Advertise support so the app doesn't take a different code path.
    true
}

// MARK: - Synchronous API

+ (id)sendSynchronousRequest:(id)request
           returningResponse:(MutPtr<id>)response_ptr
                       error:(MutPtr<id>)error_ptr {

    log!("NSURLConnection sendSynchronousRequest: returning fake 200 OK response");

    // Build a fake HTTP 200 response
    let response = make_fake_response(env, request);

    // Write the response into *response_ptr
    if !response_ptr.is_null() {
        retain(env, response);
        env.mem.write(response_ptr, response);
    }

    // No error — write nil
    if !error_ptr.is_null() {
        env.mem.write(error_ptr, nil);
    }

    // Return fake response data (empty JSON)
    let data = make_fake_data(env);
    retain(env, data);
    autorelease(env, data)
}

// MARK: - Asynchronous block API

+ (())sendAsynchronousRequest:(id)request
                        queue:(id)queue
            completionHandler:(MutVoidPtr)handler {
    if handler.is_null() {
        return;
    }
    log!(
        "NSURLConnection sendAsynchronousRequest:queue:completionHandler: \
         delivering fake 200 OK success response (touchHLE network stub)"
    );

    // Build fake successful response
    let response = make_fake_response(env, request);
    let data = make_fake_data(env);

    // The completion handler is a `void (^)(NSURLResponse *, NSData *,
    // NSError *)` block. ARM32 ABI: the block struct's third word
    // (index 3 == byte offset 12) is the invoke function pointer.
    let invoke_ptr = env.mem.read(handler.cast::<u32>() + 3u32);
    if invoke_ptr == 0 {
        return;
    }
    use crate::abi::CallFromHost;
    let invoke = crate::abi::GuestFunction::from_addr_with_thumb_bit(invoke_ptr);

    let _ = queue;
    // Call with (response, data, nil_error) — success!
    let _: () = invoke.call_from_host(env, (handler, response, data, nil));
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

    log_dbg!(
        "NSURLConnection initWithRequest:... delegate:... \
         startImmediately:{} (stub — fake success via delegate)",
        start_immediately,
    );

    retain(env, delegate);
    {
        let host = env.objc.borrow_mut::<NSURLConnectionHostObject>(this);
        host.delegate  = delegate;
        host.cancelled = false;
    }

    if start_immediately {
        // Schedule the delegate success callback via
        // performSelector:withObject:afterDelay: so that it fires on the
        // next run-loop iteration rather than synchronously during init.
        // This matches real iOS timing behavior — delegates are never
        // called during the initializer itself.
        log_dbg!(
            "NSURLConnection: scheduling deferred success notification \
             (faking network connectivity in touchHLE)"
        );
        let sel = env.objc.register_host_selector("_touchHLE_deliverSuccess".to_string(), &mut env.mem);
        () = msg![env; this performSelector:sel withObject:nil afterDelay:0.0_f64];
    }

    this
}

// Internal helper method: delivers the success callback to the delegate.
- (())_touchHLE_deliverSuccess {
    let host = env.objc.borrow::<NSURLConnectionHostObject>(this);
    if host.cancelled {
        return;
    }
    let delegate = host.delegate;
    if delegate == nil {
        return;
    }
    notify_delegate_success(env, this, delegate, nil);
}

// MARK: - Instance methods

- (())start {
    log_dbg!(
        "NSURLConnection start: scheduling deferred success \
         (faking network connectivity in touchHLE)"
    );
    let sel = env.objc.register_host_selector("_touchHLE_deliverSuccess".to_string(), &mut env.mem);
    () = msg![env; this performSelector:sel withObject:nil afterDelay:0.0_f64];
}

- (())cancel {
    log_dbg!("NSURLConnection cancel");
    env.objc
        .borrow_mut::<NSURLConnectionHostObject>(this)
        .cancelled = true;
}

// MARK: - Dealloc

- (())dealloc {
    log_dbg!("NSURLConnection dealloc");
    let delegate = env.objc
        .borrow::<NSURLConnectionHostObject>(this)
        .delegate;
    release(env, delegate);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};

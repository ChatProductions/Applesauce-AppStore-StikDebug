/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSURLConnection`.

use crate::mem::MutPtr;
use crate::objc::{autorelease, id, msg, msg_class, nil, objc_classes, release, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURLConnection: NSObject

+ (id)sendSynchronousRequest:(id)request
           returningResponse:(MutPtr<id>)response
                       error:(MutPtr<id>)error {
    log!("TODO: [NSURLConnection sendSynchronousRequest:returningResponse:error:] stubbed");
    
    // Если игра передала указатель для ответа, записываем туда nil
    if !response.is_null() {
        env.mem.write(response, nil);
    }
    
    // Если игра передала указатель для ошибки, тоже записываем nil
    if !error.is_null() {
        env.mem.write(error, nil);
    }
    
    // Создаем пустой объект NSData, чтобы игра думала, что запрос успешен, но данных нет
    let data: id = msg_class![env; NSData alloc];
    let data: id = msg![env; data init];
    autorelease(env, data)
}

+ (id)connectionWithRequest:(id)request // NSURLRequest *
                   delegate:(id)delegate {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRequest:request delegate:delegate];
    autorelease(env, new)
}

- (id)initWithRequest:(id)request // NSURLRequest *
             delegate:(id)delegate {
    msg![env; this initWithRequest:request delegate:delegate startImmediately:true]
}

- (id)initWithRequest:(id)request // NSURLRequest *
             delegate:(id)delegate
     startImmediately:(bool)start_immediately {
    log!(
        "TODO: [(NSURLConnection *){:?} initWithRequest:{:?} delegate:{:?} startImmediately:{}]",
        this,
        request,
        delegate,
        start_immediately,
    );
    release(env, this);
    nil
}

@end

};

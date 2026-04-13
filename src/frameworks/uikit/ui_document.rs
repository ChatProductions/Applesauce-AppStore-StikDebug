/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIDocument`.

use crate::abi::GuestFunction;
use crate::frameworks::foundation::NSUInteger;
use crate::mem::Ptr;
use crate::objc::{
    id, impl_HostObject, msg, msg_class, nil, objc_classes, release, retain, ClassExports, NSZonePtr,
};
use crate::Environment;

pub struct UIDocumentHostObject {
    file_url: id,
}
impl_HostObject!(UIDocumentHostObject);

impl Default for UIDocumentHostObject {
    fn default() -> Self {
        Self { file_url: nil }
    }
}

// Вспомогательная функция для честного вызова Objective-C блоков (Completion Handlers)
fn call_bool_block(env: &mut Environment, block: id, arg: bool) {
    if block != nil {
        let block_ptr = block.to_bits();
        // В 32-битной архитектуре iOS указатель на функцию invoke находится по смещению 12 байт в структуре блока
        let invoke_addr: u32 = env.mem.read(Ptr::from_bits(block_ptr + 12));
        let invoke_func = GuestFunction::from_addr_with_thumb_bit(invoke_addr);
        
        // Первый аргумент блока — всегда сам блок, затем идут пользовательские аргументы
        let _: () = invoke_func.call_from_host(env, (block, arg as u32));
    }
}

pub const CLASSES: ClassExports = objc_classes! {
    (env, this, _cmd);

    @implementation UIDocument: NSObject

    + (id)allocWithZone:(NSZonePtr)_zone {
        let host_object = Box::<UIDocumentHostObject>::default();
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    - (id)initWithFileURL:(id)url {
        let this: id = msg![env; super init];
        if url != nil {
            retain(env, url);
        }
        env.objc.borrow_mut::<UIDocumentHostObject>(this).file_url = url;
        this
    }

    - (())dealloc {
        let url = env.objc.borrow::<UIDocumentHostObject>(this).file_url;
        if url != nil {
            release(env, url);
        }
        msg![env; super dealloc]
    }

    - (id)fileURL {
        env.objc.borrow::<UIDocumentHostObject>(this).file_url
    }

    - (NSUInteger)documentState {
        0 // UIDocumentStateNormal (0) означает, что документ открыт и готов к работе
    }

    - (())openWithCompletionHandler:(id)completionHandler {
        // Сообщаем игре, что документ успешно открыт
        call_bool_block(env, completionHandler, true);
    }

    - (())closeWithCompletionHandler:(id)completionHandler {
        // Сообщаем игре, что документ успешно закрыт
        call_bool_block(env, completionHandler, true);
    }

    - (())saveToURL:(id)_url forSaveOperation:(NSUInteger)_operation completionHandler:(id)completionHandler {
        // Сообщаем игре, что сохранение прошло успешно
        call_bool_block(env, completionHandler, true);
    }

    - (())updateChangeCount:(NSUInteger)_change {
        // Требуется игрой для отметки изменений, в базовой реализации просто принимаем
    }
    
    - (id)localizedName {
        let url = env.objc.borrow::<UIDocumentHostObject>(this).file_url;
        if url != nil {
            msg![env; url lastPathComponent]
        } else {
            nil
        }
    }

    @end
};

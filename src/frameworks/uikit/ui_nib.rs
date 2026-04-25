/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//!
//! `UINib` and loading of nib files.
//!
//! Resources:
//! - Apple's [Resource Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/LoadingResources/CocoaNibs/CocoaNibs.html) is very helpful.
//!
//! - GitHub user 0xced's [reverse-engineering of UIClassSwapper](https://gist.github.com/0xced/45daf79b62ad6a20be1c).

use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::frameworks::foundation::{ns_string, NSUInteger};
use crate::frameworks::uikit::ui_view::ui_control::UIControlEvents;
use crate::fs::GuestPathBuf;
use crate::mem::ConstVoidPtr;
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, retain, Class, ClassExports, HostObject,
};
use crate::Environment;

#[derive(Default)]
struct UINibHostObject {
    /// `NSString*`
    nib_name: id,
    /// `NSBundle*`
    bundle: id,
    /// File's Owner (weak, non-retaining)
    file_owner: id,
}
impl HostObject for UINibHostObject {}

#[derive(Default)]
struct UIRuntimeConnectionHostObject {
    destination: id,
    label: id,
    source: id,
}
impl HostObject for UIRuntimeConnectionHostObject {}

#[derive(Default)]
struct UIRuntimeEventConnectionHostObject {
    superclass: UIRuntimeConnectionHostObject,
    event_mask: UIControlEvents,
}
impl_HostObject_with_superclass!(UIRuntimeEventConnectionHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UINib: NSObject

+ (id)nibWithNibName:(id)nib_name bundle:(id)bundle {
    let main_bundle = msg_class![env; NSBundle mainBundle];
    let bundle: id = if bundle == nil {
        main_bundle
    } else {
        // TODO: non-main bundles
        assert_eq!(bundle, main_bundle);
        bundle
    };

    retain(env, nib_name);
    retain(env, bundle);
    let host_object = Box::new(UINibHostObject {
        nib_name,
        bundle,
        file_owner: nil
    });
    
    let new = env.objc.alloc_object(this, host_object, &mut env.mem);
    autorelease(env, new)
}

- (())dealloc {
    let &UINibHostObject {
        nib_name,
        bundle,
        ..
    } = env.objc.borrow(this);
    
    release(env, nib_name);
    release(env, bundle);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)instantiateWithOwner:(id)owner options:(id)options {
        let UINibHostObject { nib_name, bundle, .. } = *env.objc.borrow(this);
        // Если bundle не указан, по правилам iOS используется mainBundle
        let target_bundle = if bundle != nil { bundle } else { msg_class![env; NSBundle mainBundle] };
        
        let ext = get_static_str(env, "nib");
        
        // 1. Ищем стандартный путь через NSBundle (с расширением "nib")
        let mut path: id = msg![env; target_bundle pathForResource:nib_name ofType:ext];
        
        // 2. Если не нашли, вдруг игра передала имя уже с расширением (ofType: nil)
        if path == nil {
            path = msg![env; target_bundle pathForResource:nib_name ofType:nil];
        }
        
        if path == nil {
            let name_str = to_rust_string(env, nib_name);
            log!("touchHLE::frameworks::uikit::ui_nib: Warning: UINib instantiateWithOwner: nib file {:?} not found", name_str);
            // Возвращаем пустой массив вместо nil, чтобы не уронить игру, если она делает итерацию
            return msg_class![env; NSArray array]; 
        }

        // 3. Пытаемся прочитать данные честным путем.
        let mut data: id = msg_class![env; NSData dataWithContentsOfFile:path];
        
        // Если data == nil, скорее всего путь указывает на директорию.
        // Заглядываем внутрь и ищем keyedobjects.nib (современный формат)
        if data == nil {
            let keyed = get_static_str(env, "keyedobjects.nib");
            let path_keyed: id = msg![env; path stringByAppendingPathComponent:keyed];
            data = msg_class![env; NSData dataWithContentsOfFile:path_keyed];
        }

        // Запасной вариант для очень старых версий формата XIB/NIB
        if data == nil {
            let designable = get_static_str(env, "designable.nib");
            let path_des: id = msg![env; path stringByAppendingPathComponent:designable];
            data = msg_class![env; NSData dataWithContentsOfFile:path_des];
        }

        if data == nil {
            log!("touchHLE::frameworks::uikit::ui_nib: Warning: Could not read data from nib at path {:?}", to_rust_string(env, path));
            return msg_class![env; NSArray array];
        }

        // 4. Инициализируем unarchiver успешно найденными данными
        let unarchiver: id = msg_class![env; NSKeyedUnarchiver alloc];
        let unarchiver: id = msg![env; unarchiver initForReadingWithData:data];
        
        // ... Дальше оставляй свой текущий код без изменений! ...
        // (начиная с let objects_key = get_static_str(env, "UINibObjectsKey"); и так далее)
    
    let top_level_objects = if let Ok(unarchiver) = load_nib_file(env, this, GuestPathBuf::from(nib_path)) {
        let top_level_objects_key = get_static_str(env, "UINibTopLevelObjectsKey");
        let objects = msg![env; unarchiver decodeObjectForKey:top_level_objects_key];
        
        // Удерживаем объекты ДО удаления анрайхиватора, иначе они могут вычиститься
        if objects != nil {
            retain(env, objects);
        }
        release(env, unarchiver);
        
        if objects != nil {
            autorelease(env, objects)
        } else {
            nil
        }
    } else {
        nil
    };
    
    env.objc.borrow_mut::<UINibHostObject>(this).file_owner = nil;
    top_level_objects
}

@end

@implementation UIProxyObject: NSObject

- (id)initWithCoder:(id)coder {
    let id_key = get_static_str(env, "UIProxiedObjectIdentifier");
    let id_nss: id = msg![env; coder decodeObjectForKey:id_key];
    let id = to_rust_string(env, id_nss);
    
    if id == "IBFilesOwner" {
        let delegate: id = msg![env; coder delegate];
        if delegate != nil {
            let file_owner = env.objc.borrow::<UINibHostObject>(delegate).file_owner;
            if file_owner != nil {
                return file_owner;
            }
        }
        
        log!("touchHLE Warning: IBFilesOwner requested but file_owner is nil! Returning dummy.");
        let ns_object_class = env.objc.get_known_class("NSObject", &mut env.mem);
        let dummy: id = msg![env; ns_object_class alloc];
        msg![env; dummy init]
    } else if id == "IBFirstResponder" {
        log!("touchHLE: Bypassing IBFirstResponder replacement with dummy NSObject");
        let proxy_class = env.objc.get_known_class("NSObject", &mut env.mem);
        let dummy: id = msg![env; proxy_class alloc];
        let dummy_init: id = msg![env; dummy init];
        release(env, this);
        dummy_init
    } else {
        log!("TODO: UIProxyObject replacement for {}, instance {:?} left unreplaced", id, this);
        this
    }
}

@end

@implementation UIClassSwapper: NSObject

- (id)initWithCoder:(id)coder {
    let name_key = get_static_str(env, "UIClassName");
    let name_nss: id = msg![env; coder decodeObjectForKey:name_key];
    let name = to_rust_string(env, name_nss);

    let orig_key = get_static_str(env, "UIOriginalClassName");
    let orig_nss: id = msg![env; coder decodeObjectForKey:orig_key];
    let orig = to_rust_string(env, orig_nss);

    log!("[DEBUG NIB] UIClassSwapper loading class: {} (original: {})", name, orig);
    
    // Блок для определения подменного класса без ворнингов на лишний `mut`
    let selected_class = {
        let mut c = env.objc.get_known_class(&name, &mut env.mem);
        if c == nil {
            log!("[DEBUG NIB] Warning: Custom class {} not found. Falling back to original: {}", name, orig);
            c = env.objc.get_known_class(&orig, &mut env.mem);
        }
        
        let problematic_views = ["FBLoginButton"];
        if c == nil || problematic_views.iter().any(|&prob| name == prob) {
            log!("[DEBUG NIB] Warning: Substituting {} with generic UIView", name);
            c = env.objc.get_known_class("UIView", &mut env.mem);
        }

        if c == nil {
            log!("[DEBUG NIB] CRITICAL: Fallback class not found! Falling back to NSObject.");
            c = env.objc.get_known_class("NSObject", &mut env.mem);
        }
        c
    };

    let object: id = msg![env; selected_class alloc];
    
    // ВАЖНО: Всегда используем initWithCoder:, кроме тех случаев, когда это чисто кастомный плейсхолдер Interface Builder
    // Инициализация системных UIViewController через 'init' оставляет их сломанными и ведет к NULL-PAGE READ.
    let object: id = if orig == "UICustomObject" {
        msg![env; object init]
    } else {
        msg![env; object initWithCoder:coder]
    };
    
    release(env, this);
    object
}

@end

@implementation UIRuntimeConnection: NSObject

+ (id)alloc {
    let host_object = Box::<UIRuntimeConnectionHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let destination_key = get_static_str(env, "UIDestination");
    let destination: id = msg![env; coder decodeObjectForKey: destination_key];

    let label_key = get_static_str(env, "UILabel");
    let label: id = msg![env; coder decodeObjectForKey: label_key];

    let source_key = get_static_str(env, "UISource");
    let source: id = msg![env; coder decodeObjectForKey: source_key];

    retain(env, destination);
    retain(env, source);
    retain(env, label);
    
    let host_obj = env.objc.borrow_mut::<UIRuntimeConnectionHostObject>(this);
    host_obj.destination = destination;
    host_obj.label = label;
    host_obj.source = source;
    this
}

- (())dealloc {
    let &UIRuntimeConnectionHostObject { destination, label, source } = env.objc.borrow(this);
    release(env, destination);
    release(env, label);
    release(env, source);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

@implementation UIRuntimeEventConnection: UIRuntimeConnection

+ (id)alloc {
    let host_object = Box::<UIRuntimeEventConnectionHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())connect {
    let &UIRuntimeConnectionHostObject { destination, label, source } = env.objc.borrow(this);
    let &UIRuntimeEventConnectionHostObject { superclass: _, event_mask } = env.objc.borrow(this);
    
    if source != nil && destination != nil && label != nil {
        let selector = to_rust_string(env, label);
        if let Some(action) = env.objc.lookup_selector(&selector) {
            () = msg![env; source addTarget:destination action:action forControlEvents:event_mask];
        } else {
            log!("touchHLE Warning: UIRuntimeEventConnection missing selector '{}', skipping.", selector);
        }
    }
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder: coder];
    let event_mask_key = get_static_str(env, "UIEventMask");
    let event_mask: i32 = msg![env; coder decodeIntForKey: event_mask_key];

    let host_obj = env.objc.borrow_mut::<UIRuntimeEventConnectionHostObject>(this);
    host_obj.event_mask = event_mask as UIControlEvents;
    this
}

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

@implementation UIRuntimeOutletConnection: UIRuntimeConnection

- (())connect {
    let &UIRuntimeConnectionHostObject { destination, label, source } = env.objc.borrow(this);
    
    if source != nil && destination != nil && label != nil {
        // ЯВНО УКАЗЫВАЕМ ТИП Class, чтобы компилятор не ругался
        let source_class: Class = msg![env; source class];
        let ns_object_class = env.objc.get_known_class("NSObject", &mut env.mem);
        
        // Предотвращаем краш KVC (Key-Value Coding), если source — это просто заглушка NSObject
        if source_class == ns_object_class {
            let label_str = to_rust_string(env, label);
            log!("touchHLE NIB: Skipping outlet '{}' connection because source is an unhandled NSObject", label_str);
            return;
        }

        () = msg![env; source setValue:destination forKey:label];
    }
}

@end

};

fn load_nib_file(env: &mut Environment, ui_nib: id, path: GuestPathBuf) -> Result<id, ()> {
    let path_str = ns_string::from_rust_string(env, path.as_str().to_string());
    let ns_data: id = msg_class![env; NSData dataWithContentsOfFile:path_str];
    
    if ns_data == nil {
        log!("Warning: couldn't load nib file {:?}", path);
        return Err(());
    };

    let len: NSUInteger = msg![env; ns_data length];
    // Если длина файла достаточна, значит указатель bytes гарантированно существует
    if len < 10 {
        return Err(());
    }
    
    let bytes: ConstVoidPtr = msg![env; ns_data bytes];

    let unarchiver = if env.mem.bytes_at(bytes.cast(), 10) == b"NIBArchive" {
        let decoder: id = msg_class![env; _touchHLE_NIBArchiveDecoder alloc];
        msg![env; decoder _touchHLE_initForReadingWithData:ns_data]
    } else {
        let unarchiver = msg_class![env; NSKeyedUnarchiver alloc];
        msg![env; unarchiver initForReadingWithData:ns_data]
    };

    () = msg![env; unarchiver setDelegate:ui_nib];

    let objects_key = get_static_str(env, "UINibObjectsKey");
    let objects: id = msg![env; unarchiver decodeObjectForKey:objects_key];
    
    if objects != nil {
        retain(env, objects); // Защита от преждевременного удаления
    }
    
    let conns_key = get_static_str(env, "UINibConnectionsKey");
    let conns: id = msg![env; unarchiver decodeObjectForKey:conns_key];
    if conns != nil {
        let conns_count: NSUInteger = msg![env; conns count];
        for i in 0..conns_count {
            let conn: id = msg![env; conns objectAtIndex:i];
            if conn != nil {
                () = msg![env; conn connect];
            }
        }
    }

    if objects != nil {
        let enumerator: id = msg![env; objects objectEnumerator];
        if enumerator != nil {
            loop {
                let next: id = msg![env; enumerator nextObject];
                if next == nil {
                    break;
                }
                () = msg![env; next awakeFromNib];
            }
        }
    }

    let visibles_key = get_static_str(env, "UINibVisibleWindowsKey");
    let visibles: id = msg![env; unarchiver decodeObjectForKey:visibles_key];
    if visibles != nil {
        let visibles_count: NSUInteger = msg![env; visibles count];
        for i in 0..visibles_count {
            let visible: id = msg![env; visibles objectAtIndex:i];
            if visible != nil {
                () = msg![env; visible setHidden:false];
            }
        }
    }

    Ok(unarchiver)
}


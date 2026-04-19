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
    assert!(owner != nil);
    // TODO: implement options handling
    assert!(options == nil);

    let bundle = env.objc.borrow::<UINibHostObject>(this).bundle;
    let nib_name = env.objc.borrow::<UINibHostObject>(this).nib_name;
    let type_: id = get_static_str(env, "nib");
    let path: id = msg![env; bundle pathForResource:nib_name ofType:type_];

    if path == nil {
        log!("Warning: UINib instantiateWithOwner: nib file {:?} not found", to_rust_string(env, nib_name));
        return nil;
    }
    
    let nib_path = to_rust_string(env, path).to_string();
    assert!(env.objc.borrow::<UINibHostObject>(this).file_owner == nil);
    env.objc.borrow_mut::<UINibHostObject>(this).file_owner = owner;
    
    let top_level_objects = if let Ok(unarchiver) = load_nib_file(env, this, GuestPathBuf::from(nib_path)) {
        let top_level_objects_key = get_static_str(env, "UINibTopLevelObjectsKey");
        let objects = msg![env; unarchiver decodeObjectForKey:top_level_objects_key];
        release(env, unarchiver);
        objects
    } else {
        nil
    };
    
    env.objc.borrow_mut::<UINibHostObject>(this).file_owner = nil;
    top_level_objects
}

@end

// An undocumented type that nib files reference by name.
@implementation UIProxyObject: NSObject

- (id)initWithCoder:(id)coder {
    let id_key = get_static_str(env, "UIProxiedObjectIdentifier");
    let id_nss: id = msg![env; coder decodeObjectForKey:id_key];
    let id = to_rust_string(env, id_nss);
    
    if id == "IBFilesOwner" {
        let delegate: id = msg![env; coder delegate];
        assert!(delegate != nil);
        let ui_nib_class: Class = msg_class![env; UINib class];
        let delegate_class: Class = msg![env; delegate class];
        assert!(msg![env; delegate_class isKindOfClass:ui_nib_class]);
        
        let file_owner = env.objc.borrow::<UINibHostObject>(delegate).file_owner;
        if file_owner != nil {
            file_owner
        } else {
            log!("touchHLE Warning: IBFilesOwner requested but file_owner is nil! Returning dummy.");
            let ns_object_class = env.objc.get_known_class("NSObject", &mut env.mem);
            let dummy: id = msg![env; ns_object_class alloc];
            msg![env; dummy init]
        }
    } else if id == "IBFirstResponder" {
        log!("touchHLE: Bypassing IBFirstResponder replacement with dummy UIResponder/NSObject");
        // Пытаемся использовать UIResponder, если он доступен, иначе NSObject
        let mut proxy_class = env.objc.get_known_class("UIResponder", &mut env.mem);
        if proxy_class == nil {
            proxy_class = env.objc.get_known_class("NSObject", &mut env.mem);
        }
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

// Another undocumented type used by nib files.
@implementation UIClassSwapper: NSObject

- (id)initWithCoder:(id)coder {
    let name_key = get_static_str(env, "UIClassName");
    let name_nss: id = msg![env; coder decodeObjectForKey:name_key];
    let name = to_rust_string(env, name_nss);

    let orig_key = get_static_str(env, "UIOriginalClassName");
    let orig_nss: id = msg![env; coder decodeObjectForKey:orig_key];
    let orig = to_rust_string(env, orig_nss);

    log!("[DEBUG NIB] UIClassSwapper loading class: {} (original: {})", name, orig);
    
    // БЕЗОПАСНАЯ ЦЕПОЧКА ФОЛБЕКОВ
    let mut class = env.objc.get_known_class(&name, &mut env.mem);
    
    // 1. Если кастомный класс из приложения не найден, используем базовый класс
    if class == nil {
        log!("[DEBUG NIB] Warning: Custom class {} not found. Falling back to original: {}", name, orig);
        class = env.objc.get_known_class(&orig, &mut env.mem);
    }
    
    // 2. Хак для проблемных View (избегаем крашей на специфичных контролах вроде FBLoginButton)
    let problematic_views = ["FBLoginButton"];
    if class == nil || problematic_views.iter().any(|&c| name == c) {
        log!("[DEBUG NIB] Warning: Substituting {} with generic UIView to ensure compatibility", name);
        class = env.objc.get_known_class("UIView", &mut env.mem);
    }

    // 3. Последний рубеж защиты (если даже UIView не проинициализирован должным образом)
    if class == nil {
        log!("[DEBUG NIB] CRITICAL: Fallback class not found! Falling back to NSObject to prevent NULL-PAGE READ.");
        class = env.objc.get_known_class("NSObject", &mut env.mem);
    }

    let object: id = msg![env; class alloc];
    let object: id = if orig == "UICustomObject" {
        msg![env; object init]
    } else {
        // UIViewController, UITableViewController и другие наследуемые вьюхи поддерживают initWithCoder
        msg![env; object initWithCoder:coder]
    };
    
    release(env, this);
    object
}

@end

// Connects outlets once objects are deserialized.
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
    let &UIRuntimeConnectionHostObject {
        destination,
        label,
        source
    } = env.objc.borrow(this);
    
    release(env, destination);
    release(env, label);
    release(env, source);

    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// Connects events (Actions).
@implementation UIRuntimeEventConnection: UIRuntimeConnection

+ (id)alloc {
    let host_object = Box::<UIRuntimeEventConnectionHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())connect {
    let &UIRuntimeConnectionHostObject {
        destination,
        label,
        source
    } = env.objc.borrow(this);
    let &UIRuntimeEventConnectionHostObject {
        superclass: _,
        event_mask
    } = env.objc.borrow(this);
    
    let selector = to_rust_string(env, label);
    
    // БЕЗОПАСНЫЙ ПОИСК СЕЛЕКТОРА (убрано .unwrap(), чтобы не было паники на неизвестных экшенах)
    if let Some(action) = env.objc.lookup_selector(&selector) {
        () = msg![env; source addTarget:destination action:action forControlEvents:event_mask];
    } else {
        log!("touchHLE Warning: UIRuntimeEventConnection missing selector '{}', skipping connection.", selector);
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

// Connects runtime outlets.
@implementation UIRuntimeOutletConnection: UIRuntimeConnection

- (())connect {
    let &UIRuntimeConnectionHostObject {
        destination,
        label,
        source
    } = env.objc.borrow(this);
    () = msg![env; source setValue:destination forKey:label];
}

@end

};

/// Takes a [GuestPathBuf] where a nib file is located and deserializes it.
fn load_nib_file(env: &mut Environment, ui_nib: id, path: GuestPathBuf) -> Result<id, ()> {
    let path_str = ns_string::from_rust_string(env, path.as_str().to_string());
    let ns_data: id = msg_class![env; NSData dataWithContentsOfFile:path_str];
    
    if ns_data == nil {
        log!("Warning: couldn't load nib file {:?}", path);
        return Err(());
    };

    let len: NSUInteger = msg![env; ns_data length];
    assert!(len >= 10);
    
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
    
    // --- ХАК ДЛЯ ПРЕДОТВРАЩЕНИЯ ВЫЛЕТОВ ПРИ КАСАНИИ ---
    // Удерживаем все объекты NIB-файла в памяти. Во многих играх (и старом iOS коде)
    // корневые объекты NIB не удерживаются должным образом после загрузки.
    if objects != nil {
        retain(env, objects);
    }
    // --------------------------------------------------
    
    // Подключаем Outlets
    let conns_key = get_static_str(env, "UINibConnectionsKey");
    let conns: id = msg![env; unarchiver decodeObjectForKey:conns_key];
    if conns != nil {
        let conns_count: NSUInteger = msg![env; conns count];
        for i in 0..conns_count {
            let conn: id = msg![env; conns objectAtIndex:i];
            () = msg![env; conn connect];
        }
    }

    // Рассылаем awakeFromNib
    if objects != nil {
        let enumerator: id = msg![env; objects objectEnumerator];
        loop {
            let next: id = msg![env; enumerator nextObject];
            if next == nil {
                break;
            }
            // Вызываем awakeFromNib. Классы iOS 2-4 неявно поддерживают его через NSObject
            () = msg![env; next awakeFromNib];
        }
    }

    // Показываем видимые окна (UIWindow)
    let visibles_key = get_static_str(env, "UINibVisibleWindowsKey");
    let visibles: id = msg![env; unarchiver decodeObjectForKey:visibles_key];
    if visibles != nil {
        let visibles_count: NSUInteger = msg![env; visibles count];
        for i in 0..visibles_count {
            let visible: id = msg![env; visibles objectAtIndex:i];
            () = msg![env; visible setHidden:false];
        }
    }

    Ok(unarchiver)
}


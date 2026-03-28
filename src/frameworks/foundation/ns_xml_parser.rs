/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! NSXMLParser.

use super::ns_string::from_rust_string; // Убрал to_rust_string
use super::NSUInteger;
use crate::environment::Environment;
use crate::mem::ConstVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, todo_objc_setter,
    ClassExports, HostObject, NSZonePtr, SEL,
};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

struct NSXMLParserHostObject {
    data: id,
    delegate: id,
}
impl HostObject for NSXMLParserHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSXMLParser: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSXMLParserHostObject {
        data: nil,
        delegate: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithContentsOfURL:(id)url { // NSURL *
    let data: id = msg_class![env; NSData dataWithContentsOfURL:url];
    msg![env; this initWithData:data]
}

- (id)initWithData:(id)data { // NSData *
    retain(env, data);
    env.objc.borrow_mut::<NSXMLParserHostObject>(this).data = data;
    this
}

- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<NSXMLParserHostObject>(this).delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<NSXMLParserHostObject>(this).delegate
}

- (())setShouldResolveExternalEntities:(bool)should {
    todo_objc_setter!(this, should);
}
- (())setShouldProcessNamespaces:(bool)should {
    todo_objc_setter!(this, should);
}
- (())setShouldReportNamespacePrefixes:(bool)should {
    todo_objc_setter!(this, should);
}

- (bool)parse {
    let data = env.objc.borrow::<NSXMLParserHostObject>(this).data;
    
    // Вместо вылета с паникой просто возвращаем false, если данных нет.
    if data == nil {
        log!("NSXMLParser: data is nil, cannot parse!");
        return false;
    }

    let bytes: ConstVoidPtr = msg![env; data bytes];
    let length: NSUInteger = msg![env; data length];
    
    if length == 0 {
        return false;
    }

    log_dbg!("Parsing XML data...");
    let bytes: &[u8] = env.mem.bytes_at_mut(bytes.cast().cast_mut(), length);

    let mut reader = Reader::from_reader(bytes);
    let mut events = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(e) => events.push(e.into_owned()),
            Err(e) => {
                log!("XML Parse Error at position {}: {:?}", 
                     reader.error_position(), e);
                return false;
            },
        }
    }

    let delegate = env.objc.borrow::<NSXMLParserHostObject>(this).delegate;
    let sel: SEL = env
        .objc
        .register_host_selector("parserDidStartDocument:".to_string(), &mut env.mem);
    let responds: bool = msg![env; delegate respondsToSelector:sel];
    if responds {
        () = msg![env; delegate parserDidStartDocument:this];
    }
    for event in events {
        match event {
            Event::Empty(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let name: id = from_rust_string(env, name);
                let name = autorelease(env, name);
                let sel: SEL = env
                    .objc
                    .register_host_selector(
                        "parser:didStartElement:namespaceURI:qualifiedName:attributes:".to_string(),
                        &mut env.mem
                    );
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    let dict = build_attributes_dict(env, e);
                    () = msg![env; delegate parser:this
                                   didStartElement:name
                                      namespaceURI:nil
                                     qualifiedName:nil
                                        attributes:dict];
                }
                let sel: SEL = env
                    .objc
                    .register_host_selector(
                        "parser:didEndElement:namespaceURI:qualifiedName:".to_string(),
                        &mut env.mem
                    );
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    () = msg![env; delegate parser:this
                                     didEndElement:name
                                      namespaceURI:nil
                                     qualifiedName:nil];
                }
            }
            Event::Text(e) => {
                let text = e.decode().unwrap().to_string();
                if text != "\0" {
                    let sel: SEL = env
                        .objc
                        .register_host_selector("parser:foundCharacters:".to_string(), &mut env.mem);
                    let responds: bool = msg![env; delegate respondsToSelector:sel];
                    if responds {
                        let chars = from_rust_string(env, text);
                        let chars = autorelease(env, chars);
                        () = msg![env; delegate parser:this foundCharacters:chars];
                    }
                }
            }
            Event::Start(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let sel: SEL = env
                    .objc
                    .register_host_selector(
                        "parser:didStartElement:namespaceURI:qualifiedName:attributes:".to_string(),
                        &mut env.mem
                    );
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    let name: id = from_rust_string(env, name);
                    let name = autorelease(env, name);
                    let dict = build_attributes_dict(env, e);
                    () = msg![env; delegate parser:this
                                   didStartElement:name
                                      namespaceURI:nil
                                     qualifiedName:nil
                                        attributes:dict];
                }
            }
            Event::End(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let sel: SEL = env
                    .objc
                    .register_host_selector(
                        "parser:didEndElement:namespaceURI:qualifiedName:".to_string(),
                        &mut env.mem
                    );
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    let name: id = from_rust_string(env, name);
                    let name = autorelease(env, name);
                    () = msg![env; delegate parser:this
                                     didEndElement:name
                                      namespaceURI:nil
                                     qualifiedName:nil];
                }
            }
            Event::CData(e) => {
                let text = e.decode().unwrap().to_string();
                let sel: SEL = env
                    .objc
                    .register_host_selector("parser:foundCharacters:".to_string(), &mut env.mem);
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    let text = from_rust_string(env, text);
                    let text = autorelease(env, text);
                    () = msg![env; delegate parser:this foundCharacters:text];
                }
            }
            Event::Comment(e) => {
                let comment = e.decode().unwrap().to_string();
                let sel: SEL = env
                    .objc
                    .register_host_selector("parser:foundComment:".to_string(), &mut env.mem);
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    let comment = from_rust_string(env, comment);
                    let comment = autorelease(env, comment);
                    () = msg![env; delegate parser:this foundComment:comment];
                }
            }
            _ => {} 
        }
    }
    let sel: SEL = env
        .objc
        .register_host_selector("parserDidEndDocument:".to_string(), &mut env.mem);
    let responds: bool = msg![env; delegate parserDidEndDocument:this];
    if responds {
        () = msg![env; delegate parserDidEndDocument:this];
    }
    true
}

- (())dealloc {
    let &NSXMLParserHostObject { data, .. } = env.objc.borrow(this);
    release(env, data);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};

fn build_attributes_dict(env: &mut Environment, e: BytesStart) -> id {
    let pairs = e.attributes().map(|a| a.unwrap()).map(|a| {
        (
            String::from_utf8(a.key.local_name().as_ref().to_vec()).unwrap(),
            a.unescape_value().unwrap().to_string(),
        )
    });
    let dict: id = msg_class![env; NSMutableDictionary new];
    for (x, y) in pairs {
        let key = from_rust_string(env, x);
        let val = from_rust_string(env, y);
        () = msg![env; dict setObject:val forKey:key];
        release(env, key);
        release(env, val);
    }
    autorelease(env, dict)
}


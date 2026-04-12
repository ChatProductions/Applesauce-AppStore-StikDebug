use crate::dyld::{export_c_func, ConstantExports, FunctionExports};
use crate::Environment;

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/usr/lib/libxml2.2.dylib",
    aliases: &["/usr/lib/libxml2.dylib"],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[FUNCTIONS],
};

const FUNCTIONS: FunctionExports = &[
    // --- существующие ---
    export_c_func!(xmlNewParserCtxt()),
    export_c_func!(xmlClearParserCtxt()),
    export_c_func!(xmlCtxtReadMemory(_, _, _, _, _, _)),
    export_c_func!(xmlReadFile(_, _, _)),
    export_c_func!(xmlReadMemory(_, _, _, _, _)),
    export_c_func!(xmlParseMemory(_, _)),
    export_c_func!(xmlDocGetRootElement(_)),
    export_c_func!(xmlFree(_)),
    export_c_func!(xmlCtxtGetLastError(_)),
    export_c_func!(xmlFreeDoc(_)),
    export_c_func!(xmlCleanupParser()),
    export_c_func!(xmlFreeParserCtxt(_)),
    export_c_func!(xmlGetProp(_, _)),
    export_c_func!(xmlHasProp(_, _)),
    export_c_func!(xmlStrcmp(_, _)),
    export_c_func!(xmlNodeGetContent(_)),
    // --- TextReader API ---
    export_c_func!(xmlReaderForFile(_, _, _)),
    export_c_func!(xmlFreeTextReader(_)),
    export_c_func!(xmlTextReaderRead(_)),
    export_c_func!(xmlTextReaderNodeType(_)),
    export_c_func!(xmlTextReaderConstName(_)),
    export_c_func!(xmlTextReaderConstValue(_)),
    export_c_func!(xmlTextReaderReadInnerXml(_)),
    export_c_func!(xmlTextReaderReadOuterXml(_)),
    export_c_func!(xmlTextReaderGetAttribute(_, _)),
    export_c_func!(xmlTextReaderAttributeCount(_)),
    export_c_func!(xmlTextReaderDepth(_)),
    export_c_func!(xmlTextReaderHasAttributes(_)),
    export_c_func!(xmlTextReaderIsEmptyElement(_)),
    export_c_func!(xmlTextReaderNext(_)),
    export_c_func!(xmlTextReaderNextSibling(_)),
    export_c_func!(xmlTextReaderValue(_)),
    export_c_func!(xmlTextReaderLocalName(_)),
    export_c_func!(xmlTextReaderPrefix(_)),
    export_c_func!(xmlTextReaderNamespaceUri(_)),
    export_c_func!(xmlStrdup(_)),
    export_c_func!(xmlStrlen(_)),
    export_c_func!(xmlStrsub(_, _, _)),
];

// ============================================================
// Хелперы
// ============================================================

fn alloc_xml_mem(env: &mut Environment) -> u32 {
    let size = 512u32;
    let ptr: crate::mem::MutPtr<u8> = env.mem.alloc(size).cast();
    let slice = env.mem.bytes_at_mut(ptr, size);
    for byte in slice.iter_mut() {
        *byte = 0;
    }
    ptr.to_bits()
}

fn alloc_empty_string(env: &mut Environment) -> u32 {
    let ptr: crate::mem::MutPtr<u8> = env.mem.alloc(1).cast();
    env.mem.write(ptr, 0u8);
    ptr.to_bits()
}

// ============================================================
// Существующие функции
// ============================================================

#[allow(non_snake_case)]
fn xmlNewParserCtxt(env: &mut Environment) -> u32 {
    alloc_xml_mem(env)
}

#[allow(non_snake_case)]
fn xmlClearParserCtxt(_env: &mut Environment) {}

#[allow(non_snake_case)]
fn xmlCtxtReadMemory(
    env: &mut Environment,
    _ctxt: u32,
    _buf: u32,
    _sz: u32,
    _url: u32,
    _enc: u32,
    _opt: u32,
) -> u32 {
    alloc_xml_mem(env)
}

#[allow(non_snake_case)]
fn xmlReadFile(
    env: &mut Environment,
    _filename: u32,
    _encoding: u32,
    _options: u32,
) -> u32 {
    alloc_xml_mem(env)
}

#[allow(non_snake_case)]
fn xmlReadMemory(
    env: &mut Environment,
    _buffer: u32,
    _size: u32,
    _url: u32,
    _encoding: u32,
    _options: u32,
) -> u32 {
    alloc_xml_mem(env)
}

#[allow(non_snake_case)]
fn xmlParseMemory(env: &mut Environment, _buffer: u32, _size: u32) -> u32 {
    alloc_xml_mem(env)
}

#[allow(non_snake_case)]
fn xmlDocGetRootElement(env: &mut Environment, _doc: u32) -> u32 {
    alloc_xml_mem(env)
}

#[allow(non_snake_case)]
fn xmlFree(_env: &mut Environment, _ptr: u32) {}

#[allow(non_snake_case)]
fn xmlCtxtGetLastError(_env: &mut Environment, _ctxt: u32) -> u32 {
    0
}

#[allow(non_snake_case)]
fn xmlFreeDoc(_env: &mut Environment, _doc: u32) {}

#[allow(non_snake_case)]
fn xmlCleanupParser(_env: &mut Environment) {}

#[allow(non_snake_case)]
fn xmlFreeParserCtxt(_env: &mut Environment, _ctxt: u32) {}

#[allow(non_snake_case)]
fn xmlGetProp(_env: &mut Environment, _node: u32, _name: u32) -> u32 {
    0
}

#[allow(non_snake_case)]
fn xmlHasProp(_env: &mut Environment, _node: u32, _name: u32) -> u32 {
    0
}

#[allow(non_snake_case)]
fn xmlStrcmp(_env: &mut Environment, _str1: u32, _str2: u32) -> i32 {
    0
}

#[allow(non_snake_case)]
fn xmlNodeGetContent(_env: &mut Environment, _node: u32) -> u32 {
    0
}

// ============================================================
// XML TextReader API
// ============================================================

/// xmlTextReaderPtr xmlReaderForFile(const char *filename,
///                                    const char *encoding,
///                                    int options);
#[allow(non_snake_case)]
fn xmlReaderForFile(env: &mut Environment, filename: u32, _encoding: u32, _options: u32) -> u32 {
    let filename_str = if filename != 0 {
        env.mem
            .cstr_at_utf8(crate::mem::Ptr::from_bits(filename))
            .unwrap_or("<invalid>")
    } else {
        "<null>"
    };
    log!(
        "xmlReaderForFile(\"{}\") — returning stub reader",
        filename_str
    );
    alloc_xml_mem(env)
}

/// void xmlFreeTextReader(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlFreeTextReader(_env: &mut Environment, _reader: u32) {}

/// int xmlTextReaderRead(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderRead(_env: &mut Environment, _reader: u32) -> i32 {
    0
}

/// int xmlTextReaderNodeType(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderNodeType(_env: &mut Environment, _reader: u32) -> i32 {
    0
}

/// const xmlChar *xmlTextReaderConstName(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderConstName(env: &mut Environment, _reader: u32) -> u32 {
    alloc_empty_string(env)
}

/// const xmlChar *xmlTextReaderConstValue(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderConstValue(env: &mut Environment, _reader: u32) -> u32 {
    alloc_empty_string(env)
}

/// xmlChar *xmlTextReaderReadInnerXml(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderReadInnerXml(env: &mut Environment, _reader: u32) -> u32 {
    alloc_empty_string(env)
}

/// xmlChar *xmlTextReaderReadOuterXml(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderReadOuterXml(env: &mut Environment, _reader: u32) -> u32 {
    alloc_empty_string(env)
}

/// xmlChar *xmlTextReaderGetAttribute(xmlTextReaderPtr reader,
///                                     const xmlChar *name);
#[allow(non_snake_case)]
fn xmlTextReaderGetAttribute(env: &mut Environment, _reader: u32, _name: u32) -> u32 {
    alloc_empty_string(env)
}

/// int xmlTextReaderAttributeCount(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderAttributeCount(_env: &mut Environment, _reader: u32) -> i32 {
    0
}

/// int xmlTextReaderDepth(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderDepth(_env: &mut Environment, _reader: u32) -> i32 {
    0
}

/// int xmlTextReaderHasAttributes(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderHasAttributes(_env: &mut Environment, _reader: u32) -> i32 {
    0
}

/// int xmlTextReaderIsEmptyElement(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderIsEmptyElement(_env: &mut Environment, _reader: u32) -> i32 {
    1
}

/// int xmlTextReaderNext(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderNext(_env: &mut Environment, _reader: u32) -> i32 {
    0
}

/// int xmlTextReaderNextSibling(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderNextSibling(_env: &mut Environment, _reader: u32) -> i32 {
    0
}

/// const xmlChar *xmlTextReaderValue(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderValue(env: &mut Environment, _reader: u32) -> u32 {
    alloc_empty_string(env)
}

/// const xmlChar *xmlTextReaderLocalName(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderLocalName(env: &mut Environment, _reader: u32) -> u32 {
    alloc_empty_string(env)
}

/// const xmlChar *xmlTextReaderPrefix(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderPrefix(env: &mut Environment, _reader: u32) -> u32 {
    alloc_empty_string(env)
}

/// const xmlChar *xmlTextReaderNamespaceUri(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderNamespaceUri(env: &mut Environment, _reader: u32) -> u32 {
    alloc_empty_string(env)
}

// ============================================================
// Строковые утилиты libxml2
// ============================================================

/// xmlChar *xmlStrdup(const xmlChar *str);
#[allow(non_snake_case)]
fn xmlStrdup(env: &mut Environment, src_ptr: u32) -> u32 {
    if src_ptr == 0 {
        return 0;
    }
    let src = crate::mem::Ptr::from_bits(src_ptr);
    let cstr = env.mem.cstr_at_utf8(src).unwrap_or("");
    let len = cstr.len() + 1;
    let dst: crate::mem::MutPtr<u8> = env.mem.alloc(len).cast();
    for (i, &b) in cstr.as_bytes().iter().enumerate() {
        env.mem.write(dst + (i as u32), b);
    }
    env.mem.write(dst + (cstr.len() as u32), 0u8);
    dst.to_bits()
}

/// int xmlStrlen(const xmlChar *str);
#[allow(non_snake_case)]
fn xmlStrlen(env: &mut Environment, str_ptr: u32) -> i32 {
    if str_ptr == 0 {
        return 0;
    }
    let mut len: i32 = 0;
    let mut ptr = crate::mem::Ptr::<u8>::from_bits(str_ptr);
    loop {
        let byte: u8 = env.mem.read(ptr);
        if byte == 0 {
            break;
        }
        len += 1;
        ptr = ptr + 1u32;
    }
    len
}

/// xmlChar *xmlStrsub(const xmlChar *str, int start, int len);
#[allow(non_snake_case)]
fn xmlStrsub(env: &mut Environment, str_ptr: u32, start: u32, len: u32) -> u32 {
    if str_ptr == 0 {
        return 0;
    }
    let src = crate::mem::Ptr::from_bits(str_ptr + start);
    let dst: crate::mem::MutPtr<u8> = env.mem.alloc(len + 1).cast();
    for i in 0..len {
        let byte: u8 = env.mem.read(src + i);
        env.mem.write(dst + i, byte);
    }
    env.mem.write(dst + len, 0u8);
    dst.to_bits()
                     }

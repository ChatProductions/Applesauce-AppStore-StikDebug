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
    export_c_func!(xmlReadFile(_, _, _, _, _, _)),
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
    // --- НОВЫЕ: XML TextReader API ---
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
    export_c_func!(xmlTextReaderNodeType(_)),
    export_c_func!(xmlTextReaderValue(_)),
    export_c_func!(xmlTextReaderLocalName(_)),
    export_c_func!(xmlTextReaderPrefix(_)),
    export_c_func!(xmlTextReaderNamespaceUri(_)),
    export_c_func!(xmlStrdup(_)),
    export_c_func!(xmlStrlen(_)),
    export_c_func!(xmlStrsub(_, _, _)),
];

// ============================================================
// Хелпер для выделения памяти под фейковые XML-структуры
// ============================================================

fn alloc_xml_mem(env: &mut Environment) -> u32 {
    let size = 512;
    let ptr: crate::mem::MutPtr<u8> = env.mem.alloc(size).cast();
    let slice = env.mem.bytes_at_mut(ptr, size);
    for byte in slice.iter_mut() {
        *byte = 0;
    }
    ptr.to_bits()
}

/// Возвращает указатель на пустую строку (null-terminator) в guest memory
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
// XML TextReader API (stream-based parser)
// ============================================================

/// xmlTextReaderPtr xmlReaderForFile(const char *filename,
///                                    const char *encoding,
///                                    int options);
///
/// Создаёт reader для разбора XML-файла.
/// Возвращает NULL при ошибке, иначе — валидный указатель.
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
/// Возвращает 1 = есть узел, 0 = конец документа, -1 = ошибка.
#[allow(non_snake_case)]
fn xmlTextReaderRead(_env: &mut Environment, _reader: u32) -> i32 {
    0
}

/// int xmlTextReaderNodeType(xmlTextReaderPtr reader);
/// Типы: 1=element, 3=text, 14=whitespace, ...
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
fn xmlTextReaderReadInnerXml(env: &mut Environment, _reader: 

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
    // TextReader API
    export_c_func!(xmlReaderForFile(_, _, _)),
    export_c_func!(xmlFreeTextReader(_)),
    export_c_func!(xmlTextReaderRead(_)),
    export_c_func!(xmlTextReaderNodeType(_)),
    export_c_func!(xmlTextReaderConstName(_)),
    export_c_func!(xmlTextReaderConstValue(_)),
    export_c_func!(xmlTextReaderName(_)),
    export_c_func!(xmlTextReaderValue(_)),
    export_c_func!(xmlTextReaderReadInnerXml(_)),
    export_c_func!(xmlTextReaderReadOuterXml(_)),
    export_c_func!(xmlTextReaderGetAttribute(_, _)),
    export_c_func!(xmlTextReaderAttributeCount(_)),
    export_c_func!(xmlTextReaderDepth(_)),
    export_c_func!(xmlTextReaderHasAttributes(_)),
    export_c_func!(xmlTextReaderIsEmptyElement(_)),
    export_c_func!(xmlTextReaderNext(_)),
    export_c_func!(xmlTextReaderNextSibling(_)),
    export_c_func!(xmlTextReaderLocalName(_)),
    export_c_func!(xmlTextReaderPrefix(_)),
    export_c_func!(xmlTextReaderNamespaceUri(_)),
    // DOM tree
    export_c_func!(xmlChildrenNode(_)),
    export_c_func!(xmlGetLastChild(_)),
    export_c_func!(xmlNextElementSibling(_)),
    export_c_func!(xmlFirstElementChild(_)),
    export_c_func!(xmlNodeListGetString(_, _, _)),
    // Строковые утилиты
    export_c_func!(xmlStrdup(_)),
    export_c_func!(xmlStrlen(_)),
    export_c_func!(xmlStrsub(_, _, _)),
];

use crate::mem::{GuestUSize, MutPtr};

// ============================================================
// Хелперы
// ============================================================

fn alloc_xml_mem(env: &mut Environment) -> u32 {
    let size: GuestUSize = 512;
    let ptr: MutPtr<u8> = env.mem.alloc(size).cast();
    let slice = env.mem.bytes_at_mut(ptr, size);
    for byte in slice.iter_mut() {
        *byte = 0;
    }
    ptr.to_bits()
}

fn alloc_empty_string(env: &mut Environment) -> u32 {
    let ptr: MutPtr<u8> = env.mem.alloc(1).cast();
    env.mem.write(ptr, 0u8);
    ptr.to_bits()
}

fn alloc_string(env: &mut Environment, s: &str) -> u32 {
    let len: GuestUSize = s.len() as GuestUSize;
    let ptr: MutPtr<u8> = env.mem.alloc(len + 1).cast();
    for (i, &b) in s.as_bytes().iter().enumerate() {
        env.mem.write(ptr + (i as GuestUSize), b);
    }
    env.mem.write(ptr + len, 0u8);
    ptr.to_bits()
}

/// Читает C-строку из guest memory, возвращает Vec<u8> (владеющий).
fn read_cstr_bytes(env: &Environment, str_ptr: u32) -> Vec<u8> {
    if str_ptr == 0 {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut offset: GuestUSize = 0;
    loop {
        let byte: u8 = env.mem.read(crate::mem::Ptr::<u8, false>::from_bits(str_ptr + offset));
        if byte == 0 {
            break;
        }
        result.push(byte);
        offset += 1;
    }
    result
}

fn read_cstr_as_string(env: &Environment, str_ptr: u32) -> String {
    if str_ptr == 0 {
        return "<null>".to_string();
    }
    let bytes = read_cstr_bytes(env, str_ptr);
    String::from_utf8_lossy(&bytes).into_owned()
}

fn dup_cstr(env: &mut Environment, src_ptr: u32) -> u32 {
    if src_ptr == 0 {
        return 0;
    }
    let bytes = read_cstr_bytes(env, src_ptr);
    let len: GuestUSize = bytes.len() as GuestUSize;
    let dst: MutPtr<u8> = env.mem.alloc(len + 1).cast();
    for (i, &b) in bytes.iter().enumerate() {
        env.mem.write(dst + (i as GuestUSize), b);
    }
    env.mem.write(dst + len, 0u8);
    dst.to_bits()
}

// ============================================================
// Fake XML TextReader: эмулирует проход по валидному XML-документу
// ============================================================
//
// Структура эмулируемого XML для MenuElements.xml:
//
//   <MenuElements>          (depth=0, OPEN, pos=0)
//     <Image>               (depth=1, OPEN, pos=1)
//     </Image>              (depth=1, CLOSE, pos=2)
//     <Button>              
//     (depth=1, OPEN, pos=3)
//     </Button>             (depth=1, CLOSE, pos=4)
//     <Text>                (depth=1, OPEN, pos=5)
//     </Text>               (depth=1, CLOSE, pos=6)
//   </MenuElements>         (depth=0, CLOSE, pos=7)
//

// Типы узлов XML TextReader
const XML_READER_TYPE_ELEMENT: i32 = 1;
const XML_READER_TYPE_TEXT: i32 = 3;
const XML_READER_TYPE_SIGNIFICANT_WHITESPACE: i32 = 14;
const XML_READER_TYPE_END_ELEMENT: i32 = 15;

struct FakeXmlReader {
    position: u32,
    max_position: u32,
}

impl FakeXmlReader {
    fn new() -> Self {
        FakeXmlReader {
            position: 0, // будет увеличен при первом read
            max_position: 7,
        }
    }

    fn read_next(&mut self) -> i32 {
        self.position += 1;
        if self.position <= self.max_position {
            1 // есть узел
        } else {
            0 // EOF
        }
    }

    fn node_type(&self) -> i32 {
        match self.position {
            0 => 0,
            1 => XML_READER_TYPE_ELEMENT, // <MenuElements>
            2 => XML_READER_TYPE_ELEMENT, // <Image>
            3 => XML_READER_TYPE_END_ELEMENT, // </Image>
            4 => XML_READER_TYPE_ELEMENT, // <Button>
            5 => XML_READER_TYPE_END_ELEMENT, // </Button>
            6 => XML_READER_TYPE_ELEMENT, // <Text>
            7 => XML_READER_TYPE_END_ELEMENT, // </Text>
            8 => XML_READER_TYPE_END_ELEMENT, // </MenuElements>
            _ => 0,
        }
    }

    fn element_name(&self) -> &str {
        match self.position {
            1 => "MenuElements",
            2 => "Image",
            3 => "Image",
            4 => "Button",
            5 => "Button",
            6 => "Text",
            7 => "Text",
            8 => "MenuElements",
            _ => "",
        }
    }

    fn depth(&self) -> i32 {
        match self.position {
            1 => 0,
            2 => 1,
            3 => 1,
            4 => 1,
            5 => 1,
            6 => 1,
            7 => 1,
            8 => 0,
            _ => 0,
        }
    }

    fn is_empty_element(&self) -> i32 {
        0 // нет self-closing элементов
    }

    fn attribute_count(&self) -> i32 {
        0 // атрибутов нет в этой версии
    }

    fn has_attributes(&self) -> i32 {
        0
    }
}

/// Структура reader'а в guest memory.
/// offset 0:   позиция чтения (u32)
/// offset 4:   макс позиция (u32)
/// offset 8:   резерв
/// offset 12:  резерв
/// ...до 512 байт нулей от alloc_xml_mem

fn get_reader(env: &Environment, reader_ptr: u32) -> FakeXmlReader {
    if reader_ptr == 0 {
        return FakeXmlReader::new();
    }
    let pos: u32 = env.mem.read(crate::mem::Ptr::<u32, false>::from_bits(reader_ptr));
    let max: u32 = env.mem.read(crate::mem::Ptr::<u32, false>::from_bits(reader_ptr + 4));
    FakeXmlReader {
        position: pos,
        max_position: if max == 0 { 7 } else { max },
    }
}

fn set_reader(env: &mut Environment, reader_ptr: u32, reader: &FakeXmlReader) {
    env.mem.write(
        crate::mem::Ptr::<u32, true>::from_bits(reader_ptr),
        reader.position,
    );
    env.mem.write(
        crate::mem::Ptr::<u32, true>::from_bits(reader_ptr + 4),
        reader.max_position,
    );
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
    let filename_str = read_cstr_as_string(env, filename);
    log!("xmlReaderForFile(\"{}\") — returning stub", filename_str);

    // Выделяем память под reader и инициализируем состояние
    let ptr = alloc_xml_mem(env);
    let reader = FakeXmlReader::new();
    set_reader(env, ptr, &reader);
    ptr
}

/// void xmlFreeTextReader(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlFreeTextReader(_env: &mut Environment, _reader: u32) {}

/// int xmlTextReaderRead(xmlTextReaderPtr reader);
/// Возвращает 1 = есть узел, 0 = конец документа, -1 = ошибка.
#[allow(non_snake_case)]
fn xmlTextReaderRead(env: &mut Environment, reader_ptr: u32) -> i32 {
    let mut reader = get_reader(env, reader_ptr);
    let result = reader.read_next();
    set_reader(env, reader_ptr, &reader);
    log!(
        "xmlTextReaderRead -> {} (pos={})",
        result,
        reader.position
    );
    result
}

/// int xmlTextReaderNodeType(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderNodeType(env: &mut Environment, reader_ptr: u32) -> i32 {
    let reader = get_reader(env, reader_ptr);
    let t = reader.node_type();
    log!("xmlTextReaderNodeType -> {} (pos={})", t, reader.position);
    t
}

/// const xmlChar *xmlTextReaderConstName(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderConstName(env: &mut Environment, reader_ptr: u32) -> u32 {
    let reader = get_reader(env, reader_ptr);
    let name = reader.element_name();
    log!("xmlTextReaderConstName -> \"{}\"", name);
    alloc_string(env, name)
}

/// const xmlChar *xmlTextReaderConstValue(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderConstValue(env: &mut Environment, _reader: u32) -> u32 {
    alloc_empty_string(env)
}

/// xmlChar *xmlTextReaderName(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderName(env: &mut Environment, reader_ptr: u32) -> u32 {
    let reader = get_reader(env, reader_ptr);
    let name = reader.element_name();
    alloc_string(env, name)
}

/// xmlChar *xmlTextReaderValue(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderValue(env: &mut Environment, _reader: u32) -> u32 {
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
fn xmlTextReaderAttributeCount(env: &mut Environment, reader_ptr: u32) -> i32 {
    let reader = get_reader(env, reader_ptr);
    reader.attribute_count()
}

/// int xmlTextReaderDepth(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderDepth(env: &mut Environment, reader_ptr: u32) -> i32 {
    let reader = get_reader(env, reader_ptr);
    reader.depth()
}

/// int xmlTextReaderHasAttributes(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderHasAttributes(env: &mut Environment, reader_ptr: u32) -> i32 {
    let reader = get_reader(env, reader_ptr);
    reader.has_attributes()
}

/// int xmlTextReaderIsEmptyElement(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderIsEmptyElement(env: &mut Environment, reader_ptr: u32) -> i32 {
    let reader = get_reader(env, reader_ptr);
    reader.is_empty_element()
}

/// int xmlTextReaderNext(xmlTextReaderPtr reader);
/// Переход к следующему узлу того же уровня. Возвращает 1 при успехе, 0 при конце.
#[allow(non_snake_case)]
fn xmlTextReaderNext(env: &mut Environment, reader_ptr: u32) -> i32 {
    let mut reader = get_reader(env, reader_ptr);
    // Пропускаем всё до следующего элемента того же depth
    loop {
        let result = reader.read_next();
        if result != 1 {
            set_reader(env, reader_ptr, &reader);
            return 0;
        }
        if reader.node_type() == XML_READER_TYPE_ELEMENT {
            set_reader(env, reader_ptr, &reader);
            return 1;
        }
    }
}

/// int xmlTextReaderNextSibling(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderNextSibling(env: &mut Environment, reader_ptr: u32) -> i32 {
    let mut reader = get_reader(env, reader_ptr);
    let current_depth = reader.depth();
    loop {
        let result = reader.read_next();
        if result != 1 {
            set_reader(env, reader_ptr, &reader);
            return 0;
        }
        if reader.depth() == current_depth && reader.node_type() == XML_READER_TYPE_ELEMENT {
            set_reader(env, reader_ptr, &reader);
            return 1;
        }
    }
}

/// const xmlChar *xmlTextReaderLocalName(xmlTextReaderPtr reader);
#[allow(non_snake_case)]
fn xmlTextReaderLocalName(env: &mut Environment, reader_ptr: u32) -> u32 {
    let reader = get_reader(env, reader_ptr);
    let name = reader.element_name();
    alloc_string(env, name)
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
// DOM tree traversal
// ============================================================

#[allow(non_snake_case)]
fn xmlChildrenNode(_env: &mut Environment, _node: u32) -> u32 {
    0
}

#[allow(non_snake_case)]
fn xmlGetLastChild(_env: &mut Environment, _node: u32) -> u32 {
    0
}

#[allow(non_snake_case)]
fn xmlNextElementSibling(_env: &mut Environment, _node: u32) -> u32 {
    0
}

#[allow(non_snake_case)]
fn xmlFirstElementChild(_env: &mut Environment, _node: u32) -> u32 {
    0
}

#[allow(non_snake_case)]
fn xmlNodeListGetString(env: &mut Environment, _doc: u32, _list: u32, _inline: u32) -> u32 {
    alloc_empty_string(env)
}

// ============================================================
// Строковые утилиты libxml2
// ============================================================

#[allow(non_snake_case)]
fn xmlStrdup(env: &mut Environment, src_ptr: u32) -> u32 {
    dup_cstr(env, src_ptr)
}

#[allow(non_snake_case)]
fn xmlStrlen(env: &mut Environment, str_ptr: u32) -> i32 {
    if str_ptr == 0 {
        return 0;
    }
    let mut len: i32 = 0;
    let mut offset: GuestUSize = 0;
    loop {
        let byte: u8 = env.mem.read(crate::mem::Ptr::<u8, false>::from_bits(str_ptr + offset));
        if byte == 0 {
            break;
        }
        len += 1;
        offset += 1;
    }
    len
}

#[allow(non_snake_case)]
fn xmlStrsub(env: &mut Environment, str_ptr: u32, start: u32, len: u32) -> u32 {
    if str_ptr == 0 {
        return 0;
    }
    let mut bytes: Vec<u8> = Vec::new();
    for i in 0..len {
        let byte: u8 = env.mem.read(crate::mem::Ptr::<u8, false>::from_bits(str_ptr + start + i));
        bytes.push(byte);
    }
    let dst: MutPtr<u8> = env.mem.alloc(len + 1).cast();
    for (i, &b) in bytes.iter().enumerate() {
        env.mem.write(dst + (i as GuestUSize), b);
    }
    env.mem.write(dst + len, 0u8);
    dst.to_bits()
}

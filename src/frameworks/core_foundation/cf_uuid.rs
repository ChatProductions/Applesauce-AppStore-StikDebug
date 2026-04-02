use uuid::Uuid; // Assuming a uuid crate is available in the environment

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CFUUIDBytes {
    pub byte0: u8, pub byte1: u8, pub byte2: u8, pub byte3: u8,
    pub byte4: u8, pub byte5: u8, pub byte6: u8, pub byte7: u8,
    pub byte8: u8, pub byte9: u8, pub byte10: u8, pub byte11: u8,
    pub byte12: u8, pub byte13: u8, pub byte14: u8, pub byte15: u8,
}

pub type CFUUIDRef = super::CFTypeRef;

struct CFUUIDHostObject {
    bytes: CFUUIDBytes,
}
impl HostObject for CFUUIDHostObject {}

// Helper to convert Uuid crate to CFUUIDBytes
fn uuid_to_cf_bytes(u: Uuid) -> CFUUIDBytes {
    let b = u.as_bytes();
    CFUUIDBytes {
        byte0: b[0], byte1: b[1], byte2: b[2], byte3: b[3],
        byte4: b[4], byte5: b[5], byte6: b[6], byte7: b[7],
        byte8: b[8], byte9: b[9], byte10: b[10], byte11: b[11],
        byte12: b[12], byte13: b[13], byte14: b[14], byte15: b[15],
    }
}

// --- Functions ---

fn CFUUIDCreate(env: &mut Environment, allocator: CFAllocatorRef) -> CFUUIDRef {
    assert_eq!(allocator, kCFAllocatorDefault);
    let new_uuid = Uuid::new_v4();
    let host_obj = CFUUIDHostObject { bytes: uuid_to_cf_bytes(new_uuid) };
    
    // We assume a 'CFUUID' class exists or we use a generic HostObject wrapper
    env.objc.alloc_object_with_host(msg_class![env; CFUUID], Box::new(host_obj), &mut env.mem)
}

fn CFUUIDCreateFromString(env: &mut Environment, allocator: CFAllocatorRef, string: CFStringRef) -> CFUUIDRef {
    let rust_str = ns_string::to_rust_string(env, string);
    let u = Uuid::parse_str(&rust_str).unwrap_or_else(|_| Uuid::nil());
    let host_obj = CFUUIDHostObject { bytes: uuid_to_cf_bytes(u) };
    env.objc.alloc_object_with_host(msg_class![env; CFUUID], Box::new(host_obj), &mut env.mem)
}

fn CFUUIDCreateWithBytes(
    env: &mut Environment, 
    allocator: CFAllocatorRef,
    b0: u8, b1: u8, b2: u8, b3: u8, b4: u8, b5: u8, b6: u8, b7: u8,
    b8: u8, b9: u8, b10: u8, b11: u8, b12: u8, b13: u8, b14: u8, b15: u8
) -> CFUUIDRef {
    let host_obj = CFUUIDHostObject {
        bytes: CFUUIDBytes { b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15 }
    };
    env.objc.alloc_object_with_host(msg_class![env; CFUUID], Box::new(host_obj), &mut env.mem)
}

fn CFUUIDGetUUIDBytes(env: &mut Environment, uuid: CFUUIDRef) -> CFUUIDBytes {
    let host = env.objc.borrow::<CFUUIDHostObject>(uuid);
    host.bytes
}

fn CFUUIDCreateString(env: &mut Environment, allocator: CFAllocatorRef, uuid: CFUUIDRef) -> CFStringRef {
    let host = env.objc.borrow::<CFUUIDHostObject>(uuid);
    let b = host.bytes;
    let s = format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        b.byte0, b.byte1, b.byte2, b.byte3, b.byte4, b.byte5, b.byte6, b.byte7,
        b.byte8, b.byte9, b.byte10, b.byte11, b.byte12, b.byte13, b.byte14, b.byte15
    );
    ns_string::from_rust_string(env, s)
}

pub const FUNCTIONS: FunctionExports = &[
    // ... existing ...
    export_c_func!(CFUUIDCreate(_)),
    export_c_func!(CFUUIDCreateFromString(_, _)),
    export_c_func!(CFUUIDCreateWithBytes(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _)),
    export_c_func!(CFUUIDGetUUIDBytes(_, _)),
    export_c_func!(CFUUIDCreateString(_, _)),
];

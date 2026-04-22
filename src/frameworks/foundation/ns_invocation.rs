/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSInvocation` and `NSMethodSignature`.

use crate::abi::{extend_stack_for_args, write_next_arg, GuestArg};
use crate::cpu::Cpu;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::libc::string::strdup;
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, objc_msgSend, release, retain, ClassExports, HostObject,
    NSZonePtr, SEL,
};

// =========================================================================
// MARK: - Helpers (Parsing ObjC Types)
// =========================================================================

/// Честный парсинг строки типов Objective-C
fn parse_objc_types(s: &str) -> Vec<String> {
    let mut res = Vec::new();
    let mut chars = s.chars().peekable();
    while chars.peek().is_some() {
        let mut ty = String::new();
        while let Some(&c) = chars.peek() {
            if "rnNoORV".contains(c) { ty.push(chars.next().unwrap()); } 
            else { break; }
        }
        while let Some(&c) = chars.peek() {
            if c == '^' { ty.push(chars.next().unwrap()); } 
            else { break; }
        }
        if let Some(&c) = chars.peek() {
            ty.push(chars.next().unwrap());
            match c {
                '{' | '(' | '[' => {
                    let open = c;
                    let close = match c { '{' => '}', '(' => ')', '[' => ']', _ => unreachable!() };
                    let mut depth = 1;
                    while let Some(&sc) = chars.peek() {
                        ty.push(chars.next().unwrap());
                        if sc == open { depth += 1; }
                        else if sc == close {
                            depth -= 1;
                            if depth == 0 { break; }
                        }
                    }
                }
                _ => {}
            }
        }
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() { chars.next(); } 
            else { break; }
        }
        if !ty.is_empty() { res.push(ty); }
    }
    res
}

/// ИСПРАВЛЕННЫЙ МАКРОС: использует .alloc() и .write() вместо несуществующего .alloc_bytes()
macro_rules! alloc_c_string {
    ($env:expr, $s:expr) => {{
        let s_str = $s;
        let c_str = std::ffi::CString::new(s_str).unwrap();
        let bytes = c_str.as_bytes_with_nul();
        // Выделяем память в гостевой системе
        let ptr = $env.mem.alloc(bytes.len() as u32).cast::<u8>();
        // Побайтово записываем строку
        for (i, &byte) in bytes.iter().enumerate() {
            $env.mem.write(ptr.offset(i as isize), byte);
        }
        ptr
    }};
}

// =========================================================================
// MARK: - NSMethodSignature Host Object
// =========================================================================

struct NSMethodSignatureHostObject {
    return_type: String,
    argument_types: Vec<String>,
    return_type_ptr: Option<MutPtr<u8>>,
    argument_type_ptrs: Vec<MutPtr<u8>>,
}
impl HostObject for NSMethodSignatureHostObject {}

// =========================================================================
// MARK: - NSInvocation Host Object
// =========================================================================

struct NSInvocationHostObject {
    /// `NSMethodSignature *`
    sig: id,
    /// Строки типов аргументов, полученные из `sig` во время создания
    argument_types: Vec<String>,
    target: id,
    selector: Option<SEL>,
    /// Выделенный буфер для каждого аргумента.
    arguments: Vec<Option<MutVoidPtr>>,
    arguments_retained: bool,
    /// Объекты, удержанные через `retainArguments`
    retained_objects: Vec<id>,
    /// Копии C-строк, созданные через `retainArguments`
    copied_strings: Vec<MutPtr<u8>>,
    /// Буфер возвращаемого значения (до 8 байт для примитивов в R0/R1)
    return_value: [u32; 2],
}
impl HostObject for NSInvocationHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - NSMethodSignature
// =========================================================================

@implementation NSMethodSignature: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSMethodSignatureHostObject {
        return_type: "v".to_string(),
        argument_types: vec!["@".to_string(), ":".to_string()],
        return_type_ptr: None,
        argument_type_ptrs: Vec::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)signatureWithObjCTypes:(MutVoidPtr)_types {
    let sig: id = msg_class![env; NSMethodSignature alloc];
    let sig: id = msg![env; sig init];
    
    if !_types.is_null() {
        let types_str = env.mem.cstr_at_utf8(_types.cast()).unwrap_or("");
        let parsed = parse_objc_types(types_str);
        
        if !parsed.is_empty() {
            let mut host = env.objc.borrow_mut::<NSMethodSignatureHostObject>(sig);
            host.return_type = parsed[0].clone();
            
            if parsed.len() > 1 {
                host.argument_types = parsed[1..].to_vec();
            } else {
                host.argument_types.clear();
            }
            
            host.return_type_ptr = Some(alloc_c_string!(env, host.return_type.as_str()));
            
            for arg in &host.argument_types {
                let arg_ptr = alloc_c_string!(env, arg.as_str());
                host.argument_type_ptrs.push(arg_ptr);
            }
        }
    }
    autorelease(env, sig)
}

- (id)init {
    this
}

- (NSUInteger)numberOfArguments {
    env.objc.borrow::<NSMethodSignatureHostObject>(this).argument_types.len() as NSUInteger
}

- (())_touchHLE_setNumberOfArguments:(NSUInteger)count {
    let mut host = env.objc.borrow_mut::<NSMethodSignatureHostObject>(this);
    while host.argument_types.len() < count as usize {
        host.argument_types.push("@".to_string());
        // Используем исправленный макрос
        let arg_ptr = alloc_c_string!(env, "@");
        host.argument_type_ptrs.push(arg_ptr);
    }
}
    
- (crate::mem::ConstPtr<std::ffi::c_char>)methodReturnType {
    let host = env.objc.borrow::<NSMethodSignatureHostObject>(this);
    if let Some(ptr) = host.return_type_ptr {
        ptr.cast_const().cast()
    } else {
        crate::mem::ConstPtr::null()
    }
}

- (crate::mem::ConstPtr<std::ffi::c_char>)getArgumentTypeAtIndex:(NSUInteger)index {
    let host = env.objc.borrow::<NSMethodSignatureHostObject>(this);
    if index < host.argument_type_ptrs.len() as NSUInteger {
        host.argument_type_ptrs[index as usize].cast_const().cast()
    } else {
        crate::mem::ConstPtr::null()
    }
}

- (NSUInteger)methodReturnLength {
    let ret_type_ptr: crate::mem::ConstPtr<std::ffi::c_char> = msg![env; this methodReturnType];
    if ret_type_ptr.is_null() {
        return 0;
    }
    
    let ret_type_str = env.mem.cstr_at_utf8(ret_type_ptr.cast()).unwrap_or("");
    let core_type = ret_type_str.trim_start_matches(|c| "rnNoORV".contains(c));
    
    match core_type.chars().next() {
        Some('v') => 0, 
        Some('c') | Some('C') | Some('B') => 1,
        Some('s') | Some('S') => 2, 
        Some('i') | Some('I') | Some('l') | Some('L') | Some('f') => 4, 
        Some('q') | Some('Q') | Some('d') => 8, 
        Some('@') | Some('#') | Some('*') | Some('^') | Some(':') | Some('?') => 4, 
        Some('{') => {
            log!("Warning: methodReturnLength for struct {} is stubbed, returning 0", core_type);
            0
        }
        _ => 4
    }
}
    
- (())dealloc {
    let mut host = env.objc.borrow_mut::<NSMethodSignatureHostObject>(this);
    if let Some(ptr) = host.return_type_ptr {
        env.mem.free(ptr.cast());
    }
    for ptr in host.argument_type_ptrs.drain(..) {
        env.mem.free(ptr.cast());
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// =========================================================================
// MARK: - NSInvocation
// =========================================================================

@implementation NSInvocation: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSInvocationHostObject {
        sig: nil,
        argument_types: Vec::new(),
        target: nil,
        selector: None,
        arguments: Vec::new(),
        arguments_retained: false,
        retained_objects: Vec::new(),
        copied_strings: Vec::new(),
        return_value: [0; 2],
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)invocationWithMethodSignature:(id)sig {
    retain(env, sig);
    let num_of_args: NSUInteger = msg![env; sig numberOfArguments];
    let mut argument_types: Vec<String> = Vec::with_capacity(num_of_args as usize);
    for i in 0..num_of_args {
        let type_ptr: ConstPtr<u8> = msg![env; sig getArgumentTypeAtIndex:i];
        if !type_ptr.is_null() {
            argument_types.push(env.mem.cstr_at_utf8(type_ptr).unwrap_or("@").to_string());
        } else {
            argument_types.push("@".to_string());
        }
    }
    let host_object = Box::new(NSInvocationHostObject {
        sig,
        argument_types,
        target: nil,
        selector: None,
        arguments: vec![None; num_of_args as usize],
        arguments_retained: false,
        retained_objects: Vec::new(),
        copied_strings: Vec::new(),
        return_value: [0; 2],
    });
    let res = env.objc.alloc_object(this, host_object, &mut env.mem);
    autorelease(env, res)
}

- (id)init {
    this
}

- (id)methodSignature {
    env.objc.borrow::<NSInvocationHostObject>(this).sig
}

- (id)target {
    env.objc.borrow::<NSInvocationHostObject>(this).target
}

- (())setTarget:(id)target {
    let old_target = env.objc.borrow::<NSInvocationHostObject>(this).target;
    let arguments_retained = env.objc.borrow::<NSInvocationHostObject>(this).arguments_retained;
    env.objc.borrow_mut::<NSInvocationHostObject>(this).target = target;
    if arguments_retained {
        retain(env, target);
        release(env, old_target);
    }
}

- (SEL)selector {
    env.objc.borrow::<NSInvocationHostObject>(this).selector.expect("NSInvocation selector not set")
}

- (())setSelector:(SEL)selector {
    env.objc.borrow_mut::<NSInvocationHostObject>(this).selector = Some(selector);
}

- (bool)argumentsRetained {
    env.objc.borrow::<NSInvocationHostObject>(this).arguments_retained
}

- (())retainArguments {
    let target = env.objc.borrow::<NSInvocationHostObject>(this).target;
    retain(env, target);

    let mut retained_objects: Vec<id> = Vec::new();
    let mut copied_strings: Vec<MutPtr<u8>> = Vec::new();

    let num_of_args = env.objc.borrow::<NSInvocationHostObject>(this).argument_types.len();
    for i in 2..num_of_args {
        let host = env.objc.borrow::<NSInvocationHostObject>(this);
        let Some(arg_loc) = host.arguments[i] else { continue };
        match host.argument_types[i].as_str() {
            "@" => {
                let obj: id = env.mem.read(arg_loc.cast().cast_const());
                retain(env, obj);
                retained_objects.push(obj);
            }
            "*" => {
                let str: MutPtr<u8> = env.mem.read(arg_loc.cast().cast_const());
                let str_copy = strdup(env, str.cast_const());
                env.mem.write(arg_loc.cast(), str_copy);
                copied_strings.push(str_copy);
            }
            _ => {}
        }
    }

    let mut host = env.objc.borrow_mut::<NSInvocationHostObject>(this);
    host.retained_objects = retained_objects;
    host.copied_strings = copied_strings;
    host.arguments_retained = true;
}

- (())getArgument:(MutVoidPtr)buffer atIndex:(NSInteger)index {
    let host = env.objc.borrow::<NSInvocationHostObject>(this);
    if let Some(arg_loc) = host.arguments.get(index as usize).and_then(|&a| a) {
        let arg_type = host.argument_types.get(index as usize).map(|s| s.as_str()).unwrap_or("");
        match arg_type {
            "f" => {
                let val: f32 = env.mem.read(arg_loc.cast().cast_const());
                env.mem.write(buffer.cast(), val);
            }
            "@" => {
                let val: id = env.mem.read(arg_loc.cast().cast_const());
                env.mem.write(buffer.cast(), val);
            }
            "*" => {
                let val: MutPtr<u8> = env.mem.read(arg_loc.cast().cast_const());
                env.mem.write(buffer.cast(), val);
            }
            _ if arg_type.starts_with('^') => {
                let val: MutVoidPtr = env.mem.read(arg_loc.cast().cast_const());
                env.mem.write(buffer.cast(), val);
            }
            _ => {
                let val: u32 = env.mem.read(arg_loc.cast().cast_const());
                env.mem.write(buffer.cast(), val);
            }
        }
    }
}

- (())setArgument:(MutVoidPtr)arg_loc atIndex:(NSInteger)idx {
    let &NSInvocationHostObject {
        ref arguments,
        ..
    } = env.objc.borrow::<NSInvocationHostObject>(this);

    assert!(1 < idx && idx < arguments.len() as NSInteger);

    if let Some(prev_arg) = arguments[idx as usize] {
        env.mem.free(prev_arg.cast());
    }

    let argument_types: &Vec<String> = env.objc.borrow::<NSInvocationHostObject>(this).argument_types.as_ref();
    let arg_type = argument_types.get(idx as usize).unwrap();
    let new: MutVoidPtr = match arg_type.as_str() {
        "f" => {
            let arg_loc: MutPtr<f32> = arg_loc.cast();
            let arg = env.mem.read(arg_loc);
            env.mem.alloc_and_write(arg).cast()
        }
        "@" => {
            let arg_loc: MutPtr<id> = arg_loc.cast();
            let arg = env.mem.read(arg_loc);
            env.mem.alloc_and_write(arg).cast()
        }
        "*" => {
            let arg_loc: MutPtr<MutPtr<u8>> = arg_loc.cast();
            let arg = env.mem.read(arg_loc);
            env.mem.alloc_and_write(arg).cast()
        }
        _ if arg_type.starts_with('^') => {
            let arg_loc: MutPtr<MutVoidPtr> = arg_loc.cast();
            let arg = env.mem.read(arg_loc);
            env.mem.alloc_and_write(arg).cast()
        }
        _ => {
            let arg_loc: MutPtr<u32> = arg_loc.cast();
            let arg = env.mem.read(arg_loc);
            env.mem.alloc_and_write(arg).cast()
        }
    };

    env.objc.borrow_mut::<NSInvocationHostObject>(this).arguments[idx as usize] = Some(new);
}

- (())getReturnValue:(MutVoidPtr)buffer {
    let host = env.objc.borrow::<NSInvocationHostObject>(this);
    let sig = host.sig;
    let ret_len: NSUInteger = msg![env; sig methodReturnLength];
    
    if ret_len > 0 && ret_len <= 4 {
        env.mem.write(buffer.cast::<u32>(), host.return_value[0]);
    } else if ret_len > 4 && ret_len <= 8 {
        let val64 = ((host.return_value[1] as u64) << 32) | (host.return_value[0] as u64);
        env.mem.write(buffer.cast::<u64>(), val64);
    }
}

- (())setReturnValue:(MutVoidPtr)buffer {
    let mut host = env.objc.borrow_mut::<NSInvocationHostObject>(this);
    let sig = host.sig;
    let ret_len: NSUInteger = msg![env; sig methodReturnLength];
    
    if ret_len > 0 && ret_len <= 4 {
        host.return_value[0] = env.mem.read(buffer.cast::<u32>().cast_const());
    } else if ret_len > 4 && ret_len <= 8 {
        let val64: u64 = env.mem.read(buffer.cast::<u64>().cast_const());
        host.return_value[0] = (val64 & 0xFFFFFFFF) as u32;
        host.return_value[1] = (val64 >> 32) as u32;
    }
}

- (())invokeWithTarget:(id)target {
    () = msg![env; this setTarget:target];
    () = msg![env; this invoke];
}

- (())invoke {
    let arguments: &Vec<Option<MutVoidPtr>> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
    let set_count = arguments.iter().flatten().count();
    let all_count = arguments.len();
    
    if set_count + 2 != all_count && all_count > 2 {
        log!("Warning: NSInvocation invoked without all arguments set");
    }

    let mut reg_count = 0;
    let argument_types: &Vec<String> = env.objc.borrow::<NSInvocationHostObject>(this).argument_types.as_ref();
    for arg_type in argument_types.iter() {
        reg_count += match arg_type.as_str() {
            "@" => <id as GuestArg>::REG_COUNT,
            ":" => <SEL as GuestArg>::REG_COUNT,
            "f" => <f32 as GuestArg>::REG_COUNT,
            "c" => <u8 as GuestArg>::REG_COUNT,
            "*" => <MutPtr<u8> as GuestArg>::REG_COUNT,
            _ if arg_type.starts_with('^') => <MutVoidPtr as GuestArg>::REG_COUNT,
            _ => <u32 as GuestArg>::REG_COUNT
        }
    }
    let regs = env.cpu.regs_mut();
    let old_sp = extend_stack_for_args(reg_count, regs);

    let arguments: &Vec<Option<MutVoidPtr>> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
    let mut reg_offset = 0;
    for i in 0..arguments.len() {
        if i == 0 {
            let target = env.objc.borrow::<NSInvocationHostObject>(this).target;
            let regs = env.cpu.regs_mut();
            write_next_arg::<id>(&mut reg_offset, regs, &mut env.mem, target);
            continue;
        }
        if i == 1 {
            let selector = env.objc.borrow::<NSInvocationHostObject>(this).selector.unwrap();
            let regs = env.cpu.regs_mut();
            write_next_arg::<SEL>(&mut reg_offset, regs, &mut env.mem, selector);
            continue;
        }
        
        if let Some(arg_slot) = arguments[i] {
            let arg_type = argument_types[i].as_str();
            match arg_type {
                "@" => {
                    let arg: ConstPtr<id> = arg_slot.cast().cast_const();
                    let arg_val = env.mem.read(arg);
                    let regs = env.cpu.regs_mut();
                    write_next_arg::<id>(&mut reg_offset, regs, &mut env.mem, arg_val);
                },
                "f" => {
                    let arg: ConstPtr<f32> = arg_slot.cast().cast_const();
                    let arg_val = env.mem.read(arg);
                    let regs = env.cpu.regs_mut();
                    write_next_arg::<f32>(&mut reg_offset, regs, &mut env.mem, arg_val);
                },
                "c" => {
                    let arg: ConstPtr<u8> = arg_slot.cast().cast_const();
                    let arg_val = env.mem.read(arg);
                    let regs = env.cpu.regs_mut();
                    write_next_arg::<u8>(&mut reg_offset, regs, &mut env.mem, arg_val);
                }
                "*" => {
                    let arg: ConstPtr<MutPtr<u8>> = arg_slot.cast().cast_const();
                    let arg_val = env.mem.read(arg);
                    let regs = env.cpu.regs_mut();
                    write_next_arg::<MutPtr<u8>>(&mut reg_offset, regs, &mut env.mem, arg_val);
                }
                _ if arg_type.starts_with('^') => {
                    let arg: ConstPtr<MutVoidPtr> = arg_slot.cast().cast_const();
                    let arg_val = env.mem.read(arg);
                    let regs = env.cpu.regs_mut();
                    write_next_arg::<MutVoidPtr>(&mut reg_offset, regs, &mut env.mem, arg_val);
                }
                _ => {
                    let arg: ConstPtr<u32> = arg_slot.cast().cast_const();
                    let arg_val = env.mem.read(arg);
                    let regs = env.cpu.regs_mut();
                    write_next_arg::<u32>(&mut reg_offset, regs, &mut env.mem, arg_val);
                }
            }
        }
    }

    // Вызов
    let &NSInvocationHostObject { target, selector, .. } = env.objc.borrow::<NSInvocationHostObject>(this);
    objc_msgSend(env, target, selector.unwrap());

    // Сохраняем результат из R0 и R1 (в ARM это индексы 0 и 1)
    let regs = env.cpu.regs_mut();
    let r0 = regs[0];
    let r1 = regs[1];
    
    // Восстанавливаем указатель стека
    regs[Cpu::SP] = old_sp;
    
    let mut host = env.objc.borrow_mut::<NSInvocationHostObject>(this);
    host.return_value[0] = r0;
    host.return_value[1] = r1;
}

- (())dealloc {
    let &NSInvocationHostObject { sig, target, arguments_retained, .. } = env.objc.borrow::<NSInvocationHostObject>(this);
    release(env, sig);
    if arguments_retained {
        release(env, target);
        let retained_objects = std::mem::take(
            &mut env.objc.borrow_mut::<NSInvocationHostObject>(this).retained_objects
        );
        for obj in retained_objects {
            release(env, obj);
        }
        let copied_strings = std::mem::take(
            &mut env.objc.borrow_mut::<NSInvocationHostObject>(this).copied_strings
        );
        for s in copied_strings {
            env.mem.free(s.cast());
        }
    }
    for ptr in env.objc.borrow::<NSInvocationHostObject>(this).arguments.iter().flatten() {
        env.mem.free(ptr.cast());
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

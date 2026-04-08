/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFArray` and `CFMutableArray`.
//!
//! These are toll-free bridged to `NSArray` and `NSMutableArray` in Apple's
//! implementation. Here they are the same types.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::{CFIndex, CFRelease, CFRetain, CFTypeRef};
use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::{ConstPtr, ConstVoidPtr, MutVoidPtr};
use crate::objc::{id, msg, msg_class, nil};
use crate::Environment;

pub type CFArrayRef        = CFTypeRef;
pub type CFMutableArrayRef = CFTypeRef;

// CFArrayCallBacks — we accept the pointer but don't use callbacks.
// A null pointer means "no callbacks" (non-retaining).
// The kCFTypeArrayCallBacks constant has a non-null pointer but we
// ignore the actual callbacks and use NSArray's retain/release instead.

// MARK: - Retain / Release

pub fn CFArrayRetain(env: &mut Environment, arr: CFArrayRef) -> CFArrayRef {
    if !arr.is_null() { CFRetain(env, arr) } else { arr }
}

pub fn CFArrayRelease(env: &mut Environment, arr: CFArrayRef) {
    if !arr.is_null() { CFRelease(env, arr); }
}

// MARK: - Immutable constructors

pub fn CFArrayCreate(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    values: ConstPtr<ConstVoidPtr>,
    num_values: CFIndex,
    _callbacks: ConstVoidPtr,
) -> CFArrayRef {
    let arr: id = msg_class![env; NSMutableArray new];
    for i in 0..num_values as u32 {
        let val: id = env.mem.read(values + i).cast().cast_mut();
        () = msg![env; arr addObject:val];
    }
    // Return an immutable copy.
    let immutable: id = msg![env; arr copy];
    crate::objc::release(env, arr);
    immutable
}

pub fn CFArrayCreateCopy(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    the_array: CFArrayRef,
) -> CFArrayRef {
    if the_array.is_null() { return nil; }
    msg![env; the_array copy]
}

// MARK: - Mutable constructors

pub fn CFArrayCreateMutable(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    capacity: CFIndex,
    callbacks: ConstVoidPtr,
) -> CFMutableArrayRef {
    // capacity hint is ignored — NSMutableArray grows dynamically.
    let _ = capacity;
    if callbacks.is_null() {
        msg_class![env; _touchHLE_NSMutableArray_non_retaining new]
    } else {
        // Non-null callbacks → use a retaining mutable array.
        msg_class![env; NSMutableArray new]
    }
}

pub fn CFArrayCreateMutableCopy(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    _capacity: CFIndex,
    the_array: CFArrayRef,
) -> CFMutableArrayRef {
    if the_array.is_null() {
        return msg_class![env; NSMutableArray new];
    }
    let copy: id = msg![env; the_array mutableCopy];
    copy
}

// MARK: - Queries

pub fn CFArrayGetCount(env: &mut Environment, array: CFArrayRef) -> CFIndex {
    if array.is_null() { return 0; }
    let count: NSUInteger = msg![env; array count];
    count.try_into().unwrap()
}

pub fn CFArrayGetValueAtIndex(
    env: &mut Environment,
    array: CFArrayRef,
    idx: CFIndex,
) -> ConstVoidPtr {
    let idx: NSUInteger = idx.try_into().unwrap();
    let value: id = msg![env; array objectAtIndex:idx];
    value.cast().cast_const()
}

pub fn CFArrayGetValues(
    env: &mut Environment,
    array: CFArrayRef,
    range_location: CFIndex,
    range_length:   CFIndex,
    values: crate::mem::MutPtr<ConstVoidPtr>,
) {
    if array.is_null() || values.is_null() { return; }
    for i in 0..range_length as u32 {
        let idx = (range_location as u32) + i;
        let val: id = msg![env; array objectAtIndex:idx];
        env.mem.write(values + i, val.cast().cast_const());
    }
}

pub fn CFArrayContainsValue(
    env: &mut Environment,
    array: CFArrayRef,
    range_location: CFIndex,
    range_length:   CFIndex,
    value: ConstVoidPtr,
) -> bool {
    if array.is_null() { return false; }
    let val: id = value.cast().cast_mut();
    let end = range_location + range_length;
    for i in range_location..end {
        let item: id = msg![env; array objectAtIndex:(i as NSUInteger)];
        let eq: bool = msg![env; item isEqual:val];
        if eq { return true; }
    }
    false
}

pub fn CFArrayGetFirstIndexOfValue(
    env: &mut Environment,
    array: CFArrayRef,
    range_location: CFIndex,
    range_length:   CFIndex,
    value: ConstVoidPtr,
) -> CFIndex {
    if array.is_null() { return -1; }
    let val: id = value.cast().cast_mut();
    let end = range_location + range_length;
    for i in range_location..end {
        let item: id = msg![env; array objectAtIndex:(i as NSUInteger)];
        let eq: bool = msg![env; item isEqual:val];
        if eq { return i; }
    }
    // kCFNotFound
    -1
}

pub fn CFArrayGetLastIndexOfValue(
    env: &mut Environment,
    array: CFArrayRef,
    range_location: CFIndex,
    range_length:   CFIndex,
    value: ConstVoidPtr,
) -> CFIndex {
    if array.is_null() { return -1; }
    let val: id = value.cast().cast_mut();
    let end = range_location + range_length;
    let mut last = -1i32;
    for i in range_location..end {
        let item: id = msg![env; array objectAtIndex:(i as NSUInteger)];
        let eq: bool = msg![env; item isEqual:val];
        if eq { last = i; }
    }
    last
}

// MARK: - Mutation

pub fn CFArrayAppendValue(env: &mut Environment, array: CFMutableArrayRef, value: ConstVoidPtr) {
    let value: id = value.cast().cast_mut();
    () = msg![env; array addObject:value];
}

pub fn CFArrayInsertValueAtIndex(
    env: &mut Environment,
    array: CFMutableArrayRef,
    idx: CFIndex,
    value: ConstVoidPtr,
) {
    let idx: NSUInteger = idx.try_into().unwrap();
    let val: id = value.cast().cast_mut();
    () = msg![env; array insertObject:val atIndex:idx];
}

pub fn CFArraySetValueAtIndex(
    env: &mut Environment,
    array: CFMutableArrayRef,
    idx: CFIndex,
    value: ConstVoidPtr,
) {
    let idx: NSUInteger = idx.try_into().unwrap();
    let val: id = value.cast().cast_mut();
    () = msg![env; array replaceObjectAtIndex:idx withObject:val];
}

pub fn CFArrayRemoveValueAtIndex(
    env: &mut Environment,
    array: CFMutableArrayRef,
    idx: CFIndex,
) {
    let idx: NSUInteger = idx.try_into().unwrap();
    () = msg![env; array removeObjectAtIndex:idx];
}

pub fn CFArrayRemoveAllValues(env: &mut Environment, array: CFMutableArrayRef) {
    () = msg![env; array removeAllObjects];
}

pub fn CFArrayAppendArray(
    env: &mut Environment,
    array: CFMutableArrayRef,
    other: CFArrayRef,
    range_location: CFIndex,
    range_length:   CFIndex,
) {
    if other.is_null() { return; }
    let end = range_location + range_length;
    for i in range_location..end {
        let val: id = msg![env; other objectAtIndex:(i as NSUInteger)];
        () = msg![env; array addObject:val];
    }
}

pub fn CFArrayReplaceValues(
    env: &mut Environment,
    array: CFMutableArrayRef,
    range_location: CFIndex,
    range_length:   CFIndex,
    new_values: ConstPtr<ConstVoidPtr>,
    new_count: CFIndex,
) {
    // Remove old range (in reverse order).
    for i in (range_location..range_location + range_length).rev() {
        () = msg![env; array removeObjectAtIndex:(i as NSUInteger)];
    }
    // Insert new values at range_location.
    for i in 0..new_count as u32 {
        let val: id = env.mem.read(new_values + i).cast().cast_mut();
        let idx = (range_location as u32 + i) as NSUInteger;
        () = msg![env; array insertObject:val atIndex:idx];
    }
}

pub fn CFArrayExchangeValuesAtIndices(
    env: &mut Environment,
    array: CFMutableArrayRef,
    idx1: CFIndex,
    idx2: CFIndex,
) {
    let i1 = idx1 as NSUInteger;
    let i2 = idx2 as NSUInteger;
    let v1: id = msg![env; array objectAtIndex:i1];
    let v2: id = msg![env; array objectAtIndex:i2];
    () = msg![env; array replaceObjectAtIndex:i1 withObject:v2];
    () = msg![env; array replaceObjectAtIndex:i2 withObject:v1];
}

// MARK: - Sorting

pub fn CFArraySortValues(
    env: &mut Environment,
    array: CFMutableArrayRef,
    range_location: CFIndex,
    range_length:   CFIndex,
    comparator: GuestFunction, // CFComparatorFunction
    context: MutVoidPtr,
) {
    use crate::abi::CallFromHost;
    // Extract the range into a temporary vec and sort.
    let end = range_location + range_length;
    let mut items: Vec<id> = (range_location..end)
        .map(|i| msg![env; array objectAtIndex:(i as NSUInteger)])
        .collect();

    // Simple insertion sort — avoids the borrow-checker complexity of in-place
    // sort with a closure that borrows env.
    let n = items.len();
    for i in 1..n {
        let mut j = i;
        while j > 0 {
            let a = items[j - 1];
            let b = items[j];
            let res: i32 = comparator.call_from_host(
                env,
                (a.cast::<U>().cast_const(), b.cast().cast_const(), context),
            );
            if res > 0 {
                items.swap(j - 1, j);
                j -= 1;
            } else {
                break;
            }
        }
    }

    // Write sorted items back.
    for (k, val) in items.iter().enumerate() {
        let idx = (range_location as usize + k) as NSUInteger;
        () = msg![env; array replaceObjectAtIndex:idx withObject:(*val)];
    }
}

// MARK: - Apply / callbacks

pub fn CFArrayApplyFunction(
    env: &mut Environment,
    array: CFArrayRef,
    range_location: CFIndex,
    range_length:   CFIndex,
    applier: GuestFunction, // CFArrayApplierFunction: (value, context) -> void
    context: MutVoidPtr,
) {
    use crate::abi::CallFromHost;
    let end = range_location + range_length;
    for i in range_location..end {
        let val: id = msg![env; array objectAtIndex:(i as NSUInteger)];
        let _: () = applier.call_from_host(
            env,
            (val.cast::<U>().cast_const(), context),
        );
    }
}

// MARK: - Description

pub fn CFArrayCreateDescription(
    env: &mut Environment,
    array: CFArrayRef,
) -> CFTypeRef {
    if array.is_null() { return nil; }
    msg![env; array description]
}

pub const FUNCTIONS: FunctionExports = &[
    // Lifecycle
    export_c_func!(CFArrayRetain(_)),
    export_c_func!(CFArrayRelease(_)),
    // Immutable
    export_c_func!(CFArrayCreate(_, _, _, _)),
    export_c_func!(CFArrayCreateCopy(_, _)),
    // Mutable
    export_c_func!(CFArrayCreateMutable(_, _, _)),
    export_c_func!(CFArrayCreateMutableCopy(_, _, _)),
    // Queries
    export_c_func!(CFArrayGetCount(_)),
    export_c_func!(CFArrayGetValueAtIndex(_, _)),
    export_c_func!(CFArrayGetValues(_, _, _, _)),
    export_c_func!(CFArrayContainsValue(_, _, _, _)),
    export_c_func!(CFArrayGetFirstIndexOfValue(_, _, _, _)),
    export_c_func!(CFArrayGetLastIndexOfValue(_, _, _, _)),
    // Mutation
    export_c_func!(CFArrayAppendValue(_, _)),
    export_c_func!(CFArrayInsertValueAtIndex(_, _, _)),
    export_c_func!(CFArraySetValueAtIndex(_, _, _)),
    export_c_func!(CFArrayRemoveValueAtIndex(_, _)),
    export_c_func!(CFArrayRemoveAllValues(_)),
    export_c_func!(CFArrayAppendArray(_, _, _, _)),
    export_c_func!(CFArrayReplaceValues(_, _, _, _, _)),
    export_c_func!(CFArrayExchangeValuesAtIndices(_, _, _)),
    // Sorting / applying
    export_c_func!(CFArraySortValues(_, _, _, _, _)),
    export_c_func!(CFArrayApplyFunction(_, _, _, _, _)),
    // Description
    export_c_func!(CFArrayCreateDescription(_)),
];

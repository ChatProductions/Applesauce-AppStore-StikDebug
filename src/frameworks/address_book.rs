/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! AddressBook.framework stub

use crate::dyld::{export_c_func, FunctionExports, HostDylib};
use crate::mem::{MutVoidPtr, Ptr};
use crate::Environment;

fn ABAddressBookCreate(_env: &mut Environment) -> MutVoidPtr {
    log!("Stubbed ABAddressBookCreate called. Returning NULL.");
    Ptr::null()
}

fn ABAddressBookGetPersonCount(_env: &mut Environment, _address_book: MutVoidPtr) -> i32 {
    log!("Stubbed ABAddressBookGetPersonCount called. Returning 0.");
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(ABAddressBookCreate()),
    export_c_func!(ABAddressBookGetPersonCount(_, _)),
];

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/AddressBook.framework/AddressBook",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[FUNCTIONS],
};

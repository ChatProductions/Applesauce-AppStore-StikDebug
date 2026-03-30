/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! CommonCrypto

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstVoidPtr, GuestUSize, MutVoidPtr};
use crate::Environment;

// CCCryptorStatus
const kCCSuccess: i32 = 0;
const kCCBufferTooSmall: i32 = -4301;

// Split CCCrypt into a wrapper that reads stack args manually.
// ARM ABI: R0-R3 = first 4 args, rest on stack.
// CCCrypt has 12 args total.
// We expose first 8 params as normal, then read remaining 4 from stack.
#[allow(non_snake_case)]
fn CCCrypt(
    env: &mut Environment,
    op: u32,
    alg: u32,
    options: u32,
    key: ConstVoidPtr,
    key_length: GuestUSize,
    _iv: ConstVoidPtr,
    data_in: ConstVoidPtr,
    data_in_length: GuestUSize,
) -> i32 {
    // Read remaining 4 args from guest stack
    let sp = env.cpu.regs()[13]; // SP
    let data_out = crate::mem::Ptr::from_bits(env.mem.read(crate::mem::Ptr::<u32, false>::from_bits(sp)));
    let data_out_available: u32 = env.mem.read(crate::mem::Ptr::<u32, false>::from_bits(sp + 4));
    let data_out_moved_ptr = crate::mem::Ptr::<u32, true>::from_bits(env.mem.read(crate::mem::Ptr::<u32, false>::from_bits(sp + 8)));

    log!(
        "CCCrypt(op={}, alg={}, options={:#x}, keyLen={}, dataLen={})",
        op, alg, options, key_length, data_in_length
    );

    if data_out_available < data_in_length {
        return kCCBufferTooSmall;
    }

    // RC4 stream cipher
    if alg == 4 {
        let input = env.mem.bytes_at(data_in.cast(), data_in_length).to_vec();
        let key_bytes = env.mem.bytes_at(key.cast(), key_length).to_vec();
        let mut output = vec![0u8; data_in_length as usize];

        let mut s: Vec<u8> = (0..=255u8).collect();
        let mut j: usize = 0;
        for i in 0..256usize {
            j = (j + s[i] as usize + key_bytes[i % key_length as usize] as usize) % 256;
            s.swap(i, j);
        }
        let mut i = 0usize;
        j = 0;
        for (idx, &byte) in input.iter().enumerate() {
            i = (i + 1) % 256;
            j = (j + s[i] as usize) % 256;
            s.swap(i, j);
            let k = s[(s[i] as usize + s[j] as usize) % 256];
            output[idx] = byte ^ k;
        }
        env.mem.bytes_at_mut(data_out, data_in_length).copy_from_slice(&output);
        env.mem.write(data_out_moved_ptr, data_in_length);
        return kCCSuccess;
    }

    // Other algorithms: copy as-is (TODO: implement AES/DES properly)
    let input = env.mem.bytes_at(data_in.cast(), data_in_length).to_vec();
    env.mem.bytes_at_mut(data_out, data_in_length).copy_from_slice(&input);
    env.mem.write(data_out_moved_ptr, data_in_length);
    log!("CCCrypt: alg={} not implemented, data copied as-is", alg);
    kCCSuccess
}

#[allow(non_snake_case)]
fn CCKeyDerivationPBKDF(
    _env: &mut Environment,
    _algorithm: u32,
    _password: ConstVoidPtr,
    _password_len: GuestUSize,
    _salt: ConstVoidPtr,
    _salt_len: GuestUSize,
    _prf: u32,
    _rounds: u32,
) -> i32 {
    log!("TODO: CCKeyDerivationPBKDF");
    kCCSuccess
}

#[allow(non_snake_case)]
fn CCHmac(
    _env: &mut Environment,
    _algorithm: u32,
    _key: ConstVoidPtr,
    _key_length: GuestUSize,
    _data: ConstVoidPtr,
    _data_length: GuestUSize,
    _mac_out: MutVoidPtr,
) {
    log!("TODO: CCHmac");
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CCCrypt(_, _, _, _, _, _, _, _)),
    export_c_func!(CCKeyDerivationPBKDF(_, _, _, _, _, _, _)), 
    export_c_func!(CCHmac(_, _, _, _, _, _)),
];

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/usr/lib/libcommonCrypto.dylib",
    aliases: &[
        "/System/Library/Frameworks/Security.framework/Security",
        "/usr/lib/libCommonCrypto.dylib",
    ],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[FUNCTIONS],
};

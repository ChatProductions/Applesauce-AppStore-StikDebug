/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! CommonCrypto (`/usr/lib/libcommonCrypto.dylib` etc.)
//!
//! Minimal implementation of CCCrypt and related functions.

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstVoidPtr, MutPtr, MutVoidPtr};
use crate::Environment;

// CCCrypt operation codes
const kCCEncrypt: u32 = 0;
const kCCDecrypt: u32 = 1;

// CCAlgorithm
const kCCAlgorithmAES128: u32 = 0;
const kCCAlgorithmDES: u32 = 1;
const kCCAlgorithm3DES: u32 = 2;
const kCCAlgorithmCAST: u32 = 3;
const kCCAlgorithmRC4: u32 = 4;
const kCCAlgorithmRC2: u32 = 5;
const kCCAlgorithmBlowfish: u32 = 6;

// CCOptions
const kCCOptionPKCS7Padding: u32 = 0x0001;
const kCCOptionECBMode: u32 = 0x0002;

// CCCryptorStatus
const kCCSuccess: i32 = 0;
const kCCParamError: i32 = -4300;
const kCCBufferTooSmall: i32 = -4301;
const kCCMemoryFailure: i32 = -4302;
const kCCAlignmentError: i32 = -4303;
const kCCDecodeError: i32 = -4304;
const kCCUnimplemented: i32 = -4305;

#[allow(non_snake_case)]
fn CCCrypt(
    env: &mut Environment,
    op: u32,
    alg: u32,
    options: u32,
    key: ConstVoidPtr,
    key_length: u32,
    iv: ConstVoidPtr,
    data_in: ConstVoidPtr,
    data_in_length: u32,
    data_out: MutVoidPtr,
    data_out_available: u32,
    data_out_moved: MutPtr<u32>,
) -> i32 {
    log!(
        "CCCrypt(op={}, alg={}, options={:#x}, keyLen={}, dataLen={})",
        op, alg, options, key_length, data_in_length
    );

    // Check if output buffer is large enough
    if data_out_available < data_in_length {
        return kCCBufferTooSmall;
    }

    // For RC4 (stream cipher, no padding needed) — XOR with key stream
    if alg == kCCAlgorithmRC4 {
        let input = env.mem.bytes_at(data_in.cast(), data_in_length);
        let key_bytes = env.mem.bytes_at(key.cast(), key_length);
        let mut output = vec![0u8; data_in_length as usize];

        // Simple RC4
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

        env.mem.bytes_at_mut(data_out.cast(), data_in_length).copy_from_slice(&output);
        env.mem.write(data_out_moved, data_in_length);
        return kCCSuccess;
    }

    // For other algorithms — just copy data as-is (TODO: implement properly)
    // This allows apps to at least not crash, even if crypto is wrong
    let input = env.mem.bytes_at(data_in.cast(), data_in_length).to_vec();
    env.mem.bytes_at_mut(data_out.cast(), data_in_length).copy_from_slice(&input);
    env.mem.write(data_out_moved, data_in_length);

    log!("CCCrypt: WARNING: alg={} not properly implemented, data copied as-is", alg);
    kCCSuccess
}

#[allow(non_snake_case)]
fn CCKeyDerivationPBKDF(
    _env: &mut Environment,
    _algorithm: u32,
    _password: ConstVoidPtr,
    _password_len: u32,
    _salt: ConstVoidPtr,
    _salt_len: u32,
    _prf: u32,
    _rounds: u32,
    _derived_key: MutVoidPtr,
    _derived_key_len: u32,
) -> i32 {
    log!("TODO: CCKeyDerivationPBKDF");
    kCCSuccess
}

#[allow(non_snake_case)]
fn CCHmac(
    _env: &mut Environment,
    _algorithm: u32,
    _key: ConstVoidPtr,
    _key_length: u32,
    _data: ConstVoidPtr,
    _data_length: u32,
    _mac_out: MutVoidPtr,
) {
    log!("TODO: CCHmac");
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CCCrypt(_, _, _, _, _, _, _, _, _, _, _, _)),
    export_c_func!(CCKeyDerivationPBKDF(_, _, _, _, _, _, _, _, _, _)),
    export_c_func!(CCHmac(_, _, _, _, _, _, _)),
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

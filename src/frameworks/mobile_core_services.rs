/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `MobileCoreServices.framework/MobileCoreServices`.
//!
//! On iOS, MobileCoreServices is the umbrella for Uniform Type Identifier
//! types and constants (`UTType*`, `kUTType*`). It is pulled in transitively
//! by all sorts of high-level UIKit APIs — `UIDocumentInteractionController`,
//! `MFMailComposeViewController`, `UIImagePickerController` — so most apps
//! end up listing it in their Mach-O dependency table even when they never
//! call any UTType function directly.
//!
//! Without a [crate::dyld::HostDylib] entry for the path
//! `/System/Library/Frameworks/MobileCoreServices.framework/`
//! `MobileCoreServices`,
//! touchHLE prints a `Warning: app binary depends on unimplemented or missing
//! dylib …` at startup, which can spook users into reporting otherwise-fine
//! apps as broken (e.g. HyperHLE appdb report #23, Mutant Fridge).
//!
//! This stub exists so the dependency is recognized and the warning is
//! suppressed. Real UTType handling is not implemented; functions that
//! actually need to consult UTType data should be added here as they come up.

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};

pub const FUNCTIONS: FunctionExports = &[];

// kUTTagClass* / kUTType* are CFStringRef constants exported by
// MobileCoreServices. Apps reference them in `__nl_symbol_ptr` via the
// UTType conversion API even when they never call those functions at
// runtime (e.g. UIImagePickerController media-type checks). Exporting them
// as plain CFString placeholders silences the non-lazy-symbol warning and
// keeps key-comparison-by-identity working.
pub const CONSTANTS: ConstantExports = &[
    (
        "_kUTTagClassFilenameExtension",
        HostConstant::NSString("public.filename-extension"),
    ),
    (
        "_kUTTagClassMIMEType",
        HostConstant::NSString("public.mime-type"),
    ),
    (
        "_kUTTagClassNSPboardType",
        HostConstant::NSString("com.apple.nspboard-type"),
    ),
    (
        "_kUTTagClassOSType",
        HostConstant::NSString("com.apple.ostype"),
    ),
    // The few UTType identifiers most commonly passed by name.
    ("_kUTTypeImage", HostConstant::NSString("public.image")),
    ("_kUTTypeMovie", HostConstant::NSString("public.movie")),
    ("_kUTTypeVideo", HostConstant::NSString("public.video")),
    ("_kUTTypeAudio", HostConstant::NSString("public.audio")),
    ("_kUTTypeData", HostConstant::NSString("public.data")),
    ("_kUTTypeURL", HostConstant::NSString("public.url")),
    ("_kUTTypeText", HostConstant::NSString("public.text")),
    (
        "_kUTTypePlainText",
        HostConstant::NSString("public.plain-text"),
    ),
    ("_kUTTypeJPEG", HostConstant::NSString("public.jpeg")),
    ("_kUTTypePNG", HostConstant::NSString("public.png")),
];

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSBundle`.

use super::{ns_string, NSUInteger};
use crate::bundle::Bundle;
use crate::frameworks::core_foundation::cf_bundle::{
    CFBundleCopyBundleLocalizations, CFBundleCopyPreferredLocalizationsFromArray,
};
use crate::frameworks::foundation::ns_string::from_rust_string;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;
use std::collections::{HashMap, HashSet};

// Should be ISO 639-1 (or ISO 639-2) compliant
// Legacy projects use language names while newer ones use language code lprojs
const LANG_ID_TO_LANG_PROJ: &[(&str, &[&str])] = &[
    ("da", &["Danish.lproj",    "da.lproj"]),
    ("nl", &["Dutch.lproj",     "nl.lproj"]),
    ("en", &["English.lproj",   "en.lproj"]),
    ("fi", &["Finnish.lproj",   "fi.lproj"]),
    ("fr", &["French.lproj",    "fr.lproj"]),
    ("de", &["German.lproj",    "de.lproj"]),
    ("it", &["Italian.lproj",   "it.lproj"]),
    ("ja", &["Japanese.lproj",  "ja.lproj"]),
    ("ko", &["Korean.lproj",    "ko.lproj"]),
    ("no", &["Norwegian.lproj", "no.lproj"]),
    ("pt", &["Portuguese.lproj","pt.lproj"]),
    ("ru", &["Russian.lproj",   "ru.lproj"]),
    ("zh", &["Chinese.lproj",   "zh.lproj"]),
    ("es", &["Spanish.lproj",   "es.lproj"]),
    ("sv", &["Swedish.lproj",   "sv.lproj"]),
    ("tr", &["Turkish.lproj",   "tr.lproj"]),
];

#[derive(Default)]
pub struct State {
    main_bundle: Option<id>,
    // Keyed by bundle path NSString* → NSBundle*
    bundle_cache: HashMap<String, id>,
    localization_tables: HashMap<id, id>, // NSString* to NSDictionary*
}

pub struct NSBundleHostObject {
    /// If this is [None], this is the main bundle's NSBundle instance and the
    /// [Bundle] is stored in [crate::Environment], not here.
    pub bundle: Option<Bundle>,
    /// NSString with bundle path.
    bundle_path: id,
    /// NSString with bundle identifier.
    bundle_identifier: id,
    /// NSURL with bundle path. [None] if not created yet.
    bundle_url: Option<id>,
    /// `NSDictionary*` for the `Info.plist` content. [None] if not created yet.
    info_dictionary: Option<id>,
}
impl HostObject for NSBundleHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSBundle: NSObject

// =========================================================================
// MARK: - Class methods / constructors
// =========================================================================

+ (id)mainBundle {
    if let Some(bundle) = env.framework_state.foundation.ns_bundle.main_bundle {
        bundle
    } else {
        let new = msg_class![env; _touchHLE_NSBundle_Static alloc];
        env.framework_state.foundation.ns_bundle.main_bundle = Some(new);
        new
    }
}

+ (id)bundleForClass:(id)_aClass {
    // Return the main bundle. For single-bundle iPhone apps this is always
    // correct. A full implementation would look up which bundle contains the
    // given class.
    msg_class![env; NSBundle mainBundle]
}

+ (id)bundleWithPath:(id)path { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithPath:path];
    autorelease(env, new)
}

+ (id)bundleWithURL:(id)url { // NSURL*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithURL:url];
    autorelease(env, new)
}

+ (id)bundleWithIdentifier:(id)identifier { // NSString*
    if identifier == nil {
        return nil;
    }
    // Check main bundle first.
    let main: id = msg_class![env; NSBundle mainBundle];
    let main_id: id = msg![env; main bundleIdentifier];
    if main_id != nil {
        let equal: bool = msg![env; main_id isEqualToString:identifier];
        if equal {
            return main;
        }
    }
    log!("TODO: [NSBundle bundleWithIdentifier:] non-main bundle — returning nil");
    nil
}

+ (id)allBundles {
    // Return an array containing only the main bundle.
    let main: id = msg_class![env; NSBundle mainBundle];
    let arr: id = msg_class![env; NSMutableArray new];
    let _: () = msg![env; arr addObject:main];
    autorelease(env, arr)
}

+ (id)allFrameworks {
    // No dynamically loaded frameworks in touchHLE.
    msg_class![env; NSArray array]
}

+ (id)preferredLocalizationsFromArray:(id)localizations_array { // NSArray<NSString *> *
    let preferred = CFBundleCopyPreferredLocalizationsFromArray(env, localizations_array);
    autorelease(env, preferred)
}

+ (id)preferredLocalizationsFromArray:(id)localizations_array
                       forPreferences:(id)_locale_identifiers_array {
    // Ignore the explicit preferences list and fall back to the system default.
    let preferred = CFBundleCopyPreferredLocalizationsFromArray(env, localizations_array);
    autorelease(env, preferred)
}

+ (id)pathsForResources:(id)ext          // NSString*
                  OfType:(id)r#type // NSString*
              inDirectory:(id)subpath { // NSString*
    // Prepend the lproj subdirectory when a localization is given, then
    // delegate to the non-localized variant.
    let effective_subpath: id = if inDirectory != nil {
        let lproj_suffix: id = ns_string::get_static_str(env, ".lproj");
        let lproj_dir: id = msg![env; inDirectory stringByAppendingString:lproj_suffix];
        if subpath != nil {
            msg![env; lproj_dir stringByAppendingPathComponent:subpath]
        } else {
            lproj_dir
        }
    } else {
        subpath
    };
    msg![env; this pathsForResourcesOfType:ext inDirectory:effective_subpath]
}

// =========================================================================
// MARK: - Initializers
// =========================================================================

- (id)initWithPath:(id)path { // NSString*
    if path == nil {
        release(env, this);
        return nil;
    }
    let path_str = ns_string::to_rust_string(env, path).into_owned();

    // Return cached instance if we already have one for this path.
    if let Some(&cached) = env.framework_state.foundation.ns_bundle.bundle_cache.get(&path_str) {
        release(env, this);
        return retain(env, cached);
    }

    let plist_file = format!("{}/Info.plist", path_str);
    let plist_guest = crate::fs::GuestPath::new(&plist_file);
    if env.fs.read(plist_guest).is_err() {
        log_dbg!("NSBundle initWithPath: no Info.plist at {:?}, returning nil", path_str);
        release(env, this);
        return nil;
    }

    let bundle_path_ns = ns_string::from_rust_string(env, path_str.clone());
    // Derive bundle identifier from Info.plist CFBundleIdentifier.
    let plist_path_ns = ns_string::get_static_str(env, "Info.plist");
    let full_plist: id = msg![env; bundle_path_ns stringByAppendingPathComponent:plist_path_ns];
    let dict: id = msg_class![env; NSDictionary alloc];
    let dict: id = msg![env; dict initWithContentsOfFile:full_plist];
    let id_key: id = ns_string::get_static_str(env, "CFBundleIdentifier");
    let bundle_identifier: id = if dict != nil {
        let val: id = msg![env; dict objectForKey:id_key];
        if val != nil { val } else { ns_string::get_static_str(env, "") }
    } else {
        ns_string::get_static_str(env, "")
    };

    let host = NSBundleHostObject {
        bundle: None,
        bundle_path: bundle_path_ns,
        bundle_identifier,
        bundle_url: None,
        info_dictionary: if dict != nil { Some(dict) } else { None },
    };
    env.objc.borrow_mut::<NSBundleHostObject>(this);  // ensure allocated
    *env.objc.borrow_mut::<NSBundleHostObject>(this) = host;

    env.framework_state
        .foundation
        .ns_bundle
        .bundle_cache
        .insert(path_str, this);
    this
}

- (id)initWithURL:(id)url { // NSURL*
    if url == nil {
        release(env, this);
        return nil;
    }
    let path: id = msg![env; url path];
    msg![env; this initWithPath:path]
}

// =========================================================================
// MARK: - Dealloc
// =========================================================================

- (())dealloc {
    let &NSBundleHostObject {
        bundle: _,
        bundle_path: _,
        bundle_identifier: _,
        bundle_url,
        info_dictionary,
    } = env.objc.borrow(this);
    if let Some(bundle_url) = bundle_url {
        release(env, bundle_url);
    }
    if let Some(info_dictionary) = info_dictionary {
        release(env, info_dictionary);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

// =========================================================================
// MARK: - Identity
// =========================================================================

- (id)bundlePath {
    env.objc.borrow::<NSBundleHostObject>(this).bundle_path
}

- (id)bundleIdentifier {
    env.objc.borrow::<NSBundleHostObject>(this).bundle_identifier
}

- (id)bundleURL {
    if let Some(url) = env.objc.borrow::<NSBundleHostObject>(this).bundle_url {
        url
    } else {
        let bundle_path: id = msg![env; this bundlePath];
        let new: id = msg_class![env; NSURL alloc];
        let new: id = msg![env; new initFileURLWithPath:bundle_path];
        env.objc.borrow_mut::<NSBundleHostObject>(this).bundle_url = Some(new);
        new
    }
}

// =========================================================================
// MARK: - Load state
// =========================================================================

- (bool)isLoaded {
    // The main bundle is always considered loaded; other bundles we never
    // actually load (dynamic loading is not supported).
    true
}

- (bool)load {
    log!("TODO: [NSBundle load] — returning YES (stub)");
    true
}

- (bool)unload {
    log!("TODO: [NSBundle unload] — returning NO");
    false
}

- (bool)preflightAndReturnError:(id)_error { // NSError**
    true
}

- (bool)loadAndReturnError:(id)_error { // NSError**
    log!("TODO: [NSBundle loadAndReturnError:] — returning YES (stub)");
    true
}

// =========================================================================
// MARK: - NIB loading
// =========================================================================

- (id)loadNibNamed:(id)name
             owner:(id)owner
           options:(id)options {
    if options != nil {
        let options_count: NSUInteger = msg![env; options count];
        assert_eq!(options_count, 0);
    }
    let nib: id = msg_class![env; UINib nibWithNibName:name bundle:this];
    msg![env; nib instantiateWithOwner:owner options:nil]
}

// =========================================================================
// MARK: - Paths and URLs
// =========================================================================

- (id)resourcePath {
    msg![env; this bundlePath]
}

- (id)resourceURL {
    msg![env; this bundleURL]
}

- (id)executablePath {
    let exec_path_str = env.bundle.executable_path().as_str().to_string();
    let exec_path = from_rust_string(env, exec_path_str);
    autorelease(env, exec_path)
}

- (id)executableURL {
    let exec_path: id = msg![env; this executablePath];
    if exec_path == nil {
        return nil;
    }
    let url: id = msg_class![env; NSURL alloc];
    let url: id = msg![env; url initFileURLWithPath:exec_path];
    autorelease(env, url)
}

- (id)privateFrameworksPath {
    let base: id = msg![env; this bundlePath];
    let comp: id = ns_string::get_static_str(env, "Frameworks");
    let path: id = msg![env; base stringByAppendingPathComponent:comp];
    autorelease(env, path)
}

- (id)privateFrameworksURL {
    let path: id = msg![env; this privateFrameworksPath];
    let url: id = msg_class![env; NSURL alloc];
    let url: id = msg![env; url initFileURLWithPath:path];
    autorelease(env, url)
}

- (id)sharedFrameworksPath {
    let base: id = msg![env; this bundlePath];
    let comp: id = ns_string::get_static_str(env, "SharedFrameworks");
    let path: id = msg![env; base stringByAppendingPathComponent:comp];
    autorelease(env, path)
}

- (id)sharedFrameworksURL {
    let path: id = msg![env; this sharedFrameworksPath];
    let url: id = msg_class![env; NSURL alloc];
    let url: id = msg![env; url initFileURLWithPath:path];
    autorelease(env, url)
}

- (id)builtInPlugInsPath {
    let base: id = msg![env; this bundlePath];
    let comp: id = ns_string::get_static_str(env, "PlugIns");
    let path: id = msg![env; base stringByAppendingPathComponent:comp];
    autorelease(env, path)
}

- (id)builtInPlugInsURL {
    let path: id = msg![env; this builtInPlugInsPath];
    let url: id = msg_class![env; NSURL alloc];
    let url: id = msg![env; url initFileURLWithPath:path];
    autorelease(env, url)
}

- (id)sharedSupportPath {
    let base: id = msg![env; this bundlePath];
    let comp: id = ns_string::get_static_str(env, "SharedSupport");
    let path: id = msg![env; base stringByAppendingPathComponent:comp];
    autorelease(env, path)
}

- (id)sharedSupportURL {
    let path: id = msg![env; this sharedSupportPath];
    let url: id = msg_class![env; NSURL alloc];
    let url: id = msg![env; url initFileURLWithPath:path];
    autorelease(env, url)
}

- (id)appStoreReceiptURL {
    log!("TODO: [NSBundle appStoreReceiptURL] — returning nil");
    nil
}

// =========================================================================
// MARK: - Resource lookup
// =========================================================================

- (id)pathsForResourcesOfType:(id)ext       // NSString*
                  inDirectory:(id)subpath { // NSString*
    let ext_str = if ext != nil {
        Some(ns_string::to_rust_string(env, ext))
    } else {
        None
    };
    let subpath_str = if subpath != nil {
        ns_string::to_rust_string(env, subpath)
    } else {
        std::borrow::Cow::Borrowed("")
    };
    let bundle_path: id = msg![env; this bundlePath];
    let base_path = ns_string::to_rust_string(env, bundle_path);
    let mut search_dir_str = base_path.into_owned();
    if !subpath_str.is_empty() {
        search_dir_str.push('/');
        search_dir_str.push_str(&subpath_str);
    }

    let search_dir = crate::fs::GuestPath::new(&search_dir_str);
    let mut temp_paths = Vec::new();
    if let Ok(iterator) = env.fs.enumerate(search_dir) {
        for path in iterator {
            let matches = match &ext_str {
                Some(extension) => path.ends_with(extension.as_ref()),
                None => true,
            };
            if matches {
                let mut full_path = search_dir_str.clone();
                full_path.push('/');
                full_path.push_str(path);
                temp_paths.push(full_path);
            }
        }
    } else {
        log!(
            "Warning: pathsForResourcesOfType:inDirectory: could not read directory {:?}",
            search_dir
        );
    }

    let mut res_paths = Vec::new();
    for path in temp_paths {
        res_paths.push(ns_string::from_rust_string(env, path));
    }

    let array: id = crate::frameworks::foundation::ns_array::from_vec(env, res_paths);
    autorelease(env, array)
}

- (id)pathsForResourcesOfType:(id)ext          // NSString*
                  inDirectory:(id)subpath      // NSString*
              forLocalization:(id)localization { // NSString*
    // Prepend the lproj subdirectory when a localization is given, then
    // delegate to the non-localized variant.
    let effective_subpath: id = if localization != nil {
        let lproj_suffix: id = ns_string::get_static_str(env, ".lproj");
        let lproj_dir: id = msg![env; localization stringByAppendingString:lproj_suffix];
        if subpath != nil {
            msg![env; lproj_dir stringByAppendingPathComponent:subpath]
        } else {
            lproj_dir
        }
    } else {
        subpath
    };
    msg![env; this pathsForResourcesOfType:ext inDirectory:effective_subpath]
}

- (id)URLsForResourcesWithExtension:(id)ext        // NSString*
                       subdirectory:(id)subpath {  // NSString*
    let paths: id = msg![env; this pathsForResourcesOfType:ext inDirectory:subpath];
    let count: NSUInteger = msg![env; paths count];
    let result: id = msg_class![env; NSMutableArray new];
    let mut i: NSUInteger = 0;
    while i < count {
        let path: id = msg![env; paths objectAtIndex:i];
        let url: id = msg_class![env; NSURL alloc];
        let url: id = msg![env; url initFileURLWithPath:path];
        let _: () = msg![env; result addObject:url];
        release(env, url);
        i += 1;
    }
    autorelease(env, result)
}

- (id)pathForResource:(id)name          // NSString*
               ofType:(id)extension     // NSString*
          inDirectory:(id)directory {   // NSString*
    // assert!(name != nil);

    let path = path_for_resource_helper(env, this, name, nil, directory, extension);
    if path != nil {
        return path;
    }

    // Try preferred languages in order.
    let langs: id = msg_class![env; NSLocale preferredLanguages];
    let lang_count: NSUInteger = msg![env; langs count];
    let mut unknown_codes = HashSet::new();
    for i in 0..lang_count {
        let lang_code: id = msg![env; langs objectAtIndex:i];
        let lang_code_str = ns_string::to_rust_string(env, lang_code);
        if let Some(&(_, lprojs)) = LANG_ID_TO_LANG_PROJ
            .iter()
            .find(|&&(code, _)| code == lang_code_str)
        {
            for lproj in lprojs {
                let lproj_ns: id = ns_string::get_static_str(env, lproj);
                let localized_path =
                    path_for_resource_helper(env, this, name, lproj_ns, directory, extension);
                if localized_path != nil {
                    return localized_path;
                }
            }
        } else {
            unknown_codes.insert(lang_code_str.into_owned());
        }
    }

    if !unknown_codes.is_empty() {
        log!(
            "TODO: language codes {:?} aren't mapped to a language name, falling back to English",
            unknown_codes
        );
    }

    // Fallback to English.
    for lproj in ["English.lproj", "en.lproj"] {
        let lproj_ns: id = ns_string::get_static_str(env, lproj);
        let path = path_for_resource_helper(env, this, name, lproj_ns, directory, extension);
        if path != nil {
            return path;
        }
    }
    nil
}

- (id)pathForResource:(id)name          // NSString*
               ofType:(id)extension {   // NSString*
    msg![env; this pathForResource:name ofType:extension inDirectory:nil]
}

- (id)pathForResource:(id)name            // NSString*
               ofType:(id)extension       // NSString*
          inDirectory:(id)directory       // NSString*
      forLocalization:(id)localization {  // NSString*
    // Try the requested localization's lproj first.
    if localization != nil {
        let lproj_suffix: id = ns_string::get_static_str(env, ".lproj");
        let lproj_dir: id = msg![env; localization stringByAppendingString:lproj_suffix];
        let path =
            path_for_resource_helper(env, this, name, lproj_dir, directory, extension);
        if path != nil {
            return path;
        }
    }
    msg![env; this pathForResource:name ofType:extension inDirectory:directory]
}

- (id)URLForResource:(id)name            // NSString*
       withExtension:(id)extension       // NSString*
        subdirectory:(id)subpath {       // NSString*
    let path_string: id = msg![env; this pathForResource:name
                                                 ofType:extension
                                            inDirectory:subpath];
    if path_string == nil {
        return nil;
    }
    let url: id = msg_class![env; NSURL alloc];
    let url: id = msg![env; url initFileURLWithPath:path_string];
    autorelease(env, url)
}

- (id)URLForResource:(id)name            // NSString*
       withExtension:(id)extension {     // NSString*
    msg![env; this URLForResource:name withExtension:extension subdirectory:nil]
}

- (id)URLForResource:(id)name            // NSString*
       withExtension:(id)extension       // NSString*
        subdirectory:(id)subpath         // NSString*
        localization:(id)localization {  // NSString*
    if localization != nil {
        let lproj_suffix: id = ns_string::get_static_str(env, ".lproj");
        let lproj_dir: id = msg![env; localization stringByAppendingString:lproj_suffix];
        let effective_subpath: id = if subpath != nil {
            msg![env; lproj_dir stringByAppendingPathComponent:subpath]
        } else {
            lproj_dir
        };
        let path: id = msg![env; this pathForResource:name
                                             ofType:extension
                                        inDirectory:effective_subpath];
        if path != nil {
            let url: id = msg_class![env; NSURL alloc];
            let url: id = msg![env; url initFileURLWithPath:path];
            return autorelease(env, url);
        }
    }
    msg![env; this URLForResource:name withExtension:extension subdirectory:subpath]
}

// =========================================================================
// MARK: - Info dictionary
// =========================================================================

- (id)infoDictionary {
    let &NSBundleHostObject {
        bundle_path,
        info_dictionary,
        ..
    } = env.objc.borrow(this);
    if let Some(dict) = info_dictionary {
        return dict;
    }
    let plist_path = ns_string::get_static_str(env, "Info.plist");
    let plist_path: id = msg![env; bundle_path stringByAppendingPathComponent:plist_path];
    let dict: id = msg_class![env; NSDictionary alloc];
    let dict: id = msg![env; dict initWithContentsOfFile:plist_path];
    env.objc.borrow_mut::<NSBundleHostObject>(this).info_dictionary = Some(dict);
    dict
}

- (id)objectForInfoDictionaryKey:(id)key {
    let info_dict: id = msg![env; this infoDictionary];
    msg![env; info_dict objectForKey:key]
}

- (id)localizedInfoDictionary {
    // For now return the plain info dictionary — localized Info.plist
    // (InfoPlist.strings) support can be added later.
    log!("TODO: [NSBundle localizedInfoDictionary] — returning plain infoDictionary");
    msg![env; this infoDictionary]
}

// =========================================================================
// MARK: - Localization
// =========================================================================

- (id)localizedStringForKey:(id)key
                      value:(id)value
                      table:(id)tableName {
    log_dbg!(
        "localizedStringForKey key:'{}' value:'{}' table:'{}'",
        if key == nil { std::borrow::Cow::from("(null)") } else { ns_string::to_rust_string(env, key) },
        if value == nil { std::borrow::Cow::from("(null)") } else { ns_string::to_rust_string(env, value) },
        if tableName == nil { std::borrow::Cow::from("(null)") } else { ns_string::to_rust_string(env, tableName) }
    );
    let empty_str: id = ns_string::get_static_str(env, "");
    if key == nil {
        if value == nil {
            return empty_str;
        }
        return value;
    }
    let name = if tableName == nil {
        ns_string::get_static_str(env, "Localizable")
    } else {
        tableName
    };
    assert_eq!(this, env.framework_state.foundation.ns_bundle.main_bundle.unwrap());

    let dict = if let Some(&table_dict) = env
        .framework_state
        .foundation
        .ns_bundle
        .localization_tables
        .get(&name)
    {
        table_dict
    } else {
        let extension = ns_string::get_static_str(env, "strings");
        let dict_url: id = msg![env; this URLForResource:name withExtension:extension];
        let dict: id = msg_class![env; NSDictionary dictionaryWithContentsOfURL:dict_url];
        if dict == nil {
            if value == nil || value == empty_str {
                return key;
            }
            return value;
        }
        retain(env, name);
        retain(env, dict);
        env.framework_state
            .foundation
            .ns_bundle
            .localization_tables
            .insert(name, dict);
        dict
    };

    let res: id = msg![env; dict objectForKey:key];
    if res == nil {
        if value == nil || value == empty_str {
            return key;
        }
        return value;
    }
    log_dbg!(
        "localizedStringForKey res => {:?}",
        ns_string::to_rust_string(env, res)
    );
    res
}

- (id)localizations {
    let localizations = CFBundleCopyBundleLocalizations(env, this);
    autorelease(env, localizations)
}

- (id)preferredLocalizations {
    let loc_array = CFBundleCopyBundleLocalizations(env, this);
    let preferred = CFBundleCopyPreferredLocalizationsFromArray(env, loc_array);
    autorelease(env, preferred)
}

- (id)developmentLocalization {
    // Read CFBundleDevelopmentRegion from Info.plist; fall back to "en".
    let key: id = ns_string::get_static_str(env, "CFBundleDevelopmentRegion");
    let val: id = msg![env; this objectForInfoDictionaryKey:key];
    if val != nil { val } else { ns_string::get_static_str(env, "en") }
}

- (id)localizedStringForKey:(id)key
                      value:(id)value
                      table:(id)tableName
               localization:(id)_localization {
    // Ignore the explicit localization hint and use the system preference.
    msg![env; this localizedStringForKey:key value:value table:tableName]
}

// =========================================================================
// MARK: - Class lookup
// =========================================================================

- (id)classNamed:(id)class_name { // NSString*
    if class_name == nil {
        return nil;
    }
    let name_str = ns_string::to_rust_string(env, class_name);
    log_dbg!("[NSBundle classNamed:{}]", name_str);

    // Look up via the ObjC runtime.
    let class = env.objc.get_known_class(&name_str, &mut env.mem);

    if class == nil {  // или if !class.is_null() в зависимости от твоей обёртки
        log!(
            "Warning: [NSBundle classNamed:{}] — class not found",
            name_str
        );
        nil
    } else {
        class
    }
}

- (id)principalClass {
    // Read NSPrincipalClass from Info.plist.
    let key: id = ns_string::get_static_str(env, "NSPrincipalClass");
    let val: id = msg![env; this objectForInfoDictionaryKey:key];
    if val == nil {
        return nil;
    }
    msg![env; this classNamed:val]
}

@end

// =========================================================================
// MARK: - _touchHLE_NSBundle_Static
// =========================================================================

@implementation _touchHLE_NSBundle_Static: NSBundle

+ (id)allocWithZone:(NSZonePtr)_zone {
    let bundle_path = env.bundle.bundle_path().as_str().to_string();
    let bundle_path = ns_string::from_rust_string(env, bundle_path);
    let bundle_identifier = env.bundle.bundle_identifier().to_string();
    let bundle_identifier = ns_string::from_rust_string(env, bundle_identifier);
    let host_object = NSBundleHostObject {
        bundle: None,
        bundle_path,
        bundle_identifier,
        bundle_url: None,
        info_dictionary: None,
    };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

- (id)retain    { this }
- (())release   {}
- (id)autorelease { this }

@end

};

fn path_for_resource_helper(
    env: &mut Environment,
    bundle: id,
    name: id,
    lproj: id,
    directory: id,
    extension: id,
) -> id {
    let mut path: id = msg![env; bundle resourcePath];
    if lproj != nil {
        path = msg![env; path stringByAppendingPathComponent:lproj];
    }
    if directory != nil {
        path = msg![env; path stringByAppendingPathComponent:directory];
    }
    path = msg![env; path stringByAppendingPathComponent:name];
    if extension != nil {
        path = msg![env; path stringByAppendingPathExtension:extension];
    }
    let file_manager: id = msg_class![env; NSFileManager defaultManager];
    let file_exists: bool = msg![env; file_manager fileExistsAtPath:path];
    if file_exists {
        return path;
    }

    // Case-insensitive fallback.
    let path_str = ns_string::to_rust_string(env, path);
    let rust_path = std::path::Path::new(path_str.as_ref());

    if let (Some(parent), Some(file_name)) = (rust_path.parent(), rust_path.file_name()) {
        let parent_str = parent.to_str().unwrap_or("");
        let target_name = file_name.to_str().unwrap_or("").to_lowercase();
        let parent_guest_path = crate::fs::GuestPath::new(parent_str);

        let mut found_path = None;
        if let Ok(entries) = env.fs.enumerate(parent_guest_path) {
            for entry in entries {
                let entry_path = std::path::Path::new(entry);
                if let Some(entry_name) = entry_path.file_name() {
                    if entry_name.to_str().unwrap_or("").to_lowercase() == target_name {
                        found_path = Some(entry.to_string());
                        break;
                    }
                }
            }
        }

        if let Some(p) = found_path {
            return ns_string::from_rust_string(env, p);
        }
    }

    nil
}

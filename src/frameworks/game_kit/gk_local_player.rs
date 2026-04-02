/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `GKLocalPlayer`.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::ns_string;
use crate::objc::{id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject, NSZonePtr};

struct GKLocalPlayerHostObject {
    /// `NSString*`
    player_id: id,
    /// `NSString*`
    alias: id,
    /// `NSString*`
    display_name: id,
    authenticated: bool,
    underage: bool,
    /// `NSArray*` of `NSString*` friend player IDs
    friends: id,
}
impl HostObject for GKLocalPlayerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation GKLocalPlayer: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(GKLocalPlayerHostObject {
        player_id: nil,
        alias: nil,
        display_name: nil,
        authenticated: false,
        underage: false,
        friends: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Singleton

+ (id)localPlayer {
    // Return a shared autoreleased stub instance.
    // Real GameKit keeps a per-process singleton; a fresh stub is enough here.
    let player: id = msg![env; this alloc];
    let player: id = msg![env; player init];

    // Set reasonable stub values.
    let player_id = ns_string::from_rust_string(env, "GKLocalPlayer:touchHLE".to_string());
    let alias     = ns_string::from_rust_string(env, "Player".to_string());
    let display   = ns_string::from_rust_string(env, "Player".to_string());
    let friends   = msg_class![env; NSArray new];

    {
        let host = env.objc.borrow_mut::<GKLocalPlayerHostObject>(player);
        host.player_id    = player_id;
        host.alias        = alias;
        host.display_name = display;
        host.friends      = friends;
    }

    crate::objc::autorelease(env, player)
}

// MARK: - Score / achievement convenience (class-level)

+ (())setDefaultLeaderboardIdentifier:(id)_identifier
               withCompletionHandler:(id)_handler {
    log!("GKLocalPlayer setDefaultLeaderboardIdentifier:withCompletionHandler: stubbed");
}

+ (())loadDefaultLeaderboardIdentifierWithCompletionHandler:(id)_handler {
    log!("GKLocalPlayer loadDefaultLeaderboardIdentifierWithCompletionHandler: stubbed");
}

// MARK: - Init / dealloc

- (id)init {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<GKLocalPlayerHostObject>(this);
    let (player_id, alias, display_name, friends) =
        (host.player_id, host.alias, host.display_name, host.friends);
    release(env, player_id);
    release(env, alias);
    release(env, display_name);
    release(env, friends);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Identity

- (id)playerID {
    env.objc.borrow::<GKLocalPlayerHostObject>(this).player_id
}

- (id)alias {
    env.objc.borrow::<GKLocalPlayerHostObject>(this).alias
}

- (id)displayName {
    env.objc.borrow::<GKLocalPlayerHostObject>(this).display_name
}

// MARK: - Authentication state

- (bool)isAuthenticated {
    env.objc.borrow::<GKLocalPlayerHostObject>(this).authenticated
}

- (bool)isUnderage {
    env.objc.borrow::<GKLocalPlayerHostObject>(this).underage
}

// Authenticate with a completion handler (block pointer — treat as opaque id).
- (())authenticateWithCompletionHandler:(id)_completion_handler {
    log!("GKLocalPlayer authenticateWithCompletionHandler: stubbed (authentication never succeeds)");
}

// iOS 6+ replacement.
- (())setAuthenticateHandler:(id)_handler {
    log!("GKLocalPlayer setAuthenticateHandler: stubbed");
}

// MARK: - Friends

- (id)friends {
    env.objc.borrow::<GKLocalPlayerHostObject>(this).friends
}

- (())loadFriendsWithCompletionHandler:(id)_completion_handler {
    log!("GKLocalPlayer loadFriendsWithCompletionHandler: stubbed (no friends)");
}

- (id)description {
    let player_id = env.objc.borrow::<GKLocalPlayerHostObject>(this).player_id;
    let id_str = ns_string::to_rust_string(env, player_id);
    let desc = format!("<GKLocalPlayer: playerID={}>", id_str);
    let ns = ns_string::from_rust_string(env, desc);
    crate::objc::autorelease(env, ns)
}

@end

};

pub const GKPlayerAuthenticationDidChangeNotificationName: &str =
    "GKPlayerAuthenticationDidChangeNotificationName";

pub const CONSTANTS: ConstantExports = &[(
    "_GKPlayerAuthenticationDidChangeNotificationName",
    HostConstant::NSString(GKPlayerAuthenticationDidChangeNotificationName),
)];

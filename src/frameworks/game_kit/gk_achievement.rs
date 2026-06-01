/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `GKAchievement`
//!
//! Stub implementation of the GameKit achievement class. Games use this
//! to report achievement progress to Game Center. Since there is no
//! Game Center backend in touchHLE, all methods are stubs that succeed
//! silently.
//!
//! Reference: <https://developer.apple.com/documentation/gamekit/gkachievement>

use crate::frameworks::foundation::ns_string;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

struct GKAchievementHostObject {
    /// The achievement identifier string (retained NSString*).
    identifier: id,
    /// Percentage complete (0.0 to 100.0).
    percent_complete: f64,
    /// Whether a banner should be shown on completion.
    shows_completion_banner: bool,
}
impl HostObject for GKAchievementHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation GKAchievement: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(GKAchievementHostObject {
        identifier: nil,
        percent_complete: 0.0,
        shows_completion_banner: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Initializers

- (id)initWithIdentifier:(id)identifier {
    retain(env, identifier);
    env.objc.borrow_mut::<GKAchievementHostObject>(this).identifier = identifier;
    this
}

- (id)initWithIdentifier:(id)identifier player:(id)_player {
    retain(env, identifier);
    env.objc.borrow_mut::<GKAchievementHostObject>(this).identifier = identifier;
    this
}

// MARK: - Properties

- (id)identifier {
    env.objc.borrow::<GKAchievementHostObject>(this).identifier
}

- (())setIdentifier:(id)identifier {
    let old = env.objc.borrow::<GKAchievementHostObject>(this).identifier;
    release(env, old);
    retain(env, identifier);
    env.objc.borrow_mut::<GKAchievementHostObject>(this).identifier = identifier;
}

- (f64)percentComplete {
    env.objc.borrow::<GKAchievementHostObject>(this).percent_complete
}

- (())setPercentComplete:(f64)value {
    env.objc.borrow_mut::<GKAchievementHostObject>(this).percent_complete = value;
}

- (bool)showsCompletionBanner {
    env.objc.borrow::<GKAchievementHostObject>(this).shows_completion_banner
}

- (())setShowsCompletionBanner:(bool)value {
    env.objc.borrow_mut::<GKAchievementHostObject>(this).shows_completion_banner = value;
}

- (bool)isCompleted {
    env.objc.borrow::<GKAchievementHostObject>(this).percent_complete >= 100.0
}

// lastReportedDate — return nil (no backend)
- (id)lastReportedDate {
    nil
}

// MARK: - Reporting

// - (void)reportAchievementWithCompletionHandler:(void (^)(NSError *))completionHandler
- (())reportAchievementWithCompletionHandler:(id)_handler {
    let ident = env.objc.borrow::<GKAchievementHostObject>(this).identifier;
    let pct = env.objc.borrow::<GKAchievementHostObject>(this).percent_complete;
    if ident != nil {
        let s = ns_string::to_rust_string(env, ident);
        log!(
            "GKAchievement reportAchievementWithCompletionHandler: stubbed (id={}, pct={})",
            s, pct
        );
    } else {
        log!(
            "GKAchievement reportAchievementWithCompletionHandler: stubbed (id=nil, pct={})",
            pct
        );
    }
    // If completionHandler is non-nil, we should invoke it with nil error.
    // Block invocation is complex in touchHLE — just skip for now.
}

// MARK: - Class methods

// + (void)loadAchievementsWithCompletionHandler:(void (^)(NSArray *, NSError *))handler
+ (())loadAchievementsWithCompletionHandler:(id)_handler {
    log!("GKAchievement loadAchievementsWithCompletionHandler: stubbed (returning empty)");
    // The handler block should be called with (emptyArray, nil) but block
    // invocation is not trivial — games typically handle nil/no-op gracefully.
}

// + (void)resetAchievementsWithCompletionHandler:(void (^)(NSError *))handler
+ (())resetAchievementsWithCompletionHandler:(id)_handler {
    log!("GKAchievement resetAchievementsWithCompletionHandler: stubbed");
    // Same as above — the handler should be called with nil error.
}

// + (void)reportAchievements:(NSArray *)achievements
//       withCompletionHandler:(void (^)(NSError *))handler
+ (())reportAchievements:(id)_achievements withCompletionHandler:(id)_handler {
    log!("GKAchievement reportAchievements:withCompletionHandler: stubbed");
}

// MARK: - Dealloc

- (())dealloc {
    let ident = env.objc.borrow::<GKAchievementHostObject>(this).identifier;
    release(env, ident);
    env.objc.dealloc_object(this, &mut env.mem);
}

// MARK: - Description

- (id)description {
    let ident = env.objc.borrow::<GKAchievementHostObject>(this).identifier;
    let pct = env.objc.borrow::<GKAchievementHostObject>(this).percent_complete;
    let ident_s = if ident != nil {
        ns_string::to_rust_string(env, ident).into_owned()
    } else {
        "<nil>".to_string()
    };
    let desc = format!("<GKAchievement: id={} percentComplete={}>", ident_s, pct);
    let ns = ns_string::from_rust_string(env, desc);
    autorelease(env, ns)
}

@end

};

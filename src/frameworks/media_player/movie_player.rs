/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMoviePlayerController` etc.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::{ns_string, ns_url, NSInteger};
use crate::frameworks::uikit::ui_device::UIDeviceOrientation;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, todo_objc_setter, ClassExports,
    HostObject, NSZonePtr,
};
use crate::Environment;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct State {
    active_player: Option<id>,
    /// Various apps (e.g. Crash Bandicoot Nitro Kart 3D and Spore Origins)
    /// create or start a player and await some kind of notification, but can't
    /// handle it if that notification happens immediately.
    /// This queue lets us
    /// delay such notifications until the app next returns to the run loop,
    /// which seems to be late enough.
    pending_notifications: VecDeque<(&'static str, id, Instant)>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.media_player.movie_player
    }
}

type MPMovieScalingMode = NSInteger;
type MPMovieControlStyle = NSInteger;
type MPMovieSourceType = NSInteger;
type MPMovieRepeatMode = NSInteger;

type MPMoviePlaybackState = NSInteger;
const MPMoviePlaybackStateStopped: MPMoviePlaybackState = 0;
// Values might not be correct, but as these are linked symbol constants, it
// shouldn't matter.
pub const MPMoviePlayerPlaybackDidFinishNotification: &str =
    "MPMoviePlayerPlaybackDidFinishNotification";
/// Apparently an undocumented, private API. Spore Origins uses it.
pub const MPMoviePlayerContentPreloadDidFinishNotification: &str =
    "MPMoviePlayerContentPreloadDidFinishNotification";
pub const MPMoviePlayerScalingModeDidChangeNotification: &str =
    "MPMoviePlayerScalingModeDidChangeNotification";
// TODO: More notifications?
const MPMoviePlayerPlaybackDidFinishReasonUserInfoKey: &str =
    "MPMoviePlayerPlaybackDidFinishReasonUserInfoKey";

/// `NSNotificationName` values and other constants.
pub const CONSTANTS: ConstantExports = &[
    (
        "_MPMoviePlayerPlaybackDidFinishNotification",
        HostConstant::NSString(MPMoviePlayerPlaybackDidFinishNotification),
    ),
    (
        "_MPMoviePlayerContentPreloadDidFinishNotification",
        HostConstant::NSString(MPMoviePlayerContentPreloadDidFinishNotification),
    ),
    (
        "_MPMoviePlayerScalingModeDidChangeNotification",
        HostConstant::NSString(MPMoviePlayerScalingModeDidChangeNotification),
    ),
    (
        "_MPMoviePlayerPlaybackDidFinishReasonUserInfoKey",
        HostConstant::NSString(MPMoviePlayerPlaybackDidFinishReasonUserInfoKey),
    ),
];

struct MPMoviePlayerControllerHostObject {
    // NSURL *
    content_url: id,
    // UIView *
    view: id,
    scaling_mode: MPMovieScalingMode,
    control_style: MPMovieControlStyle,
    source_type: MPMovieSourceType,
    repeat_mode: MPMovieRepeatMode,
    should_autoplay: bool,
    initial_playback_time: f64,
}
impl HostObject for MPMoviePlayerControllerHostObject {}

/// Ensure the player has a valid dummy view, creating one lazily if needed.
/// Returns the view id (always non-nil after this call).
fn ensure_view(env: &mut Environment, this: id) -> id {
    let existing = env
        .objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .view;
    if existing != nil {
        return existing;
    }
    let view_alloc: id = msg_class![env; UIView alloc];
    let view: id = msg![env; view_alloc init];
    retain(env, view);
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .view = view;
    view
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMoviePlayerController: NSObject

// TODO: actual playback

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(MPMoviePlayerControllerHostObject {
        content_url: nil,
        view: nil,
        scaling_mode: 0,
        control_style: 0,
        source_type: 0,
        repeat_mode: 0,
        should_autoplay: true,
        initial_playback_time: -1.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithContentURL:(id)url { // NSURL*
    log!(
        "TODO: [(MPMoviePlayerController*){:?} initWithContentURL:{:?} ({:?})]",
        this,
        url,
        ns_url::to_rust_path(env, url),
    );
    retain(env, url);

    // Create a dummy view to avoid null pointer dereferences when the app
    // accesses player.view.  We retain it explicitly so the host object
    // conceptually owns it.
    let view_alloc: id = msg_class![env; UIView alloc];
    let view: id = msg![env; view_alloc init];
    retain(env, view);

    {
        let host = env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this);
        host.content_url = url;
        host.view = view;
    }

    // Act as if loading immediately completed (Spore Origins waits for this).
    State::get(env).pending_notifications.push_back((
        MPMoviePlayerContentPreloadDidFinishNotification,
        this,
        Instant::now(),
    ));

    this
}

- (())dealloc {
    let url = env
        .objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .content_url;
    release(env, url);

    let view = env
        .objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .view;
    release(env, view);

    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)contentURL {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .content_url
}

- (id)backgroundColor {
    msg_class![env; UIColor blackColor] // TODO
}
- (())setBackgroundColor:(id)color { // UIColor*
    todo_objc_setter!(this, color);
}

// --- Scaling mode ---

- (MPMovieScalingMode)scalingMode {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .scaling_mode
}
- (())setScalingMode:(MPMovieScalingMode)mode {
    log!(
        "TODO: [(MPMoviePlayerController*){:?} setScalingMode:{:?}]",
        this,
        mode
    );
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .scaling_mode = mode;
}

// --- Control style ---

- (MPMovieControlStyle)controlStyle {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .control_style
}
- (())setControlStyle:(MPMovieControlStyle)style {
    log!(
        "TODO: [(MPMoviePlayerController*){:?} setControlStyle:{:?}]",
        this,
        style
    );
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .control_style = style;
}

// --- Source type ---

- (MPMovieSourceType)movieSourceType {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .source_type
}
- (())setMovieSourceType:(MPMovieSourceType)source_type {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .source_type = source_type;
}

// --- Repeat mode ---

- (MPMovieRepeatMode)repeatMode {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .repeat_mode
}
- (())setRepeatMode:(MPMovieRepeatMode)mode {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .repeat_mode = mode;
}

// --- Autoplay ---

- (bool)shouldAutoplay {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .should_autoplay
}
- (())setShouldAutoplay:(bool)autoplay {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .should_autoplay = autoplay;
}

// --- Misc setters ---

- (())setUseApplicationAudioSession:(bool)use_session {
    todo_objc_setter!(this, use_session);
}

- (())setFullscreen:(bool)fullscreen {
    todo_objc_setter!(this, fullscreen);
}

- (())setFullscreen:(bool)fullscreen animated:(bool)animated {
    log!(
        "TODO: [(MPMoviePlayerController*){:?} setFullscreen:{:?} animated:{:?}]",
        this,
        fullscreen,
        animated
    );
}

// --- View ---

// Returns the player's backing view. Created lazily if initWithContentURL:
// somehow failed to allocate it, so this always returns a non-nil UIView.
- (id)view {
    ensure_view(env, this)
}

- (id)backgroundView {
    nil // TODO
}
- (())setBackgroundView:(id)view {
    todo_objc_setter!(this, view);
}

// --- Playback state / time ---

- (MPMoviePlaybackState)playbackState {
    MPMoviePlaybackStateStopped // TODO
}

- (f64)currentPlaybackTime {
    0.0 // TODO
}
- (())setCurrentPlaybackTime:(f64)time {
    todo_objc_setter!(this, time);
}

- (f64)initialPlaybackTime {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .initial_playback_time
}
- (())setInitialPlaybackTime:(f64)time {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .initial_playback_time = time;
}

- (f64)duration {
    0.0 // TODO
}

- (())prepareToPlay {
    // Act as if we are immediately prepared; no real playback yet.
}

// Apparently an undocumented, private API, but Spore Origins uses it.
- (())setMovieControlMode:(NSInteger)_mode {
    // As this is undocumented and we don't have real video playback yet, let's
    // ignore it.
}

// Another undocumented one! But some apps may still use it :/
// https://stackoverflow.com/a/1390079/2241008
- (())setOrientation:(UIDeviceOrientation)_orientation animated:(bool)_animated {
}

// MPMediaPlayback implementation
- (())play {
    log!("TODO: [(MPMoviePlayerController*){:?} play]", this);
    if let Some(old) = env.framework_state.media_player.movie_player.active_player {
        let _: () = msg![env; old stop];
    }
    assert!(env
        .framework_state
        .media_player
        .movie_player
        .active_player
        .is_none());
    // Movie player is retained by the runtime until it is stopped
    retain(env, this);
    env.framework_state.media_player.movie_player.active_player = Some(this);

    // Act as if playback immediately completed after 1 second
    // (various apps wait for this, such as BIA and Hero of Sparta).
    let notif = (
        MPMoviePlayerPlaybackDidFinishNotification,
        this,
        Instant::now()
            .checked_add(Duration::from_millis(1000))
            .unwrap(),
    );
    for (name, obj, _) in &mut State::get(env).pending_notifications {
        // De-duplicate similar notifications.
        // This can happen if app is calling
        // `play` twice on the same player object (case of NOVA2).
        if *name == MPMoviePlayerPlaybackDidFinishNotification && *obj == this {
            return;
        }
    }
    State::get(env).pending_notifications.push_back(notif);
}

- (())pause {
    log!("TODO: [(MPMoviePlayerController*){:?} pause]", this);
}

- (())stop {
    log!("TODO: [(MPMoviePlayerController*){:?} stop]", this);
    if env
        .framework_state
        .media_player
        .movie_player
        .active_player
        .is_some()
    {
        // Some applications (like NOVA2) may send 2 `stop` messages for each
        // 1 `play` message for the player.
        // In that case, we want to release
        // the active player only once.
        assert!(
            this == env
                .framework_state
                .media_player
                .movie_player
                .active_player
                .take()
                .unwrap()
        );
        release(env, this);
    }
}

@end

@implementation MPMoviePlayerViewController: UIViewController

- (id)initWithContentURL:(id)url {
    log!(
        "TODO: [(MPMoviePlayerViewController*){:?} initWithContentURL:{:?} ({:?})]",
        this,
        url,
        ns_url::to_rust_path(env, url),
    );
    // Call designated initializer of UIViewController superclass
    let this: id = msg![env; this init];
    this
}

@end

};

/// For use by `NSRunLoop` via [super::handle_players]: check movie players'
/// status, send notifications if necessary.
pub(super) fn handle_players(env: &mut Environment) {
    let mut notifs_to_run = Vec::new();
    let pending_notifs = &mut State::get(env).pending_notifications;
    let mut i = 0;
    while i < pending_notifs.len() {
        let (name_str, object, time) = pending_notifs[i];
        if Instant::now() >= time {
            notifs_to_run.push((name_str, object));
            pending_notifs.swap_remove_back(i);
        } else {
            i += 1;
        }
    }
    for (name_str, object) in notifs_to_run {
        let name = ns_string::get_static_str(env, name_str);
        let center: id = msg_class![env; NSNotificationCenter defaultCenter];
        // TODO: should there be some user info attached?
        let _: () = msg![env; center postNotificationName:name object:object];
    }
}


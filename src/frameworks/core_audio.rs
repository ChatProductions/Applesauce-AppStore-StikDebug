// core_audio.rs
/*
* Stub implementation for CoreAudio.framework
*
* Purpose:
* - satisfy symbol lookup
* - keep old iOS games from failing on audio-session startup
* - safely no-op when audio is not emulated
*/

use crate::dyld::{export_c_func, FunctionExports};
use crate::Environment;
use crate::mem::{ConstPtr, MutPtr};

// Optional: keep this if your tree already has the type definitions.
// If the file compiles without it, you can remove the line below.
use super::core_audio_types::*;

fn AudioSessionInitialize(
    _env: &mut Environment,
    _in_run_loop: MutPtr<u8>,
    _in_run_loop_mode: ConstPtr<u8>,
    _in_interruption_listener: u32,
    _in_client_data: MutPtr<u8>,
) -> u32 {
    0
}

fn AudioSessionSetActive(_env: &mut Environment, _active: bool) -> u32 {
    0
}

fn AudioSessionGetProperty(
    _env: &mut Environment,
    _property_id: u32,
    _data_size: MutPtr<u32>,
    _data: MutPtr<u8>,
) -> u32 {
    0
}

fn AudioSessionSetProperty(
    _env: &mut Environment,
    _property_id: u32,
    _data_size: u32,
    _data: ConstPtr<u8>,
) -> u32 {
    0
}

fn AudioSessionAddPropertyListener(
    _env: &mut Environment,
    _property_id: u32,
    _listener: u32,
    _client_data: MutPtr<u8>,
) -> u32 {
    0
}

fn AudioSessionRemovePropertyListener(
    _env: &mut Environment,
    _property_id: u32,
    _listener: u32,
) -> u32 {
    0
}

fn AudioServicesPlaySystemSound(_env: &mut Environment, _sound_id: u32) {
    // No-op.
}

fn AudioServicesDisposeSystemSoundID(_env: &mut Environment, _sound_id: u32) {
    // No-op.
}

fn AudioServicesCreateSystemSoundID(
    _env: &mut Environment,
    _in_file_url: u32,
    _out_system_sound_id: MutPtr<u32>,
) -> u32 {
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioSessionInitialize(_, _, _, _)),
    export_c_func!(AudioSessionSetActive(_)),
    export_c_func!(AudioSessionGetProperty(_, _, _)),
    export_c_func!(AudioSessionSetProperty(_, _, _)),
    export_c_func!(AudioSessionAddPropertyListener(_, _, _)),
    export_c_func!(AudioSessionRemovePropertyListener(_, _)),
    export_c_func!(AudioServicesPlaySystemSound(_)),
    export_c_func!(AudioServicesDisposeSystemSoundID(_)),
    export_c_func!(AudioServicesCreateSystemSoundID(_, _)),
];

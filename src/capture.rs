//! The tools needed to capture user input for later playback.

use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::prelude::*;
use bevy_input::gamepad::GamepadEventRaw;
use std::path::PathBuf;
use winit::event::WindowEvent;

use crate::runners::capture_runner;

/// Enables user input capture of both [`winit`] and gamepad events
///
/// # Warning
///
/// This plugin must be added after `DefaultPlugins` or `WinitPlugin` in order to override the app's runner correctly.
pub struct InputCapturePlugin {
    /// The file path that the serialized user input will be recorded to
    pub file_path: PathBuf,
}

impl Plugin for InputCapturePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WindowEvent>()
            .add_event::<GamepadEventRaw>()
            .set_runner(capture_runner)
            .add_system_to_stage(CoreStage::Last, record_gamepad_events)
            // This ordering is arbtirary, but nondeterminism is bad
            .add_system_to_stage(
                CoreStage::Last,
                record_window_events.after(record_gamepad_events),
            );
    }
}

impl Default for InputCapturePlugin {
    fn default() -> Self {
        InputCapturePlugin {
            file_path: "input_playback.ron".into(),
        }
    }
}

/// Captures [`GamepadEventRaw`]
///
/// Intended to be run in [`CoreStage::Last`] to ensure that any user-generated synthetic input is captured.
pub fn record_gamepad_events() {}

/// Captures [`WindowEvent`]
///
/// Intended to be run in [`CoreStage::Last`] to ensure that any user-generated synthetic input is captured.
pub fn record_window_events() {}

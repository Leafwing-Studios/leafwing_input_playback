#![forbid(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

use bevy_app::{App, Plugin};
use bevy_input::gamepad::GamepadEventRaw;
use std::path::PathBuf;
use winit::event::WindowEvent;

pub mod playback_data;
pub mod runners;

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
            .set_runner(runners::capture_runner);
    }
}

impl Default for InputCapturePlugin {
    fn default() -> Self {
        InputCapturePlugin {
            file_path: "input_playback.ron".into(),
        }
    }
}

/// Enables user input playback from a serialized file
///
/// # Warning
///
/// This plugin must be added after `DefaultPlugins` or `WinitPlugin` in order to override the app's runner correctly.
pub struct InputPlaybackPlugin {
    /// The file path that the serialized user input will be read from
    pub file_path: PathBuf,
}

impl Plugin for InputPlaybackPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WindowEvent>()
            .add_event::<GamepadEventRaw>()
            .set_runner(runners::playback_runner);
    }
}

impl Default for InputPlaybackPlugin {
    fn default() -> Self {
        InputPlaybackPlugin {
            file_path: "input_playback.ron".into(),
        }
    }
}

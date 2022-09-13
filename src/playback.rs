//! The tools needed to play back captured user input.

use bevy_app::{App, Plugin};
use bevy_input::gamepad::GamepadEventRaw;
use std::path::PathBuf;
use winit::event::WindowEvent;

use crate::runners::playback_runner;

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
            .set_runner(playback_runner);
    }
}

impl Default for InputPlaybackPlugin {
    fn default() -> Self {
        InputPlaybackPlugin {
            file_path: "input_playback.ron".into(),
        }
    }
}

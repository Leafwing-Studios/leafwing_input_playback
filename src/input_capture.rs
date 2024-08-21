//! Captures user input from assorted raw [`Event`](bevy::ecs::event::Event) types.
//!
//! These are unified into a single [`TimestampedInputs`](crate::timestamped_input::TimestampedInputs) resource, which can be played back.

use bevy::app::{App, AppExit, First, Last, Plugin};
use bevy::ecs::prelude::*;
use bevy::input::gamepad::GamepadEvent;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::time::Time;
use bevy::window::CursorMoved;
use ron::ser::PrettyConfig;

use crate::frame_counting::{frame_counter, FrameCount};
use crate::serde::PlaybackFilePath;
use crate::timestamped_input::TimestampedInputs;
use std::fs::OpenOptions;
use std::io::Write;

/// Captures user inputs from the assorted raw `Event` types
///
/// These are collected into a [`TimestampedInputs`](crate::timestamped_input::TimestampedInputs) resource.
/// Which input modes (mouse, keyboard, etc) are captured is controlled via the [`InputModesCaptured`] resource.
///
/// Input is serialized into the path stored in the [`PlaybackFilePath`] resource, if any.
pub struct InputCapturePlugin;

impl Plugin for InputCapturePlugin {
    fn build(&self, app: &mut App) {
        // Avoid double-adding frame_counter
        if !app.world().contains_resource::<FrameCount>() {
            app.init_resource::<FrameCount>()
                .add_systems(First, frame_counter);
        }

        app.init_resource::<TimestampedInputs>()
            .init_resource::<InputModesCaptured>()
            .init_resource::<PlaybackFilePath>()
            .add_systems(
                Last,
                (
                    // Capture any mocked input as well
                    capture_input,
                    serialize_captured_input_on_exit,
                )
                    .chain(),
            );
    }
}

/// The input mechanisms captured via the [`InputCapturePlugin`], configured as a resource.
///
/// By default, all supported input modes will be captured.
#[derive(Resource, Debug, PartialEq, Eq, Clone)]
pub struct InputModesCaptured {
    /// Mouse buttons and mouse wheel inputs
    pub mouse_buttons: bool,
    /// Moving the mouse
    pub mouse_motion: bool,
    /// Keyboard inputs
    ///
    /// Captures both keycode and scan code data.
    pub keyboard: bool,
    /// Gamepad inputs
    ///
    /// Captures gamepad connections, button presses and axis values
    pub gamepad: bool,
}

impl InputModesCaptured {
    /// Disables all input capturing
    pub const DISABLE_ALL: InputModesCaptured = InputModesCaptured {
        mouse_buttons: false,
        mouse_motion: false,
        keyboard: false,
        gamepad: false,
    };

    /// Captures all supported input modes
    pub const ENABLE_ALL: InputModesCaptured = InputModesCaptured {
        mouse_buttons: true,
        mouse_motion: true,
        keyboard: true,
        gamepad: true,
    };
}

impl Default for InputModesCaptured {
    fn default() -> Self {
        InputModesCaptured::ENABLE_ALL
    }
}

/// Captures input from the [`bevy::window`] and [`bevy::input`] event streams.
///
/// The input modes can be controlled via the [`InputModesCaptured`] resource.
#[allow(clippy::too_many_arguments)]
pub fn capture_input(
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut keyboard_events: EventReader<KeyboardInput>,
    mut gamepad_events: EventReader<GamepadEvent>,
    mut app_exit_events: EventReader<AppExit>,
    mut timestamped_input: ResMut<TimestampedInputs>,
    input_modes_captured: Res<InputModesCaptured>,
    frame_count: Res<FrameCount>,
    time: Res<Time>,
) {
    let time_since_startup = time.elapsed();
    let frame = *frame_count;

    // BLOCKED: these events are arbitrarily ordered within a frame,
    // but we have no way to access their order from winit.
    // See https://github.com/bevyengine/bevy/issues/5984

    if input_modes_captured.mouse_buttons {
        timestamped_input.send_multiple(
            frame,
            time_since_startup,
            mouse_button_events.read().cloned(),
        );

        timestamped_input.send_multiple(
            frame,
            time_since_startup,
            mouse_wheel_events.read().cloned(),
        );
    } else {
        mouse_button_events.clear();
        mouse_wheel_events.clear();
    }

    if input_modes_captured.mouse_motion {
        timestamped_input.send_multiple(
            frame,
            time_since_startup,
            cursor_moved_events.read().cloned(),
        );
    } else {
        cursor_moved_events.clear();
    }

    if input_modes_captured.keyboard {
        timestamped_input.send_multiple(frame, time_since_startup, keyboard_events.read().cloned());
    } else {
        keyboard_events.clear()
    }

    if input_modes_captured.gamepad {
        timestamped_input.send_multiple(frame, time_since_startup, gamepad_events.read().cloned());
    } else {
        gamepad_events.clear()
    }

    timestamped_input.send_multiple(frame, time_since_startup, app_exit_events.read().cloned())
}

/// Serializes captured input to the path given in the [`PlaybackFilePath`] resource.
///
/// This data is only serialized once when [`AppExit`] is sent.
/// Use the [`serialized_timestamped_inputs`] function directly if you want to implement custom checkpointing strategies.
pub fn serialize_captured_input_on_exit(
    app_exit_events: EventReader<AppExit>,
    playback_file: Res<PlaybackFilePath>,
    captured_inputs: Res<TimestampedInputs>,
) {
    if !app_exit_events.is_empty() {
        serialize_timestamped_inputs(&captured_inputs, &playback_file);
    }
}

/// Writes the `timestamped_inputs` to the provided `path` (which should store [`Some(PathBuf)`]).
pub fn serialize_timestamped_inputs(
    timestamped_inputs: &TimestampedInputs,
    playback_file: &PlaybackFilePath,
) {
    if let Some(file_path) = playback_file.path() {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(file_path)
            .expect("Could not open file.");
        write!(
            file,
            "{}",
            ron::ser::to_string_pretty(timestamped_inputs, PrettyConfig::default())
                .expect("Could not convert captured input to a string.")
        )
        .expect("Could not write string to file.");
    }
}

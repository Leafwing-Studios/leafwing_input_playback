//! Captures user input from assorted raw [`Event`] types.
//!
//! These are unified into a single [`UnifiedInput`] resource, which can be played back.

use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::prelude::*;
use bevy_input::keyboard::KeyboardInput;
use bevy_input::mouse::{MouseButtonInput, MouseWheel};
use bevy_time::Time;
use bevy_window::CursorMoved;

use crate::unified_input::{FrameCount, UnifiedInput};

/// Captures user inputs from the assorted raw `Event` types
///
/// These are collected into a [`UnifiedInput`] event stream.
/// Which input modes (mouse, keyboard, etc) are captured is controlled via the [`InputModesCaptured`] resource.
pub struct InputCapturePlugin;

impl Plugin for InputCapturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameCount>()
            .init_resource::<UnifiedInput>()
            .add_system_to_stage(CoreStage::First, frame_counter)
            .add_system_to_stage(
                // Capture any mocked input as well
                CoreStage::Last,
                capture_input,
            );
    }
}

/// The input mechanisms captured via the [`InputCapturePlugin`].
///
/// By default, all supported input modes will be captured.
#[derive(Debug, PartialEq, Clone)]
pub struct InputModesCaptured {
    /// Mouse buttons and mouse wheel inputs
    pub mouse_buttons: bool,
    /// Moving the mouse
    pub mouse_motion: bool,
    /// Keyboard inputs (both keycodes and scancodes)
    pub keyboard: bool,
}

impl Default for InputModesCaptured {
    fn default() -> Self {
        InputModesCaptured {
            mouse_buttons: true,
            mouse_motion: true,
            keyboard: true,
        }
    }
}

/// Increases the value of the [`FrameCount`] resource by 1 every frame
///
/// This system should run during [`CoreStage::First`].
pub fn frame_counter(mut frame_count: ResMut<FrameCount>) {
    frame_count.0 += 1;
}

/// Captures input from the [`bevy_window`] and [`bevy_input`] event streams.
///
/// The input modes can be controlled via the [`InputModesCaptured`] resource.
pub fn capture_input(
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut keyboard_events: EventReader<KeyboardInput>,
    mut unified_input: ResMut<UnifiedInput>,
    input_modes_captured: Res<InputModesCaptured>,
    frame_count: Res<FrameCount>,
    time: Res<Time>,
) {
    let time_since_startup = time.time_since_startup();
    let frame = *frame_count;

    // BLOCKED: these events are arbitrarily ordered within a frame,
    // but we have no way to access their order from winit.
    // See https://github.com/bevyengine/bevy/issues/5984

    if input_modes_captured.mouse_buttons {
        unified_input.send_multiple(
            frame,
            time_since_startup,
            mouse_button_events.iter().cloned(),
        );

        unified_input.send_multiple(
            frame,
            time_since_startup,
            mouse_wheel_events.iter().cloned(),
        );
    }

    if input_modes_captured.mouse_motion {
        unified_input.send_multiple(
            frame,
            time_since_startup,
            cursor_moved_events.iter().cloned(),
        );
    }

    if input_modes_captured.keyboard {
        unified_input.send_multiple(frame, time_since_startup, keyboard_events.iter().cloned());
    }
}

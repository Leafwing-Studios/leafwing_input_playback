//! Captures user input from assorted raw [`Event`](bevy::ecs::event::Event) types.
//!
//! These are unified into a single [`TimestampedInputs`](crate::timestamped_input::TimestampedInputs) resource, which can be played back.

use bevy::app::{App, AppExit, Last, Plugin};
use bevy::core::{update_frame_count, FrameCount};
use bevy::ecs::prelude::*;
use bevy::input::gamepad::GamepadEvent;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::time::Time;
use bevy::window::CursorMoved;
use ron::ser::PrettyConfig;

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
        app.observe(BeginInputCapture::observer)
            .observe(EndInputCapture::observer)
            .add_systems(
                Last,
                (
                    // Capture any mocked input as well
                    capture_input,
                    handle_final_capture_frame.run_if(resource_exists::<FinalCaptureFrame>),
                )
                    .chain()
                    .before(update_frame_count),
            );
    }
}

/// An Event that users can send to initiate input capture.
///
/// Data is serialized to the provided `filepath` when either an [`EndInputCapture`] or an [`AppExit`] event is detected.
#[derive(Debug, Default, Event)]
pub struct BeginInputCapture {
    /// The input mechanisms that will be captured, see [`InputModesCaptured`].
    pub input_modes_captured: InputModesCaptured,
    /// The filepath at which to serialize captured input data.
    pub filepath: Option<String>,
    /// The number of frames for which inputs should be captured.
    /// If None, inputs will be captured until an [`EndInputCapture`] or [`AppExit`] event is detected.
    pub frames_to_capture: Option<FrameCount>,
    /// A `Window` entity which acts as a filter for which inputs will be captured.
    /// This data will not be serialized, so that a target window can be selected on playback.
    pub window_to_capture: Option<Entity>,
}

impl BeginInputCapture {
    /// Initiates input capture when a [`BeginInputCapture`] is detected.
    pub fn observer(trigger: Trigger<Self>, mut commands: Commands, frame_count: Res<FrameCount>) {
        let event = trigger.event();
        commands.init_resource::<TimestampedInputs>();
        commands.insert_resource(event.input_modes_captured.clone());
        if let Some(path) = &event.filepath {
            commands.insert_resource(PlaybackFilePath::new(path));
        }
        if let Some(final_frame) = event.frames_to_capture {
            commands.insert_resource(FinalCaptureFrame(FrameCount(
                frame_count.0.wrapping_add(final_frame.0),
            )));
        }
        if let Some(window_entity) = &event.window_to_capture {
            commands.insert_resource(InputCaptureWindow(*window_entity));
        }
    }
}

/// An Observer Trigger that users can send to end input capture and serialize data to disk.
#[derive(Debug, Event)]
pub struct EndInputCapture;

impl EndInputCapture {
    /// An ObserverSystem for `EndInputCapture` that removes all capture-related resources and serializes timestamps if `PlaybackFilePath` exists
    pub fn observer(
        _trigger: Trigger<Self>,
        mut commands: Commands,
        captured_inputs: Res<TimestampedInputs>,
        playback_file: Option<Res<PlaybackFilePath>>,
    ) {
        // if a PlaybackFilePath exists, serialize `TimestampedInputs` and remove it
        if let Some(playback_file) = playback_file.as_deref() {
            serialize_timestamped_inputs(&captured_inputs, playback_file);
            commands.remove_resource::<TimestampedInputs>();
            commands.remove_resource::<PlaybackFilePath>();
        }
        // also remove capture-related resources
        commands.remove_resource::<InputModesCaptured>();
        commands.remove_resource::<FinalCaptureFrame>();
        commands.remove_resource::<InputCaptureWindow>();
    }
}

/// The final [`FrameCount`] at which inputs will stop being captured.
///
/// If this Resource is attached, [`TimestampedInputs`] will be serialized and input capture will stop once `FrameCount` reaches this value.
#[derive(Debug, Resource)]
pub struct FinalCaptureFrame(FrameCount);

/// The `Window` entity for which inputs will be captured.
///
/// If this Resource is attached, only input events on the window corresponding to this entity will be captured.
#[derive(Debug, Resource)]
pub struct InputCaptureWindow(Entity);

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
    window_to_capture: Option<Res<InputCaptureWindow>>,
    input_modes_captured: Option<Res<InputModesCaptured>>,
    frame_count: Res<FrameCount>,
    time: Res<Time>,
) {
    let Some(input_modes_captured) = input_modes_captured else {
        mouse_button_events.clear();
        mouse_wheel_events.clear();
        cursor_moved_events.clear();
        keyboard_events.clear();
        gamepad_events.clear();
        app_exit_events.clear();
        return;
    };

    let time_since_startup = time.elapsed();
    let frame = *frame_count;

    // BLOCKED: these events are arbitrarily ordered within a frame,
    // but we have no way to access their order from winit.
    // See https://github.com/bevyengine/bevy/issues/5984

    if input_modes_captured.mouse_buttons {
        timestamped_input.send_multiple(
            frame,
            time_since_startup,
            mouse_button_events
                .read()
                .filter(|event| {
                    window_to_capture
                        .as_deref()
                        .map(|window| window.0 == event.window)
                        .unwrap_or(true)
                })
                .cloned(),
        );

        timestamped_input.send_multiple(
            frame,
            time_since_startup,
            mouse_wheel_events
                .read()
                .filter(|event| {
                    window_to_capture
                        .as_deref()
                        .map(|window| window.0 == event.window)
                        .unwrap_or(true)
                })
                .cloned(),
        );
    } else {
        mouse_button_events.clear();
        mouse_wheel_events.clear();
    }

    if input_modes_captured.mouse_motion {
        timestamped_input.send_multiple(
            frame,
            time_since_startup,
            cursor_moved_events
                .read()
                .filter(|event| {
                    window_to_capture
                        .as_deref()
                        .map(|window| window.0 == event.window)
                        .unwrap_or(true)
                })
                .cloned(),
        );
    } else {
        cursor_moved_events.clear();
    }

    if input_modes_captured.keyboard {
        timestamped_input.send_multiple(
            frame,
            time_since_startup,
            keyboard_events
                .read()
                .filter(|event| {
                    window_to_capture
                        .as_deref()
                        .map(|window| window.0 == event.window)
                        .unwrap_or(true)
                })
                .cloned(),
        );
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

/// Serializes captured input to the path given in the [`PlaybackFilePath`] resource once [`AppExit`] is sent.
pub fn trigger_input_capture_on_exit(
    app_exit_events: EventReader<AppExit>,
    mut commands: Commands,
) {
    if !app_exit_events.is_empty() {
        commands.trigger(EndInputCapture);
    }
}

/// Triggers `EndInputCapture` once the provided number of frames have elapsed.
pub fn handle_final_capture_frame(
    mut commands: Commands,
    frame_count: Res<FrameCount>,
    final_frame: Res<FinalCaptureFrame>,
) {
    if *frame_count == final_frame.0 {
        commands.trigger(EndInputCapture);
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
            .truncate(true)
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

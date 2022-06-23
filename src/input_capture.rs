//! Captures user input from assorted raw `Event` types.
//!
//! These are unified into a single [`UnifiedInput`] event stream, which can be played back
use crate::axislike::AxisPairType;
use crate::user_input::InputButton;

use bevy_app::{App, CoreStage, Plugin};
use bevy_core::Time;
use bevy_ecs::prelude::*;
use bevy_input::gamepad::{Gamepad, GamepadAxisType, GamepadEvent, GamepadEventType};
use bevy_input::keyboard::KeyboardInput;
use bevy_input::mouse::{MouseButtonInput, MouseWheel};
use bevy_input::ElementState;
/// via the provided [`input_mocking`](crate::input_mocking) functionality
use bevy_utils::Duration;
use bevy_window::CursorMoved;

/// A timestamped device-agnostic user-input event
///
/// These are re-emitted as events, and commonly serialized to disk
#[derive(Debug, Clone)]
pub struct UnifiedInput {
    /// The number of frames that have elapsed since the app began
    pub frame: FrameCount,
    /// The amount of time that has elapsed since the app began
    pub time: Duration,
    /// The [`InputEvent`] that was captured
    pub event: InputEvent,
}

/// The number of frames that have elapsed since the app started
///
/// Updated in [`time_tracker`] during [`CoreStage::First`].
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameCount(pub u64);

/// Collects input-relevant events for use in [`UnifiedInput`]
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// The [`InputButton`] was pressed
    Pressed(InputButton),
    /// The [`InputButton`] was released
    Released(InputButton),
    /// A two-dimensional axis-like input was changed
    AxisPairChanged {
        /// Which axis changed
        axis: AxisPairType,
        /// The new horizontal value of the axis
        x: f32,
        /// The new vertical value of the axis
        y: f32,
    },
    /// A gamepad axis changed
    GamepadAxisChanged {
        /// Which gamepad axis changed
        axis: GamepadAxisType,
        /// The new value of the axis
        value: f32,
    },
    /// A gamepad was connected to the computer
    GamepadConnected(Gamepad),
    /// A gamepad was disconnected from the computer
    GamepadDisconnected(Gamepad),
}

/// Captures user inputs from the assorted raw `Event` types
///
/// These are collected into a [`UnifiedInput`] event stream.
pub struct InputCapturePlugin;

impl Plugin for InputCapturePlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::First, frame_counter)
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .with_system(capture_mouse_input)
                    .with_system(capture_keyboard_input)
                    .with_system(capture_gamepad_input),
            );
    }
}

/// Increases the value of the [`FrameCount`] resource by 1 every frame
///
/// This system should run during [`CoreStage::First`].
pub fn frame_counter(mut frame_count: ResMut<FrameCount>) {
    frame_count.0 += 1;
}

/// Captures mouse-driven input from the [`MouseButtonInput`] event stream
///
/// Limitations:
///  - the unit of mouse scrolling is discarded; when played back this is assumed to be pixels
///  - mouse inputs performed with a locked window will be lost, as [`MouseMotion`](bevy::input::mouse::MouseMotion) events are not captured
///  - this is not robust to multiple windows; the window that the mouse is on is lost
pub fn capture_mouse_input(
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut unified_input: EventWriter<UnifiedInput>,
    frame_count: Res<FrameCount>,
    time: Res<Time>,
) {
    for mouse_button_event in mouse_button_events.iter() {
        let input_event = match mouse_button_event.state {
            ElementState::Pressed => InputEvent::Pressed(mouse_button_event.button.into()),
            ElementState::Released => InputEvent::Released(mouse_button_event.button.into()),
        };

        unified_input.send(UnifiedInput {
            frame: *frame_count,
            time: time.time_since_startup(),
            event: input_event,
        })
    }

    for mouse_wheel_event in mouse_wheel_events.iter() {
        let input_event = InputEvent::AxisPairChanged {
            axis: AxisPairType::ScrollWheel,
            x: mouse_wheel_event.x,
            y: mouse_wheel_event.y,
        };

        unified_input.send(UnifiedInput {
            frame: *frame_count,
            time: time.time_since_startup(),
            event: input_event,
        })
    }

    for cursor_moved_event in cursor_moved_events.iter() {
        let input_event = InputEvent::AxisPairChanged {
            axis: AxisPairType::Mouse,
            x: cursor_moved_event.position.x,
            y: cursor_moved_event.position.y,
        };

        unified_input.send(UnifiedInput {
            frame: *frame_count,
            time: time.time_since_startup(),
            event: input_event,
        })
    }
}

/// Captures [`KeyCode`](bevy_input::keyboard::KeyCode) input from the [`MouseButtonInput`] stream
pub fn capture_keyboard_input(
    mut raw_events: EventReader<KeyboardInput>,
    mut unified_input: EventWriter<UnifiedInput>,
    frame_count: Res<FrameCount>,
    time: Res<Time>,
) {
    for raw_event in raw_events.iter() {
        // Only keyboard events that are coercible to `KeyCode` are currently usable as Input in Bevy
        // so all other keyboard events are ignored
        if let Some(key_code) = raw_event.key_code {
            let input_event = match raw_event.state {
                ElementState::Pressed => InputEvent::Pressed(key_code.into()),
                ElementState::Released => InputEvent::Released(key_code.into()),
            };

            unified_input.send(UnifiedInput {
                frame: *frame_count,
                time: time.time_since_startup(),
                event: input_event,
            });
        }
    }
}

/// Captures [`InputEvent`]s from the [`GamepadEvent`] stream
pub fn capture_gamepad_input(
    mut raw_events: EventReader<GamepadEvent>,
    mut unified_input: EventWriter<UnifiedInput>,
    frame_count: Res<FrameCount>,
    time: Res<Time>,
) {
    use GamepadEventType::*;

    for raw_event in raw_events.iter() {
        let gamepad = raw_event.0;

        let input_event = match raw_event.1 {
            Connected => InputEvent::GamepadConnected(gamepad),
            Disconnected => InputEvent::GamepadDisconnected(gamepad),
            ButtonChanged(button, value) => {
                if value == 1.0 {
                    InputEvent::Pressed(button.into())
                } else if value == 0.0 {
                    InputEvent::Released(button.into())
                } else {
                    todo!()
                }
            }
            AxisChanged(axis, value) => InputEvent::GamepadAxisChanged { axis, value },
        };

        unified_input.send(UnifiedInput {
            frame: *frame_count,
            time: time.time_since_startup(),
            event: input_event,
        })
    }
}

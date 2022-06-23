//! Captures user input from assorted raw `Event` types.
//!
//! These are unified into a single [`UnifiedInput`] event stream, which can be played back
use crate::user_input::InputButton;
use bevy_app::{App, CoreStage, Plugin};
use bevy_core::Time;
use bevy_ecs::prelude::*;
use bevy_input::gamepad::{Gamepad, GamepadEvent, GamepadEventType};
use bevy_input::keyboard::KeyboardInput;
use bevy_input::mouse::MouseButtonInput;
use bevy_input::ElementState;
/// via the provided [`input_mocking`](crate::input_mocking) functionality
use bevy_utils::Duration;

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
                    .with_system(capture_mouse_button_input)
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

/// Captures [`MouseButton`] input from the [`MouseButtonInput`] event stream
pub fn capture_mouse_button_input(
    mut raw_events: EventReader<MouseButtonInput>,
    mut unified_input: EventWriter<UnifiedInput>,
    frame_count: Res<FrameCount>,
    time: Res<Time>,
) {
    for raw_event in raw_events.iter() {
        let input_event = match raw_event.state {
            ElementState::Pressed => InputEvent::Pressed(raw_event.button.into()),
            ElementState::Released => InputEvent::Released(raw_event.button.into()),
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
            ButtonChanged(_button, _value) => todo!(),
            AxisChanged(_axis, _value) => todo!(),
        };

        unified_input.send(UnifiedInput {
            frame: *frame_count,
            time: time.time_since_startup(),
            event: input_event,
        })
    }
}

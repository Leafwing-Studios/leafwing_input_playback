//! Captures user input from assorted raw `Event` types.
//!
//! These are unified into a single [`UnifiedInput`] event stream, which can be played back
use bevy_app::{App, CoreStage, Plugin};
use bevy_core::Time;
use bevy_ecs::prelude::*;
use bevy_input::keyboard::KeyboardInput;
use bevy_input::mouse::{MouseButtonInput, MouseWheel};
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
    pub time_since_startup: Duration,
    /// The [`InputEvent`] that was captured
    pub input_event: InputEvent,
}

/// The number of frames that have elapsed since the app started
///
/// Updated in [`time_tracker`] during [`CoreStage::First`].
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameCount(pub u64);

/// Collects input-relevant events for use in [`UnifiedInput`]
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub enum InputEvent {
    Keyboard(KeyboardInput),
    MouseButton(MouseButtonInput),
    MouseWheel(MouseWheel),
    CursorMoved(CursorMoved),
}

impl From<KeyboardInput> for InputEvent {
    fn from(event: KeyboardInput) -> Self {
        InputEvent::Keyboard(event)
    }
}

impl From<MouseButtonInput> for InputEvent {
    fn from(event: MouseButtonInput) -> Self {
        InputEvent::MouseButton(event)
    }
}

impl From<MouseWheel> for InputEvent {
    fn from(event: MouseWheel) -> Self {
        InputEvent::MouseWheel(event)
    }
}

impl From<CursorMoved> for InputEvent {
    fn from(event: CursorMoved) -> Self {
        InputEvent::CursorMoved(event)
    }
}

/// Captures user inputs from the assorted raw `Event` types
///
/// These are collected into a [`UnifiedInput`] event stream.
pub struct InputCapturePlugin;

impl Plugin for InputCapturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameCount>()
            .add_event::<UnifiedInput>()
            .add_system_to_stage(CoreStage::First, frame_counter)
            .add_system_set_to_stage(
                // Capture any mocked input as well
                CoreStage::Last,
                SystemSet::new()
                    .with_system(capture_mouse_input)
                    .with_system(capture_keyboard_input),
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
    let time_since_startup = time.time_since_startup();
    let frame = *frame_count;

    // BLOCKED: these events are arbitrarily ordered within a frame,
    // but we have no way to access their order from winit.
    // See https://github.com/bevyengine/bevy/issues/5984
    for input_event in mouse_button_events.iter().cloned() {
        unified_input.send(UnifiedInput {
            frame,
            time_since_startup,
            input_event: input_event.into(),
        })
    }

    for input_event in mouse_wheel_events.iter().cloned() {
        unified_input.send(UnifiedInput {
            frame,
            time_since_startup,
            input_event: input_event.into(),
        })
    }

    for input_event in cursor_moved_events.iter().cloned() {
        unified_input.send(UnifiedInput {
            frame,
            time_since_startup,
            input_event: input_event.into(),
        })
    }
}

/// Captures [`KeyCode`](bevy_input::keyboard::KeyCode) input from the [`MouseButtonInput`] stream
pub fn capture_keyboard_input(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut unified_input: EventWriter<UnifiedInput>,
    frame_count: Res<FrameCount>,
    time: Res<Time>,
) {
    let time_since_startup = time.time_since_startup();
    let frame = *frame_count;

    for input_event in keyboard_events.iter().cloned() {
        unified_input.send(UnifiedInput {
            frame,
            time_since_startup,
            input_event: input_event.into(),
        })
    }
}

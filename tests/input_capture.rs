use bevy::core::FrameCount;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::ButtonState;
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::window::WindowPlugin;

use leafwing_input_playback::input_capture::EndInputCapture;
use leafwing_input_playback::input_capture::{
    BeginInputCapture, InputCapturePlugin, InputModesCaptured,
};
use leafwing_input_playback::timestamped_input::{
    InputEvent, TimestampedInputEvent, TimestampedInputs,
};

const TEST_PRESS: KeyboardInput = KeyboardInput {
    logical_key: Key::Character(smol_str::SmolStr::new_static("F")),
    key_code: KeyCode::KeyF,
    state: ButtonState::Pressed,
    window: Entity::PLACEHOLDER,
    repeat: false,
};

const TEST_RELEASE: KeyboardInput = KeyboardInput {
    logical_key: Key::Character(smol_str::SmolStr::new_static("F")),
    key_code: KeyCode::KeyF,
    state: ButtonState::Released,
    window: Entity::PLACEHOLDER,
    repeat: false,
};

const TEST_MOUSE: MouseButtonInput = MouseButtonInput {
    button: MouseButton::Left,
    state: ButtonState::Pressed,
    window: Entity::PLACEHOLDER,
};

fn capture_app() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        WindowPlugin::default(),
        InputPlugin,
        InputCapturePlugin,
    ));
    app
}

#[test]
fn capture_sent_events() {
    let mut app = capture_app();
    app.world_mut().trigger(BeginInputCapture::default());

    let mut keyboard_events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);
    keyboard_events.send(TEST_RELEASE);

    app.update();
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 2);
}

#[test]
fn identity_of_sent_events() {
    let mut app = capture_app();
    app.world_mut().trigger(BeginInputCapture::default());

    let mut keyboard_events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);

    // Events within the same frame are not ordered reliably
    app.update();

    let mut mouse_events = app.world_mut().resource_mut::<Events<MouseButtonInput>>();
    mouse_events.send(TEST_MOUSE);

    app.update();
    let mut timestamped_input = app.world_mut().resource_mut::<TimestampedInputs>();
    let mut iterator = timestamped_input.iter_all().into_iter();

    let first_event: TimestampedInputEvent = iterator.next().unwrap();
    let second_event: TimestampedInputEvent = iterator.next().unwrap();

    // Unfortunately these input types don't impl PartialEq :(
    assert!(matches!(first_event.input_event, InputEvent::Keyboard(_)));
    assert!(matches!(
        second_event.input_event,
        InputEvent::MouseButton(_)
    ));
}

#[test]
fn framecount_of_sent_events() {
    let mut app = capture_app();
    app.world_mut().trigger(BeginInputCapture::default());

    let mut keyboard_events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);

    app.update();

    let mut mouse_events = app.world_mut().resource_mut::<Events<MouseButtonInput>>();
    mouse_events.send(TEST_MOUSE);

    app.update();
    let mut timestamped_input = app.world_mut().resource_mut::<TimestampedInputs>();
    let mut iterator = timestamped_input.iter_all().into_iter();

    let first_event: TimestampedInputEvent = iterator.next().expect("Keyboard event failed.");
    let second_event: TimestampedInputEvent = iterator.next().expect("Mouse event failed.");

    assert_eq!(first_event.frame, FrameCount(0));
    assert_eq!(second_event.frame, FrameCount(1));
}

#[test]
fn toggle_input_capture() {
    let mut app = capture_app();
    app.world_mut().trigger(BeginInputCapture::default());

    let mut keyboard_events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);
    keyboard_events.send(TEST_RELEASE);

    app.update();

    // Inputs are captured while input capturing is enabled by default
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 2);

    // Disabling input capture
    let mut input_modes_captured = app.world_mut().resource_mut::<InputModesCaptured>();
    *input_modes_captured = InputModesCaptured::DISABLE_ALL;

    let mut keyboard_events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);
    keyboard_events.send(TEST_RELEASE);

    app.update();

    // Inputs are not captured while input capturing is disabled
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 2);

    // Partially re-enabling input capture
    let mut input_modes_captured = app.world_mut().resource_mut::<InputModesCaptured>();
    *input_modes_captured = InputModesCaptured {
        mouse_buttons: false,
        keyboard: true,
        ..Default::default()
    };

    let mut keyboard_events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);

    let mut mouse_events = app.world_mut().resource_mut::<Events<MouseButtonInput>>();
    mouse_events.send(TEST_MOUSE);

    app.update();

    // Only the keyboard events (and app exit events) were captured
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 3);
}

#[test]
fn end_input_capture() {
    let mut app = capture_app();
    app.world_mut().trigger(BeginInputCapture::default());

    let mut keyboard_events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);

    app.update();

    // Inputs are captured while input capturing is enabled by default
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 1);

    // End input capture
    app.world_mut().trigger(EndInputCapture);

    let mut keyboard_events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_RELEASE);

    app.update();

    // Inputs are not captured once capture is ended.
    // Since we have not provided a file path, TimestampedInputs remains.
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 1);

    // Beginning capture again also works.
    app.world_mut().trigger(BeginInputCapture::default());

    app.update();

    // The previous results have not been overwritten.
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 1);

    let mut keyboard_events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_RELEASE);

    app.update();

    // New inputs are still accepted.
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 2);
}

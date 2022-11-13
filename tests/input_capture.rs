use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::ButtonState;
use bevy::input::InputPlugin;
use bevy::prelude::*;

use bevy::window::WindowPlugin;
use leafwing_input_playback::frame_counting::FrameCount;
use leafwing_input_playback::input_capture::{InputCapturePlugin, InputModesCaptured};
use leafwing_input_playback::timestamped_input::{
    InputEvent, TimestampedInputEvent, TimestampedInputs,
};

const TEST_PRESS: KeyboardInput = KeyboardInput {
    scan_code: 1,
    key_code: Some(KeyCode::F),
    state: ButtonState::Pressed,
};

const TEST_RELEASE: KeyboardInput = KeyboardInput {
    scan_code: 1,
    key_code: Some(KeyCode::F),
    state: ButtonState::Released,
};

const TEST_MOUSE: MouseButtonInput = MouseButtonInput {
    button: MouseButton::Left,
    state: ButtonState::Pressed,
};

fn capture_app() -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugin(WindowPlugin::default())
        .add_plugin(InputPlugin)
        .add_plugin(InputCapturePlugin);

    app
}

#[test]
fn app_update_sends_app_exit() {
    let mut app = capture_app();

    app.update();
    let timestamped_input = app.world.resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 1);
    let timestamped_event = timestamped_input.events.first().unwrap();
    assert_eq!(timestamped_event.input_event, InputEvent::AppExit);
}

#[test]
fn capture_sent_events() {
    let mut app = capture_app();

    let mut keyboard_events = app.world.resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);
    keyboard_events.send(TEST_RELEASE);

    app.update();
    let timestamped_input = app.world.resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 3);
}

#[test]
fn identity_of_sent_events() {
    let mut app = capture_app();

    let mut keyboard_events = app.world.resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);

    // Events within the same frame are not ordered reliably
    app.update();

    let mut mouse_events = app.world.resource_mut::<Events<MouseButtonInput>>();
    mouse_events.send(TEST_MOUSE);

    app.update();
    let mut timestamped_input = app.world.resource_mut::<TimestampedInputs>();
    let mut iterator = timestamped_input.iter_all().into_iter();

    let first_event: TimestampedInputEvent = iterator.next().unwrap();
    let second_event: TimestampedInputEvent = iterator.next().unwrap();
    let third_event: TimestampedInputEvent = iterator.next().unwrap();

    // Unfortunately these input types don't impl PartialEq :(
    assert!(matches!(first_event.input_event, InputEvent::Keyboard(_)));
    assert!(matches!(second_event.input_event, InputEvent::AppExit));
    assert!(matches!(
        third_event.input_event,
        InputEvent::MouseButton(_)
    ));
}

#[test]
fn framecount_of_sent_events() {
    let mut app = capture_app();

    let mut keyboard_events = app.world.resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);

    app.update();

    let mut mouse_events = app.world.resource_mut::<Events<MouseButtonInput>>();
    mouse_events.send(TEST_MOUSE);

    app.update();
    let mut timestamped_input = app.world.resource_mut::<TimestampedInputs>();
    let mut iterator = timestamped_input.iter_all().into_iter();

    let first_event: TimestampedInputEvent = iterator.next().unwrap();
    let second_event: TimestampedInputEvent = iterator.next().unwrap();
    let third_event: TimestampedInputEvent = iterator.next().unwrap();

    // The frame count is recorded based on the frame it is read,
    // which counts up immediately
    assert_eq!(first_event.frame, FrameCount(1));
    // The app exit event
    assert_eq!(second_event.frame, FrameCount(1));
    assert_eq!(third_event.frame, FrameCount(2));
}

#[test]
fn toggle_input_capture() {
    let mut app = capture_app();

    let mut keyboard_events = app.world.resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);
    keyboard_events.send(TEST_RELEASE);

    app.update();

    // Inputs are captured while input capturing is enabled by default
    let timestamped_input = app.world.resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 3);

    // Disabling input capture
    let mut input_modes_captured = app.world.resource_mut::<InputModesCaptured>();
    *input_modes_captured = InputModesCaptured::DISABLE_ALL;

    let mut keyboard_events = app.world.resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);
    keyboard_events.send(TEST_RELEASE);

    app.update();

    // Inputs are not captured while input capturing is disabled
    // Note that each app.update() always sends an `AppExit` event
    let timestamped_input = app.world.resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 4);

    // Partially re-enabling input capture
    let mut input_modes_captured = app.world.resource_mut::<InputModesCaptured>();
    *input_modes_captured = InputModesCaptured {
        mouse_buttons: false,
        keyboard: true,
        ..Default::default()
    };

    let mut keyboard_events = app.world.resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);

    let mut mouse_events = app.world.resource_mut::<Events<MouseButtonInput>>();
    mouse_events.send(TEST_MOUSE);

    app.update();

    // Only the keyboard events (and app exit events) were captured
    let timestamped_input = app.world.resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.len(), 6);
}

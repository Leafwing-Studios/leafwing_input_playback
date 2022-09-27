use bevy::input::keyboard::KeyboardInput;
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy_input::mouse::MouseButtonInput;
use bevy_input::ButtonState;

use bevy::window::WindowPlugin;
use leafwing_input_playback::frame_counting::FrameCount;
use leafwing_input_playback::input_capture::InputCapturePlugin;
use leafwing_input_playback::unified_input::{InputEvent, TimestampedInputEvent, UnifiedInput};

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

#[test]
fn capture_sent_events() {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugin(WindowPlugin)
        .add_plugin(InputPlugin)
        .add_plugin(InputCapturePlugin);

    let mut keyboard_events = app.world.resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);
    keyboard_events.send(TEST_RELEASE);

    app.update();
    let unified_input = app.world.resource::<UnifiedInput>();
    assert_eq!(unified_input.len(), 2);
}

#[test]
fn identity_of_sent_events() {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugin(WindowPlugin)
        .add_plugin(InputPlugin)
        .add_plugin(InputCapturePlugin);

    let mut keyboard_events = app.world.resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);

    // Events within the same frame are not ordered reliably
    app.update();

    let mut mouse_events = app.world.resource_mut::<Events<MouseButtonInput>>();
    mouse_events.send(TEST_MOUSE);

    app.update();
    let mut unified_input = app.world.resource_mut::<UnifiedInput>();
    let mut iterator = unified_input.iter_all().into_iter();

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
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugin(WindowPlugin)
        .add_plugin(InputPlugin)
        .add_plugin(InputCapturePlugin);

    let mut keyboard_events = app.world.resource_mut::<Events<KeyboardInput>>();
    keyboard_events.send(TEST_PRESS);

    // Events within the same frame are not ordered reliably
    app.update();

    let mut mouse_events = app.world.resource_mut::<Events<MouseButtonInput>>();
    mouse_events.send(TEST_MOUSE);

    app.update();
    let mut unified_input = app.world.resource_mut::<UnifiedInput>();
    let mut iterator = unified_input.iter_all().into_iter();

    let first_event: TimestampedInputEvent = iterator.next().unwrap();
    let second_event: TimestampedInputEvent = iterator.next().unwrap();

    // The frame count is recorded based on the frame it is read,
    // which counts up immediately
    assert_eq!(first_event.frame, FrameCount(1));
    assert_eq!(second_event.frame, FrameCount(2));
}

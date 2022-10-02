use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::utils::Duration;

use bevy::window::WindowPlugin;
use leafwing_input_playback::frame_counting::FrameCount;

use leafwing_input_playback::input_capture::InputCapturePlugin;
use leafwing_input_playback::input_capture::InputModesCaptured;
use leafwing_input_playback::input_playback::InputPlaybackPlugin;
use leafwing_input_playback::input_playback::PlaybackStrategy;
use leafwing_input_playback::unified_input::UnifiedInput;

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

fn playback_app(strategy: PlaybackStrategy) -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugin(WindowPlugin)
        .add_plugin(InputPlugin)
        .add_plugin(InputPlaybackPlugin);

    *app.world.resource_mut::<PlaybackStrategy>() = strategy;

    app
}

fn simple_unified_input() -> UnifiedInput {
    let mut inputs = UnifiedInput::default();
    inputs.send(FrameCount(1), Duration::from_secs(0), TEST_PRESS.into());
    inputs.send(FrameCount(2), Duration::from_secs(0), TEST_RELEASE.into());

    inputs
}

fn complex_unified_input() -> UnifiedInput {
    let mut inputs = UnifiedInput::default();
    inputs.send(FrameCount(0), Duration::from_secs(0), TEST_PRESS.into());
    inputs.send(FrameCount(1), Duration::from_secs(1), TEST_RELEASE.into());
    inputs.send(FrameCount(2), Duration::from_secs(2), TEST_PRESS.into());
    inputs.send(FrameCount(2), Duration::from_secs(3), TEST_PRESS.into());
    inputs.send(FrameCount(3), Duration::from_secs(3), TEST_PRESS.into());

    inputs
}

#[test]
fn minimal_playback() {
    let mut app = playback_app(PlaybackStrategy::FrameCount);
    let input_events = app.world.resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 0);

    *app.world.resource_mut::<UnifiedInput>() = simple_unified_input();
    app.update();

    // By default, only events up to the current frame are played back
    let input_events = app.world.resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 1);
    let input = app.world.resource::<Input<KeyCode>>();
    assert!(input.pressed(KeyCode::F));

    app.update();
    let input_events = app.world.resource::<Events<KeyboardInput>>();
    // Events are double-buffered
    assert_eq!(input_events.len(), 2);
    let input = app.world.resource::<Input<KeyCode>>();
    assert!(!input.pressed(KeyCode::F));
}

#[test]
fn capture_and_playback() {
    let mut app = playback_app(PlaybackStrategy::default());
    app.add_plugin(InputCapturePlugin);
    app.insert_resource(PlaybackStrategy::Paused);

    let mut input_events = app.world.resource_mut::<Events<KeyboardInput>>();
    input_events.send(TEST_PRESS);

    app.update();

    let input = app.world.resource::<Input<KeyCode>>();
    // Input is pressed because we just sent a real event
    assert!(input.pressed(TEST_PRESS.key_code.unwrap()));

    app.update();
    let input = app.world.resource::<Input<KeyCode>>();
    // Input is not pressed, as playback is not enabled and the previous event expired
    assert!(input.pressed(TEST_PRESS.key_code.unwrap()));

    app.insert_resource(InputModesCaptured::DISABLE_ALL);
    app.insert_resource(PlaybackStrategy::FrameCount);

    app.update();

    let input = app.world.resource::<Input<KeyCode>>();
    // Input is now pressed, as the pressed key has been played back.
    assert!(input.pressed(TEST_PRESS.key_code.unwrap()));
}

#[test]
fn playback_strategy_paused() {
    let mut app = playback_app(PlaybackStrategy::Paused);
    *app.world.resource_mut::<UnifiedInput>() = complex_unified_input();

    let unified_input = app.world.resource::<UnifiedInput>();
    assert_eq!(unified_input.cursor, 0);

    for _ in 0..10 {
        app.update();
    }

    let unified_input = app.world.resource::<UnifiedInput>();
    assert_eq!(unified_input.cursor, 0);
}

#[test]
fn playback_strategy_frame() {}

#[test]
fn playback_strategy_frame_slice() {}

#[test]
fn playback_strategy_time() {}

#[test]
fn playback_strategy_time_slice() {}
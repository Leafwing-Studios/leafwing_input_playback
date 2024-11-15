// BLOCKED: add time strategy tests: https://github.com/bevyengine/bevy/issues/6146

use bevy::core::FrameCount;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::utils::Duration;
use bevy::window::WindowPlugin;

use leafwing_input_playback::input_capture::InputCapturePlugin;
use leafwing_input_playback::input_capture::InputModesCaptured;
use leafwing_input_playback::input_playback::BeginInputPlayback;
use leafwing_input_playback::input_playback::InputPlaybackPlugin;
use leafwing_input_playback::input_playback::InputPlaybackSource;
use leafwing_input_playback::input_playback::PlaybackStrategy;
use leafwing_input_playback::timestamped_input::TimestampedInputs;

const TEST_PRESS: KeyboardInput = KeyboardInput {
    logical_key: Key::Character(smol_str::SmolStr::new_static("F")),
    key_code: KeyCode::KeyF,
    state: ButtonState::Pressed,
    window: Entity::PLACEHOLDER,
};

const TEST_RELEASE: KeyboardInput = KeyboardInput {
    logical_key: Key::Character(smol_str::SmolStr::new_static("F")),
    key_code: KeyCode::KeyF,
    state: ButtonState::Released,
    window: Entity::PLACEHOLDER,
};

fn playback_app() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        WindowPlugin::default(),
        InputPlugin,
        InputPlaybackPlugin,
    ));

    app
}

fn simple_timestamped_input() -> TimestampedInputs {
    let mut inputs = TimestampedInputs::default();
    inputs.send(FrameCount(0), Duration::from_secs(0), TEST_PRESS.into());
    inputs.send(FrameCount(1), Duration::from_secs(0), TEST_RELEASE.into());

    inputs
}

fn complex_timestamped_input() -> TimestampedInputs {
    let mut inputs = TimestampedInputs::default();
    inputs.send(FrameCount(0), Duration::from_secs(0), TEST_PRESS.into());
    inputs.send(FrameCount(1), Duration::from_secs(1), TEST_RELEASE.into());
    inputs.send(FrameCount(2), Duration::from_secs(2), TEST_PRESS.into());
    inputs.send(FrameCount(2), Duration::from_secs(3), TEST_PRESS.into());
    inputs.send(FrameCount(3), Duration::from_secs(3), TEST_PRESS.into());

    inputs
}

#[test]
fn minimal_playback() {
    let mut app = playback_app();

    app.world_mut().trigger(BeginInputPlayback {
        playback_strategy: PlaybackStrategy::FrameCount,
        source: Some(InputPlaybackSource::from_inputs(simple_timestamped_input())),
        ..Default::default()
    });
    app.world_mut().flush_commands();

    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 0);

    app.update();

    // By default, only events up to the current frame are played back
    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 1);
    let input = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(input.pressed(KeyCode::KeyF));

    app.update();
    let input_events = app.world().resource::<Events<KeyboardInput>>();
    // Events are double-buffered
    assert_eq!(input_events.len(), 2);
    let input = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(!input.pressed(KeyCode::KeyF));
}

#[test]
fn capture_and_playback() {
    let mut app = playback_app();
    app.add_plugins(InputCapturePlugin);

    app.world_mut().trigger(BeginInputPlayback {
        playback_strategy: PlaybackStrategy::Paused,
        source: Some(InputPlaybackSource::from_inputs(Default::default())),
        ..Default::default()
    });
    app.world_mut().flush_commands();

    let mut input_events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
    input_events.send(TEST_PRESS);

    app.update();

    let input = app.world().resource::<ButtonInput<KeyCode>>();
    // Input is pressed because we just sent a real event
    assert!(input.pressed(TEST_PRESS.key_code));

    app.update();
    let input = app.world().resource::<ButtonInput<KeyCode>>();
    // Input is not pressed, as playback is not enabled and the previous event expired
    assert!(input.pressed(TEST_PRESS.key_code));

    app.insert_resource(InputModesCaptured::DISABLE_ALL);
    // This should trigger playback of input captured so far.
    app.insert_resource(PlaybackStrategy::FrameCount);

    app.update();

    let input = app.world().resource::<ButtonInput<KeyCode>>();
    // Input is now pressed, as the pressed key has been played back.
    assert!(input.pressed(TEST_PRESS.key_code));
}

#[test]
fn repeated_playback() {
    // Play all of the events each pass
    let mut app = playback_app();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));

    app.world_mut().trigger(BeginInputPlayback {
        playback_strategy: PlaybackStrategy::default(),
        source: Some(InputPlaybackSource::from_inputs(simple_timestamped_input())),
        ..Default::default()
    });
    app.world_mut().flush_commands();

    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 0);

    for _ in 1..10 {
        app.update();
    }

    // Verify that we're out of events
    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 0);

    // Reset our tracking
    let mut timestamped_input: Mut<TimestampedInputs> = app.world_mut().resource_mut();
    timestamped_input.reset_cursor();

    // Play the events again
    app.update();

    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 2);
}

#[test]
fn playback_strategy_paused() {
    let mut app = playback_app();

    app.world_mut().trigger(BeginInputPlayback {
        playback_strategy: PlaybackStrategy::Paused,
        source: Some(InputPlaybackSource::from_inputs(complex_timestamped_input())),
        ..Default::default()
    });
    app.world_mut().flush_commands();

    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.cursor, 0);

    for _ in 0..10 {
        app.update();
    }

    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.cursor, 0);
}

#[test]
fn playback_strategy_frame() {
    let mut app = playback_app();

    app.world_mut().trigger(BeginInputPlayback {
        playback_strategy: PlaybackStrategy::FrameCount,
        source: Some(InputPlaybackSource::from_inputs(complex_timestamped_input())),
        ..Default::default()
    });
    app.world_mut().flush_commands();

    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.cursor, 0);

    // Check complex_timestamped_input to verify the pattern
    app.update();
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.cursor, 1);

    app.update();
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.cursor, 2);

    app.update();
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.cursor, 4);

    app.update();
    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.cursor, 5);
}

#[test]
fn playback_strategy_frame_range_once() {
    let mut app = playback_app();

    let strategy = PlaybackStrategy::FrameRangeOnce(FrameCount(2), FrameCount(5));
    app.world_mut().trigger(BeginInputPlayback {
        playback_strategy: strategy,
        source: Some(InputPlaybackSource::from_inputs(complex_timestamped_input())),
        ..Default::default()
    });
    app.world_mut().flush_commands();

    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.cursor, 0);

    // Replays the events in the frame range [2, 5)
    // This playback strategy plays back the inputs one frame at a time until the entire range is captured
    // Then swaps to PlaybackStrategy::Paused
    // Frame 2
    app.update();

    let input_events = app.world().resource::<Events<KeyboardInput>>();
    eprintln!("{input_events:?}");
    assert_eq!(input_events.len(), 2);

    // Frame 3 (events are double buffered)
    app.update();
    let input_events = app.world().resource::<Events<KeyboardInput>>();
    eprintln!("{input_events:?}");
    assert_eq!(input_events.len(), 3);

    // Frame 4 (events are double buffered)
    app.update();
    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(*app.world().resource::<PlaybackStrategy>(), strategy);
    eprintln!("{input_events:?}");
    assert_eq!(input_events.len(), 1);

    // Paused
    app.update();
    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 0);
    assert_eq!(
        *app.world().resource::<PlaybackStrategy>(),
        PlaybackStrategy::Paused
    );
}

#[test]
fn playback_strategy_frame_range_loop() {
    let mut app = playback_app();

    let strategy = PlaybackStrategy::FrameRangeLoop(FrameCount(2), FrameCount(5));
    app.world_mut().trigger(BeginInputPlayback {
        playback_strategy: strategy,
        source: Some(InputPlaybackSource::from_inputs(complex_timestamped_input())),
        ..Default::default()
    });

    app.update();

    let timestamped_input = app.world().resource::<TimestampedInputs>();
    assert_eq!(timestamped_input.cursor, 0);

    // Replays the events in the frame range [2, 5)
    // This playback strategy plays back the inputs one frame at a time until the entire range is captured
    // Then swaps to PlaybackStrategy::Paused
    // Frame 2
    app.update();
    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 2);

    // Frame 3 (events are double buffered)
    app.update();
    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 3);

    // Frame 4 (events are double buffered)
    app.update();
    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(*app.world().resource::<PlaybackStrategy>(), strategy);
    assert_eq!(input_events.len(), 1);

    // Spacing frame
    app.update();

    // Looping back to frame 2
    app.update();
    let input_events = app.world().resource::<Events<KeyboardInput>>();
    assert_eq!(input_events.len(), 2);
    assert_eq!(
        *app.world().resource::<PlaybackStrategy>(),
        PlaybackStrategy::FrameRangeLoop(FrameCount(2), FrameCount(5))
    );
}

/// Demonstrates reading saved inputs from disk, and playing them back.
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use leafwing_input_playback::input_playback::{
    BeginInputPlayback, InputPlaybackPlugin, InputPlaybackSource,
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, InputPlaybackPlugin));
    app.add_systems(Update, debug_keyboard_inputs);

    app.world_mut().trigger(BeginInputPlayback {
        source: Some(InputPlaybackSource::from_file("./data/hello_world.ron")),
        ..Default::default()
    });
    app.run();
}

fn debug_keyboard_inputs(mut keyboard_events: EventReader<KeyboardInput>) {
    for keyboard_event in keyboard_events.read() {
        dbg!(keyboard_event);
    }
}

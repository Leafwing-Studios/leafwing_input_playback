/// Demonstrates reading saved inputs from disk, and playing them back.
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use leafwing_input_playback::input_playback::InputPlaybackPlugin;
use leafwing_input_playback::serde::PlaybackFilePath;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InputPlaybackPlugin))
        .insert_resource(PlaybackFilePath::new("./data/hello_world.ron"))
        .add_systems(Update, debug_keyboard_inputs)
        .run();
}

fn debug_keyboard_inputs(mut keyboard_events: EventReader<KeyboardInput>) {
    for keyboard_event in keyboard_events.read() {
        dbg!(keyboard_event);
    }
}

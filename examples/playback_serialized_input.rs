/// Demonstrates reading saved capture inputs from disk, and playing them back.
use bevy::prelude::*;
use leafwing_input_playback::input_playback::InputPlaybackPlugin;
use leafwing_input_playback::serde::PlaybackFilePath;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(InputPlaybackPlugin)
        .insert_resource(PlaybackFilePath::new("./test_playback.ron"));
}

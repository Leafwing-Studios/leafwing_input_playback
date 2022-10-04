//! [`AppExit`] events are played back and captured too!
//!
//! This example loads the file, which only contains an `AppExit`,
//! and then immediately quits itself as soon as it is encountered.
use bevy::prelude::*;
use leafwing_input_playback::input_playback::InputPlaybackPlugin;
use leafwing_input_playback::serde::PlaybackFilePath;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(InputPlaybackPlugin)
        .insert_resource(PlaybackFilePath::new("./data/app_exit.ron"))
        .run();
}

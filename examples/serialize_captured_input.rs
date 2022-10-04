/// Demonstrates saving captured inputs to disk, to later be played back.
///
/// Just enter inputs, and watch them be serialized to disk.
use bevy::prelude::*;
use leafwing_input_playback::input_capture::InputCapturePlugin;
use leafwing_input_playback::serde::PlaybackFilePath;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(InputCapturePlugin)
        .insert_resource(PlaybackFilePath::new("./test_playback.ron"));
}

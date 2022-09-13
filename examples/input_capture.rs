use bevy::prelude::*;
use leafwing_input_playback::{capture::InputCapturePlugin, playback_data::PlaybackData};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // This MUST be added after DefaultPlugins to override the runner correctly
        .add_plugin(InputCapturePlugin::default())
        .add_system(debug_input_capture)
        .run()
}

fn debug_input_capture(mut captured_input: EventReader<PlaybackData>) {
    for input_event in captured_input.iter() {
        dbg!(input_event);
    }
}

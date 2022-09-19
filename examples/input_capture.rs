use bevy::prelude::*;
use leafwing_input_playback::{
    input_capture::InputCapturePlugin, unified_input::UnifiedInputEvent,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(InputCapturePlugin)
        .add_system(debug_input_capture)
        .run()
}

fn debug_input_capture(mut captured_input: EventReader<UnifiedInputEvent>) {
    for input_event in captured_input.iter() {
        dbg!(input_event);
    }
}

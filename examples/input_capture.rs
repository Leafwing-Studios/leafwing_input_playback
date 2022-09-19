use bevy::prelude::*;
use leafwing_input_playback::{input_capture::InputCapturePlugin, unified_input::UnifiedInput};
use std::ops::DerefMut;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(InputCapturePlugin)
        .add_system(debug_input_capture)
        .run()
}

// UnifiedInput is an iterator, so we require mutable access to track which events we've seen
fn debug_input_capture(mut captured_input: ResMut<UnifiedInput>) {
    // Dereferences out of the `ResMut` smart pointer, allowing us to access the `Iterator` trait
    for input_event in captured_input.deref_mut() {
        dbg!(input_event);
    }
}

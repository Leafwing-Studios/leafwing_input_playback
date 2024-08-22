use bevy::prelude::*;
use leafwing_input_playback::{
    input_capture::InputCapturePlugin, timestamped_input::TimestampedInputs,
};

fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, InputCapturePlugin))
        .add_systems(Update, debug_input_capture)
        .run()
}

// TimestampedInput is an iterator, so we require mutable access to track which events we've seen
fn debug_input_capture(mut captured_input: ResMut<TimestampedInputs>) {
    // Dereferences out of the `ResMut` smart pointer, allowing us to access the `Iterator` trait
    for input_event in captured_input.iter_rest() {
        dbg!(input_event);
    }
}

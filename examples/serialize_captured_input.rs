/// Demonstrates saving captured inputs to disk, to later be played back.
///
/// Just enter inputs, and watch them be serialized to disk.
use bevy::prelude::*;
use leafwing_input_playback::input_capture::{
    trigger_input_capture_on_exit, BeginInputCapture, InputCapturePlugin, InputModesCaptured,
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, InputCapturePlugin))
        .add_systems(Last, trigger_input_capture_on_exit);
    app.world_mut().trigger(BeginInputCapture {
        input_modes_captured: InputModesCaptured {
            keyboard: true,
            ..InputModesCaptured::DISABLE_ALL
        },
        filepath: Some("./data/test_playback.ron".to_string()),
        ..Default::default()
    });
    app.run();
}

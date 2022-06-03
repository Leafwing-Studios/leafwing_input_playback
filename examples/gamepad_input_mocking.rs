//! Demonstrates how to mock gamepad inputs
//!
//! This is somewhat more involved than mocking mouse and keyboard input,
//! as gamepad input is associated with a specific registered gamepad.

use bevy::prelude::*;
use bevy_input::InputPlugin;
use leafwing_input_playback::MockInput;

fn main() {
    let mut app = App::new();

    // Then, configure it
    // This is often useful to save with helper functions so you can reuse it between tests
    app.add_plugins(MinimalPlugins)
        .add_plugin(InputPlugin)
        .add_system(toggle_resource_on_start_button)
        .insert_resource(Toggle::Off);

    // Gamepads are registered in the `Gamepads` resource

    // We need to first simulate a connection

    // Gamepad input is specific to a controller

    // But some systems may accept input from any controller
}

#[derive(PartialEq, Debug)]
enum Toggle {
    Off,
    On,
}

// Systems that read gamepad input can work on either inputs sent by any gamepad
fn toggle_resource_on_start_button() {
    todo!()
}

// Or systems can be particular about which gamepad they accept inputs from

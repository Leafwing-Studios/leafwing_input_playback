//! Demonstrates how to mock gamepad inputs
//!
//! This is somewhat more involved than mocking mouse and keyboard input,
//! as gamepad input is associated with a specific registered gamepad.

use bevy::prelude::*;
use bevy_input::InputPlugin;
use leafwing_input_playback::{MockInput, RegisterGamepads};

fn main() {
    let mut app = App::new();

    // Then, configure it
    // This is often useful to save with helper functions so you can reuse it between tests
    app.add_plugins(MinimalPlugins)
        .add_plugin(InputPlugin)
        .add_system(jump::<Player1>)
        .add_system(jump::<Player2>)
        .add_system(pause_game)
        .insert_resource(Toggle::Off);

    // Gamepads are registered in the `Gamepads` resource
    let gamepads = app.world.resource::<Gamepads>();
    assert_eq!(gamepads.iter().count(), 0);

    // We need to first simulate a connection
    // The GamepadRegistration trait provides convenient methods on `App` and `World` for this
    app.register_gamepad(Gamepad(0));
    let gamepads = app.world.resource::<Gamepads>();
    assert_eq!(gamepads.iter().count(), 1);

    // Gamepad input is specific to a controller

    // But some systems may accept input from any controller
}

#[derive(PartialEq, Debug)]
enum Toggle {
    Off,
    On,
}

// Systems that read gamepad input can work on either inputs sent by any gamepad
fn pause_game() {
    if gamepad_input.pressed(KeyCode::Space) {
        *toggle = match *toggle {
            Toggle::Off => Toggle::On,
            Toggle::On => Toggle::Off,
        }
    }
}

// Or systems can be particular about which gamepad they accept inputs from

//! Demonstrates how to mock gamepad inputs
//!
//! This is somewhat more involved than mocking mouse and keyboard input,
//! as gamepad input is associated with a specific registered gamepad.

use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_input::InputPlugin;
use leafwing_input_playback::{MockInput, RegisterGamepads};

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugin(InputPlugin)
        .add_system(pause_game)
        .add_system(jump)
        .insert_resource(GamepadPlayerMap(HashMap::from_iter([
            (Gamepad(0), 0),
            (Gamepad(1), 1),
        ])));

    // Gamepads are registered in the `Gamepads` resource
    let gamepads = app.world.resource::<Gamepads>();
    assert_eq!(gamepads.iter().count(), 0);

    // We need to first simulate a connection
    // The GamepadRegistration trait provides convenient methods on `App` and `World` for this
    app.register_gamepad(Gamepad(0));
    app.register_gamepad(Gamepad(1));

    // The Gamepads resource lists all currently registered gamepads
    let gamepads = app.world.resource::<Gamepads>();
    assert_eq!(gamepads.iter().count(), 2);

    // Gamepad input is specific to a controller
    app.send_input_to_gamepad(GamepadButtonType::South, Some(Gamepad(0)));
    app.send_input_to_gamepad(GamepadButtonType::South, Some(Gamepad(1)));
    app.send_input_to_gamepad(GamepadButtonType::South, Some(Gamepad(0)));

    // But some systems may accept input from any controller
    app.send_input(GamepadButtonType::Start);

    // When all gamepads are deregistered, gamepad events will not be picked up
    app.deregister_gamepad(Gamepad(0));
    app.deregister_gamepad(Gamepad(1));
    app.send_input(GamepadButtonType::Start);
}

// Systems that read gamepad input can work on inputs sent by any gamepad
fn pause_game(gamepad_input: Res<Input<GamepadButton>>, gamepads: Res<Gamepads>) {
    for &gamepad in gamepads.iter() {
        let start_button = GamepadButton(gamepad, GamepadButtonType::Start);
        if gamepad_input.pressed(start_button) {
            println!("Game paused or unpaused.");
        }
    }
}

#[derive(Deref, DerefMut)]
struct GamepadPlayerMap(HashMap<Gamepad, u8>);

// Or systems can be particular about which gamepad they accept inputs from
fn jump(gamepad_input: Res<Input<GamepadButton>>, gamepad_player_map: Res<GamepadPlayerMap>) {
    for (&gamepad, player) in gamepad_player_map.iter() {
        let a_button = GamepadButton(gamepad, GamepadButtonType::South);
        if gamepad_input.pressed(a_button) {
            println!("Player {player} jumped!");
        }
    }
}

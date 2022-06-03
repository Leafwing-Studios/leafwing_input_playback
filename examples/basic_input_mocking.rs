//! Demonstrates how to use input mocking to send fake inputs.
//!
//! This is particularly valuable when testing Bevy apps, as mocked inputs can be performed reliably and automatically.

use bevy::prelude::*;
use bevy_input::InputPlugin;
// Check out this trait to see the convenient methods on `App`, `World` and `MutableInputStreams`!
use leafwing_input_playback::MockInput;

// Usually, this would be an integration test,
// in the root level `tests` directory
// annotated with the #[test] macro
fn main() {
    // Create and store your new App
    let mut app = App::new();

    // Then, configure it
    // This is often useful to save with helper functions so you can reuse it between tests
    app.add_plugins(MinimalPlugins)
        .add_plugin(InputPlugin)
        .add_system(toggle_resource_on_space)
        .insert_resource(Toggle::Off);

    // Type inference makes writing tests quick and easy
    assert_eq!(Toggle::Off, *app.world.resource());

    // And we can quickly debug state too
    dbg!(app.world.resource::<Toggle>());

    // Update your app one tick at a time to get full control
    app.update();

    // No trickery here
    assert_eq!(Toggle::Off, *app.world.resource());

    // Sending inputs is a breeze
    app.send_input(KeyCode::Space);

    // Oops, forgot to update the app...
    assert_eq!(Toggle::Off, *app.world.resource());

    // There we go
    app.update();
    assert_eq!(Toggle::On, *app.world.resource());

    // Inputs stay pressed until released,
    // so updating our app causes the toggle to be turned back off
    app.update();
    assert_eq!(Toggle::Off, *app.world.resource());

    // We can release them one at a time
    app.release_input(KeyCode::Space);

    // Or all at once
    app.reset_inputs();
}

#[derive(PartialEq, Debug)]
enum Toggle {
    Off,
    On,
}

fn toggle_resource_on_space(keyboard_input: Res<Input<KeyCode>>, mut toggle: ResMut<Toggle>) {
    if keyboard_input.pressed(KeyCode::Space) {
        *toggle = match *toggle {
            Toggle::Off => Toggle::On,
            Toggle::On => Toggle::Off,
        }
    }
}

use bevy::prelude::*;
use bevy::window::{CursorMoved, WindowPlugin};

const TARGET_POSITION: Vec2 = Vec2 { x: 42.0, y: 712.3 };

// FIXME: this doesn't run
// We've disabled the default harness to force main thread running
// As a result, we have to use this mildly cursed strategy for declaring tests
// rather than the standard #[test] annotation
fn main() {
    mock_input_moved();
}

fn mock_input_moved() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins).add_plugin(WindowPlugin);

    let windows = app.world.resource::<Windows>();
    let primary_window = windows.get_primary().unwrap();
    let primary_window_id = primary_window.id();

    app.world.send_event(CursorMoved {
        id: primary_window_id,
        position: TARGET_POSITION,
    });

    // Let the app respond to the cursor movement
    app.update();

    let windows = app.world.resource::<Windows>();
    let primary_window = windows.get_primary().unwrap();
    let cursor_position = primary_window.cursor_position().unwrap();

    assert_eq!(cursor_position, TARGET_POSITION);
}

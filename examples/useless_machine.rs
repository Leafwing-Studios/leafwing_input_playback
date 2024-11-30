//! [`AppExit`] events are played back and captured too!
//!
//! This example loads the file, which only contains an `AppExit`,
//! and then immediately quits itself as soon as it is encountered.
use bevy::prelude::*;
use leafwing_input_playback::input_playback::{
    BeginInputPlayback, InputPlaybackPlugin, InputPlaybackSource,
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, InputPlaybackPlugin));
    app.world_mut().trigger(BeginInputPlayback {
        source: Some(InputPlaybackSource::from_file("./data/app_exit.ron")),
        ..Default::default()
    });
    app.run();
}

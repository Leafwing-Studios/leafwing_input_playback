use bevy::prelude::*;
use leafwing_input_playback::{capture::InputCapturePlugin, playback_data::PlaybackData};
use winit::event_loop::EventLoopProxy;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(send_direct_window_event)
        .run()
}

/// Extracts the NonSend resource directly, and attempts to write to it
fn send_direct_window_event(mut winit_event_loop_proxy: NonSend<EventLoopProxy<()>>) {
    dbg!(winit_event_loop_proxy);
}

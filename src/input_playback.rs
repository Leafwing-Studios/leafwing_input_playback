//! Reads user input from a single [`UnifiedInput`] event stream
//!
//! These are played back by emulating assorted Bevy input events

use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::prelude::*;

use crate::unified_input::UnifiedInput;

/// Reads from the [`UnifiedInput`] event stream to determine which events to play back.
///
/// Events are played back during [`CoreStage::First`] to accurately mimic the behavior of native `winit`-based inputs.
pub struct InputPlaybackPlugin;

impl Plugin for InputPlaybackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnifiedInput>()
            .add_system_to_stage(CoreStage::First, playback_unified_input);
    }
}

// UnifiedInput is an iterator, so we need mutable access to be able to track which events we've seen
fn playback_unified_input(unified_input: ResMut<UnifiedInput>) {}

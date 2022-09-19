//! Reads user input from a single [`UnifiedInput`] event stream
//!
//! These are played back by emulating assorted Bevy input events

use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::prelude::*;

use crate::unified_input::UnifiedInputEvent;

/// Reads from the [`UnifiedInput`] event stream to determine which events to play back.
///
/// Events are played back during [`CoreStage::First`] to accurately mimic the behavior of native `winit`-based inputs.
pub struct InputPlaybackPlugin;

impl Plugin for InputPlaybackPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<UnifiedInputEvent>()
            .add_system_to_stage(CoreStage::First, playback_unified_input);
    }
}

fn playback_unified_input(unified_input: Res<UnifiedInputEvent>) {}

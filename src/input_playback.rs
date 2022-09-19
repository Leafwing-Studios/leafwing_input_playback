//! Reads user input from a single [`UnifiedInput`] event stream
//!
//! These are played back by emulating assorted Bevy input events

use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseWheel},
};
use bevy_time::Time;
use bevy_utils::Duration;
use bevy_window::CursorMoved;

use crate::unified_input::{FrameCount, TimestampedInputEvent, UnifiedInput};

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

/// Controls the approach used for playing back recorded inputs
///
/// [`PlaybackStrategy::Time`] is the default strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackStrategy {
    /// Plays events up to (but not past) the current [`Time`].
    ///
    /// This strategy is more reliable, as it will ensure that systems which rely on elapsed time function correctly.
    #[default]
    Time,
    /// Plays events up to (but not past) the current [`FrameCount`].
    ///
    /// This strategy is faster, as you can turn off any frame rate limiting mechanism.
    FrameCount,
    /// Plays events between the first and second [`Duration`], measured in time since app startup.
    ///
    /// This range is inclusive at the bottom, but exclusive at the top to avoid double-counting.
    TimeRange(Duration, Duration),
    /// Plays events between the first and second [`FrameCount`].
    ///
    /// This range is inclusive at the bottom, but exclusive at the top to avoid double-counting.
    FrameRange(FrameCount, FrameCount),
    /// Does not playback any events.
    ///
    /// This is useful for interactive use cases, to temporarily disable sending events.
    Paused,
}

/// The [`EventWriter`] types that correspond to the input event types stored in [`InputEvent`](crate::unified_input::InputEvent)
#[derive(SystemParam)]
#[allow(missing_docs)]
pub struct InputWriters<'w, 's> {
    pub keyboard_input: EventWriter<'w, 's, KeyboardInput>,
    pub mouse_button_input: EventWriter<'w, 's, MouseButtonInput>,
    pub mouse_wheel: EventWriter<'w, 's, MouseWheel>,
    pub cursor_moved: EventWriter<'w, 's, CursorMoved>,
}

// UnifiedInput is an iterator, so we need mutable access to be able to track which events we've seen
/// A system that reads from the [`UnifiedInput`] resources and plays back the contained events.
///
/// The strategy used is based on [`PlaybackStrategy`].
pub fn playback_unified_input(
    mut unified_input: ResMut<UnifiedInput>,
    playback_strategy: Res<PlaybackStrategy>,
    time: Res<Time>,
    frame_count: Res<FrameCount>,
    mut input_writers: InputWriters,
) {
    use PlaybackStrategy::*;

    // We cannot store the iterator, as different opaque return types are used
    match *playback_strategy {
        Time => {
            let input_events = unified_input.iter_until_time(time.time_since_startup());
            send_playback_events(input_events, &mut input_writers);
        }
        FrameCount => {
            let input_events = unified_input.iter_until_frame(*frame_count);
            send_playback_events(input_events, &mut input_writers);
        }
        TimeRange(start, end) => {
            let input_events = unified_input.iter_between_times(start, end);
            send_playback_events(input_events, &mut input_writers);
        }
        FrameRange(start, end) => {
            let input_events = unified_input.iter_between_frames(start, end);
            send_playback_events(input_events, &mut input_writers)
        }
        Paused => {
            // Do nothing
        }
    };
}

fn send_playback_events(
    timestamped_input_events: impl IntoIterator<Item = TimestampedInputEvent>,
    input_writers: &mut InputWriters,
) {
    for timestamped_input_event in timestamped_input_events {
        use crate::unified_input::InputEvent::*;
        match timestamped_input_event.input_event {
            Keyboard(e) => input_writers.keyboard_input.send(e),
            MouseButton(e) => input_writers.mouse_button_input.send(e),
            MouseWheel(e) => input_writers.mouse_wheel.send(e),
            CursorMoved(e) => input_writers.cursor_moved.send(e),
        };
    }
}

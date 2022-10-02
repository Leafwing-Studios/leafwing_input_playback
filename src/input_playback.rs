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

use crate::frame_counting::{frame_counter, FrameCount};
use crate::unified_input::{TimestampedInputEvent, UnifiedInput};

/// Reads from the [`UnifiedInput`] event stream to determine which events to play back.
///
/// Events are played back during [`CoreStage::First`] to accurately mimic the behavior of native `winit`-based inputs.
/// Which events are played back are controlled via the [`PlaybackStrategy`] resource.
pub struct InputPlaybackPlugin;

impl Plugin for InputPlaybackPlugin {
    fn build(&self, app: &mut App) {
        // Avoid double-adding frame_counter
        if !app.world.contains_resource::<FrameCount>() {
            app.init_resource::<FrameCount>()
                .add_system_to_stage(CoreStage::First, frame_counter);
        }

        app.init_resource::<UnifiedInput>()
            .init_resource::<PlaybackProgress>()
            .init_resource::<PlaybackStrategy>()
            .add_system_to_stage(
                CoreStage::First,
                playback_unified_input.after(frame_counter),
            );
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
    /// Plays events between the first and second [`Duration`] once, measured in time since app startup.
    ///
    /// The events are played back at the same rate they were captured.
    /// This range includes events sent at the start of the range, but not the end.
    TimeRangeOnce(Duration, Duration),
    /// Plays events between the first and second [`Duration`] indefinitely, measured in time since app startup.
    ///
    /// The events are played back at the same rate they were captured.
    /// This range includes events sent at the start of the range, but not the end.
    TimeRangeLoop(Duration, Duration),
    /// Plays events between the first and second [`FrameCount`] once.
    ///
    /// The events are played back at the same rate they were captured.
    /// This range includes events sent at the start of the range, but not the end.
    FrameRangeOnce(FrameCount, FrameCount),
    /// Plays events between the first and second [`FrameCount`] indefinitely.
    ///
    /// The events are played back at the same rate they were captured.
    /// This range includes events sent at the start of the range, but not the end.
    FrameRangeLoop(FrameCount, FrameCount),
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
    mut playback_strategy: ResMut<PlaybackStrategy>,
    time: Res<Time>,
    frame_count: Res<FrameCount>,
    mut input_writers: InputWriters,
    mut playback_progress: ResMut<PlaybackProgress>,
) {
    // We cannot store the iterator, as different opaque return types are used
    match *playback_strategy {
        PlaybackStrategy::Time => {
            let input_events = unified_input.iter_until_time(time.time_since_startup());
            send_playback_events(input_events, &mut input_writers);
        }
        PlaybackStrategy::FrameCount => {
            let input_events = unified_input.iter_until_frame(*frame_count);
            send_playback_events(input_events, &mut input_writers);
        }
        PlaybackStrategy::TimeRangeOnce(start, end) => {
            if playback_progress.initial_time.is_none() {
                playback_progress.initial_time = Some(time.time_since_startup());
            }
            let initial_time = playback_progress.initial_time.unwrap();

            let playback_start = playback_progress.last_seen_time.unwrap_or(start);
            // Make sure we're advancing at a one-to-one rate
            let playback_end = initial_time + time.delta();
            playback_progress.last_seen_time = Some(playback_end);

            let input_events =
                unified_input.iter_between_times(playback_start, playback_end.min(end));
            send_playback_events(input_events, &mut input_writers);

            // If we've covered the entire range, reset our progress
            if playback_end > end {
                *playback_progress = PlaybackProgress::default();
                // We only want to play back once, so pause.
                *playback_strategy = PlaybackStrategy::Paused;
            }
        }
        PlaybackStrategy::FrameRangeOnce(start, end) => {
            if playback_progress.initial_frame.is_none() {
                playback_progress.initial_frame = Some(*frame_count);
            }
            let initial_frame = playback_progress.initial_frame.unwrap();

            let playback_start = playback_progress.last_seen_frame.unwrap_or(start);
            // Make sure we're advancing at a one-to-one rate
            let playback_end = initial_frame + FrameCount(1);
            playback_progress.last_seen_frame = Some(playback_end);

            let input_events =
                unified_input.iter_between_frames(playback_start, playback_end.min(end));
            send_playback_events(input_events, &mut input_writers);

            // If we've covered the entire range, reset our progress
            if playback_end > end {
                *playback_progress = PlaybackProgress::default();
                // We only want to play back once, so pause.
                *playback_strategy = PlaybackStrategy::Paused;
            }
        }
        PlaybackStrategy::TimeRangeLoop(start, end) => {
            if playback_progress.initial_time.is_none() {
                playback_progress.initial_time = Some(time.time_since_startup());
            }
            let initial_time = playback_progress.initial_time.unwrap();

            let playback_start = playback_progress.last_seen_time.unwrap_or(start);
            // Make sure we're advancing at a one-to-one rate
            let playback_end = initial_time + time.delta();
            playback_progress.last_seen_time = Some(playback_end);

            let input_events =
                unified_input.iter_between_times(playback_start, playback_end.min(end));
            send_playback_events(input_events, &mut input_writers);

            // If we've covered the entire range, reset our progress
            if playback_end > end {
                *playback_progress = PlaybackProgress::default();
            }
        }
        PlaybackStrategy::FrameRangeLoop(start, end) => {
            if playback_progress.initial_frame.is_none() {
                playback_progress.initial_frame = Some(*frame_count);
            }
            let initial_frame = playback_progress.initial_frame.unwrap();

            let playback_start = playback_progress.last_seen_frame.unwrap_or(start);
            // Make sure we're advancing at a one-to-one rate
            let playback_end = initial_frame + FrameCount(1);
            playback_progress.last_seen_frame = Some(playback_end);

            let input_events =
                unified_input.iter_between_frames(playback_start, playback_end.min(end));
            send_playback_events(input_events, &mut input_writers);

            // If we've covered the entire range, reset our progress
            if playback_end > end {
                *playback_progress = PlaybackProgress::default();
            }
        }
        PlaybackStrategy::Paused => {
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

/// How far through the current cycle of input playback we've gotten.
///
/// Used in the [`playback_unified_input`] system to track progress.
#[derive(Default, Debug, PartialEq)]
pub struct PlaybackProgress {
    initial_time: Option<Duration>,
    last_seen_time: Option<Duration>,
    initial_frame: Option<FrameCount>,
    last_seen_frame: Option<FrameCount>,
}

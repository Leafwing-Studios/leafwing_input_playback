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
            let input_events = unified_input.iter_between_times(
                playback_progress.current_time(start),
                playback_progress.next_time(time.delta(), start),
            );
            send_playback_events(input_events, &mut input_writers);

            // If we've covered the entire range, reset our progress
            if playback_progress.current_time(start) > end {
                playback_progress.reset(&mut *unified_input);
                // We only want to play back once, so pause.
                *playback_strategy = PlaybackStrategy::Paused;
            }
        }
        PlaybackStrategy::FrameRangeOnce(start, end) => {
            let input_events = unified_input.iter_between_frames(
                playback_progress.current_frame(start),
                playback_progress.next_frame(start),
            );
            send_playback_events(input_events, &mut input_writers);

            // If we've covered the entire range, reset our progress
            if playback_progress.current_frame(start) > end {
                playback_progress.reset(&mut *unified_input);
                // We only want to play back once, so pause.
                *playback_strategy = PlaybackStrategy::Paused;
            }
        }
        PlaybackStrategy::TimeRangeLoop(start, end) => {
            let input_events = unified_input.iter_between_times(
                playback_progress.current_time(start),
                playback_progress.next_time(time.delta(), start),
            );
            send_playback_events(input_events, &mut input_writers);

            // If we've covered the entire range, reset our progress
            if playback_progress.current_time(start) > end {
                playback_progress.reset(&mut *unified_input);
            }
        }
        PlaybackStrategy::FrameRangeLoop(start, end) => {
            let input_events = unified_input.iter_between_frames(
                playback_progress.current_frame(start),
                playback_progress.next_frame(start),
            );
            send_playback_events(input_events, &mut input_writers);

            // If we've covered the entire range, reset our progress
            if playback_progress.current_frame(start) > end {
                playback_progress.reset(&mut *unified_input);
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
    // We must be caeful not to consume these values
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
/// The `initial_time` and `initial_frame` are stored to be able to compute
/// the offset between the actual time (frame count) and the time (frame count) of the recording.
///
/// Used in the [`playback_unified_input`] system to track progress.
#[derive(Default, Debug, PartialEq, Clone)]
pub struct PlaybackProgress {
    /// The [`Duration`] that this playback loop has been running for
    pub elapsed_time: Duration,
    /// The number of frames that this playback loop has been running for
    pub elapsed_frames: FrameCount,
}

impl PlaybackProgress {
    /// Gets the current frame.
    ///
    /// # Panics
    ///
    /// Panics if `self.initial_frame` is `None`. Make sure to call `set_initial_frame` first!
    pub fn current_frame(&self, start: FrameCount) -> FrameCount {
        start + self.elapsed_frames
    }

    /// Gets the current time.
    ///
    /// # Panics
    ///
    /// Panics if `self.initial_time` is `None`. Make sure to call `set_initial_time` first!
    pub fn current_time(&self, start: Duration) -> Duration {
        start + self.elapsed_time
    }

    /// Get the start of the next frame window to play back.
    ///
    /// This also records that one frame has elapsed.
    pub fn next_frame(&mut self, start: FrameCount) -> FrameCount {
        self.elapsed_frames = self.elapsed_frames + FrameCount(1);
        // The frame count has been advanced, so this returns the correct value
        self.current_frame(start)
    }

    /// Get the start of the next time window to play back.
    ///
    /// This also records that a `delta` of time has elapsed.
    pub fn next_time(&mut self, delta: Duration, start: Duration) -> Duration {
        self.elapsed_time = self.elapsed_time + delta;
        // Time has been advanced, so this returns the correct value
        self.current_time(start)
    }

    /// Resets all tracked progress.
    ///
    /// This is called when the current pass of the playback loop elapses.
    pub fn reset(&mut self, unified_input: &mut UnifiedInput) {
        unified_input.reset_cursor();
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_time() {
        let mut progress = PlaybackProgress::default();
        let start = Duration::from_secs(1);

        assert_eq!(progress.current_time(start), start);

        let delta = Duration::from_secs(1);
        let next_time = progress.next_time(delta, start);
        assert_eq!(next_time, start + delta);
        assert_eq!(progress.elapsed_time, delta);
    }

    #[test]
    fn current_frame() {
        let mut progress = PlaybackProgress::default();

        let start = FrameCount(1);

        assert_eq!(progress.current_frame(start), start);

        let delta = FrameCount(1);
        let next_frame = progress.next_frame(start);
        assert_eq!(next_frame, start + delta);
        assert_eq!(progress.elapsed_frames, delta);
    }
}

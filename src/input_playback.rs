//! Reads user input from a single [`TimestampedInputs`](crate::timestamped_input::TimestampedInputs) resource.
//!
//! These are played back by emulating assorted Bevy input events.

use bevy::app::{App, AppExit, First, Plugin};
use bevy::core::FrameCount;
use bevy::ecs::{prelude::*, system::SystemParam};
use bevy::input::{
    gamepad::GamepadEvent,
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseWheel},
};
use bevy::log::warn;
use bevy::time::Time;
use bevy::utils::Duration;
use bevy::window::{CursorMoved, PrimaryWindow, Window};
use ron::de::from_reader;
use std::fs::File;

use crate::serde::PlaybackFilePath;
use crate::timestamped_input::{TimestampedInputEvent, TimestampedInputs};

/// Reads from the [`TimestampedInputs`] event stream to determine which events to play back.
///
/// Events are played back during the [`First`] schedule to accurately mimic the behavior of native `winit`-based inputs.
/// Which events are played back are controlled via the [`PlaybackStrategy`] resource.
///  
/// Input is deserialized on app startup from the path stored in the [`PlaybackFilePath`] resource, if any.
pub struct InputPlaybackPlugin;

impl Plugin for InputPlaybackPlugin {
    fn build(&self, app: &mut App) {
        app.observe(BeginInputPlayback::observer)
            .observe(EndInputPlayback::observer)
            .add_systems(
                First,
                playback_timestamped_input
                    .run_if(
                        resource_exists::<PlaybackProgress>
                            .and_then(resource_exists::<TimestampedInputs>),
                    )
                    .after(bevy::ecs::event::EventUpdates),
            );
    }
}

/// An Observer that users can trigger to initiate input capture.
///
/// Data is serialized to the provided `filepath` when either an [`EndCaptureEvent`] or an [`AppExit`] event is detected.
#[derive(Debug, Default, Event)]
pub struct BeginInputPlayback {
    /// The source from which to read input data. Do not provide a `source` if the expected `TimestampedInputs` should already be present in `World`.
    pub source: Option<InputPlaybackSource>,
    /// Controls the approach used for playing back recorded inputs.
    ///
    /// See [`PlaybackStrategy`] for more information.
    pub playback_strategy: PlaybackStrategy,
    /// A entity corresponding to the [`bevy::window::Window`] which will receive input events.
    /// If unspecified, input events will target the serialized window entity, which may be fragile.
    pub playback_window: Option<PlaybackWindow>,
}

impl BeginInputPlayback {
    /// An `ObserverSystem` for `BeginInputPlayback` that deserializes timestamped inputs from a playback source (if provided) and attaches all playback-related resources.
    pub fn observer(trigger: Trigger<BeginInputPlayback>, mut commands: Commands) {
        let event = trigger.event();
        commands.init_resource::<PlaybackProgress>();
        commands.insert_resource(event.playback_strategy);

        if let Some(source) = event.source.as_ref() {
            let timestamped_inputs = match source {
                InputPlaybackSource::TimestampedInputs(inputs) => inputs.clone(),
                InputPlaybackSource::File(playback_path) => {
                    commands.insert_resource(playback_path.clone());
                    deserialize_timestamped_inputs(playback_path)
                        .unwrap()
                        .unwrap()
                }
            };
            commands.insert_resource(timestamped_inputs);
        }

        if let Some(playback_window) = &event.playback_window {
            commands.insert_resource(playback_window.clone());
        }
    }
}

/// The source of input data for playback.
///
/// Typically users should expect to provide a `FilePath`, but `TimestampedInputs` can still be provided manually.
#[derive(Debug)]
pub enum InputPlaybackSource {
    /// Reads from a file and deserializes the content into a `TimestampedInputs`.
    File(PlaybackFilePath),
    /// Uses the provided `TimestampedInputs` parameter as the source of input data.
    TimestampedInputs(TimestampedInputs),
}

impl InputPlaybackSource {
    /// Reads source data from a file using the provided filepath.
    pub fn from_file(filepath: impl Into<String>) -> Self {
        InputPlaybackSource::File(PlaybackFilePath::new(&filepath.into()))
    }

    /// Defines source data using raw data.
    pub fn from_inputs(inputs: TimestampedInputs) -> Self {
        InputPlaybackSource::TimestampedInputs(inputs)
    }
}

impl Default for InputPlaybackSource {
    fn default() -> Self {
        Self::TimestampedInputs(TimestampedInputs::default())
    }
}

/// An Observer that users can trigger to end input playback prematurely.
#[derive(Debug, Event)]
pub struct EndInputPlayback;

impl EndInputPlayback {
    /// An `ObserverSystem` for `EndInputPlayback` that removes playback-related resources including previously-recorded inputs.
    fn observer(_trigger: Trigger<EndInputPlayback>, mut commands: Commands) {
        commands.remove_resource::<PlaybackFilePath>();
        commands.remove_resource::<TimestampedInputs>();
        commands.remove_resource::<PlaybackProgress>();
        commands.remove_resource::<PlaybackStrategy>();
        commands.remove_resource::<PlaybackWindow>();
    }
}

/// The `Window` entity that will receive capture events from the .
///
/// If this Resource is attached, input events will be forwarded to this window entity rather than the serialized window entity.
#[derive(Clone, Debug, Default, Resource)]
pub enum PlaybackWindow {
    /// Overrides the serialized window entity with the current `PrimaryWindow` entity.
    ///
    /// This is the most common behavior.
    #[default]
    PrimaryWindow,
    /// Overrides the serialized window entity with a specific Window entity.
    ///
    /// This can be used for input playback in multi-window applications.
    Window(Entity),
}

/// Controls the approach used for playing back recorded inputs
///
/// [`PlaybackStrategy::Time`] is the default strategy.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    /// There will always be one frame between the end of the previous loop and the start of the next.
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
    /// There will always be one frame between the end of the previous loop and the start of the next.
    FrameRangeLoop(FrameCount, FrameCount),
    /// Does not playback any events.
    ///
    /// This is useful for interactive use cases, to temporarily disable sending events.
    Paused,
}

/// The [`EventWriter`] types that correspond to the input event types stored in [`InputEvent`](crate::timestamped_input::InputEvent)
#[derive(SystemParam)]
#[allow(missing_docs)]
pub struct InputWriters<'w, 's> {
    pub keyboard_input: EventWriter<'w, KeyboardInput>,
    pub mouse_button_input: EventWriter<'w, MouseButtonInput>,
    pub mouse_wheel: EventWriter<'w, MouseWheel>,
    pub cursor_moved: EventWriter<'w, CursorMoved>,
    pub windows: Query<'w, 's, &'static mut Window>,
    pub gamepad: EventWriter<'w, GamepadEvent>,
    pub app_exit: EventWriter<'w, AppExit>,
}

// `TimestampedInputs` is an iterator, so we need mutable access to be able to track which events we've seen
/// A system that reads from the [`TimestampedInputs`] resources and plays back the contained events.
///
/// The strategy used is based on [`PlaybackStrategy`].
/// Additionally, `Keyboard`, `MouseButton`, and `MouseWheel` events may target `Window` entities according to the [`PlaybackWindow`].
#[allow(clippy::too_many_arguments)]
pub fn playback_timestamped_input(
    mut timestamped_input: ResMut<TimestampedInputs>,
    mut playback_strategy: ResMut<PlaybackStrategy>,
    playback_window: Option<Res<PlaybackWindow>>,
    time: Res<Time>,
    primary_window: Query<Entity, (With<Window>, With<PrimaryWindow>)>,
    frame_count: Res<FrameCount>,
    mut input_writers: InputWriters,
    mut playback_progress: ResMut<PlaybackProgress>,
) {
    let window_override = match playback_window.as_deref() {
        Some(PlaybackWindow::PrimaryWindow) => Some(primary_window.single()),
        Some(PlaybackWindow::Window(entity)) => Some(*entity),
        None => None,
    };
    // We cannot store the iterator, as different opaque return types are used
    match *playback_strategy {
        PlaybackStrategy::Time => {
            let input_events = timestamped_input.iter_until_time(time.elapsed());
            send_playback_events(input_events, &mut input_writers, window_override);
        }
        PlaybackStrategy::FrameCount => {
            let input_events = timestamped_input.iter_until_frame(*frame_count);
            send_playback_events(input_events, &mut input_writers, window_override);
        }
        PlaybackStrategy::TimeRangeOnce(start, end) => {
            let input_events = timestamped_input.iter_between_times(
                playback_progress.current_time(start),
                playback_progress.next_time(time.delta(), start),
            );
            send_playback_events(input_events, &mut input_writers, window_override);

            // If we've covered the entire range, reset our progress
            if playback_progress.current_time(start) > end {
                playback_progress.reset(timestamped_input.into_inner());
                // We only want to play back once, so pause.
                *playback_strategy = PlaybackStrategy::Paused;
            }
        }
        PlaybackStrategy::FrameRangeOnce(start, end) => {
            let input_events = timestamped_input.iter_between_frames(
                playback_progress.current_frame(start),
                playback_progress.next_frame(start),
            );
            send_playback_events(input_events, &mut input_writers, window_override);

            // If we've covered the entire range, reset our progress
            if playback_progress.current_frame(start) > end {
                playback_progress.reset(timestamped_input.into_inner());
                // We only want to play back once, so pause.
                *playback_strategy = PlaybackStrategy::Paused;
            }
        }
        PlaybackStrategy::TimeRangeLoop(start, end) => {
            let input_events = timestamped_input.iter_between_times(
                playback_progress.current_time(start),
                playback_progress.next_time(time.delta(), start),
            );
            send_playback_events(input_events, &mut input_writers, window_override);

            // If we've covered the entire range, reset our progress
            if playback_progress.current_time(start) > end {
                playback_progress.reset(timestamped_input.into_inner());
            }
        }
        PlaybackStrategy::FrameRangeLoop(start, end) => {
            let input_events = timestamped_input.iter_between_frames(
                playback_progress.current_frame(start),
                playback_progress.next_frame(start),
            );
            send_playback_events(input_events, &mut input_writers, window_override);

            // If we've covered the entire range, reset our progress
            if playback_progress.current_frame(start) > end {
                playback_progress.reset(timestamped_input.into_inner());
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
    window_override: Option<Entity>,
) {
    for timestamped_input_event in timestamped_input_events {
        use crate::timestamped_input::InputEvent::*;
        match timestamped_input_event.input_event {
            Keyboard(mut e) => {
                if let Some(entity) = window_override {
                    e.window = entity;
                }
                input_writers.keyboard_input.send(e);
            }
            MouseButton(mut e) => {
                if let Some(entity) = window_override {
                    e.window = entity;
                }
                input_writers.mouse_button_input.send(e);
            }
            MouseWheel(mut e) => {
                if let Some(entity) = window_override {
                    e.window = entity;
                }
                input_writers.mouse_wheel.send(e);
            }
            // Window events MUST update the `Window` struct itself
            // BLOCKED: https://github.com/bevyengine/bevy/issues/6163
            CursorMoved(e) => {
                if let Ok(mut window) = input_writers.windows.get_mut(e.window) {
                    window.set_cursor_position(Some(e.position));
                } else {
                    warn!("Window entity was not found when attempting to play back {e:?}")
                }

                input_writers.cursor_moved.send(e);
            }
            Gamepad(e) => {
                input_writers.gamepad.send(e);
            }
            AppExit => {
                input_writers.app_exit.send_default();
            }
        };
    }
}

/// Reads the stored file paths from the [`PlaybackFilePath`] location (if any)
pub fn deserialize_timestamped_inputs(
    playback_path: &PlaybackFilePath,
) -> Option<Result<TimestampedInputs, TimestampedInputsError>> {
    playback_path.path().as_ref().map(|file_path| {
        let file = File::open(file_path).map_err(TimestampedInputsError::Fs)?;
        from_reader(file).map_err(TimestampedInputsError::Ron)
    })
}

/// An error type that wraps the possible error variants when deserializing `TimestampedInputs` from a file.
#[derive(Debug)]
pub enum TimestampedInputsError {
    /// The error case where the filesystem failed to open the desired file path.
    Fs(std::io::Error),
    /// The error case where the content at the provided filepath did not have valid RON content.
    Ron(ron::de::SpannedError),
}

impl std::fmt::Display for TimestampedInputsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TimestampedInputsError::Fs(_error) => write!(f, "could not find playback file "),
            TimestampedInputsError::Ron(_error) => {
                write!(f, "the provided file did not have valid RON-formatted data")
            }
        }
    }
}

impl std::error::Error for TimestampedInputsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            TimestampedInputsError::Fs(ref error) => Some(error),
            TimestampedInputsError::Ron(ref error) => Some(error),
        }
    }
}

/// How far through the current cycle of input playback we've gotten.
///
/// The `initial_time` and `initial_frame` are stored to be able to compute
/// the offset between the actual time (frame count) and the time (frame count) of the recording.
///
/// Used in the [`playback_timestamped_input`] system to track progress.
#[derive(Resource, Default, Debug, PartialEq, Eq, Clone)]
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
        FrameCount(start.0.wrapping_add(self.elapsed_frames.0))
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
        self.elapsed_frames = FrameCount(self.elapsed_frames.0.wrapping_add(1));
        // The frame count has been advanced, so this returns the correct value
        self.current_frame(start)
    }

    /// Get the start of the next time window to play back.
    ///
    /// This also records that a `delta` of time has elapsed.
    pub fn next_time(&mut self, delta: Duration, start: Duration) -> Duration {
        self.elapsed_time += delta;
        // Time has been advanced, so this returns the correct value
        self.current_time(start)
    }

    /// Resets all tracked progress.
    ///
    /// This is called when the current pass of the playback loop elapses.
    pub fn reset(&mut self, timestamped_input: &mut TimestampedInputs) {
        timestamped_input.reset_cursor();
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
        assert_eq!(next_frame.0, start.0.wrapping_add(delta.0));
        assert_eq!(progress.elapsed_frames, delta);
    }
}

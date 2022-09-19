//! Captures user input from assorted raw `Event` types.
//!
//! These are unified into a single [`UnifiedInput`] event stream, which can be played back
use std::collections::VecDeque;

use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::prelude::*;
use bevy_input::keyboard::KeyboardInput;
use bevy_input::mouse::{MouseButtonInput, MouseWheel};
use bevy_time::Time;
use bevy_utils::Duration;
use bevy_window::CursorMoved;

/// A timestamped device-agnostic user-input event
///
/// These are re-emitted as events, and commonly serialized to disk
#[derive(Debug, Clone)]
pub struct UnifiedInputEvent {
    /// The number of frames that have elapsed since the app began
    pub frame: FrameCount,
    /// The amount of time that has elapsed since the app began
    pub time_since_startup: Duration,
    /// The [`InputEvent`] that was captured
    pub input_event: InputEvent,
}

/// A resource that stores the complete event-like list of [`UnifiedInputEvent]s
///
/// Read and write to this struct when performing input capture and playback
#[derive(Debug, Clone)]
pub struct UnifiedInput {
    /// The underlying [`UnifiedInputEvent`] data
    ///
    /// New events are pushed to the back of the list.
    /// If input events are recorded immediately, the final list will be sorted,
    /// with older events at the start of the [`VecDeque`].
    ///
    /// This type implements the [`Iterator`] trait;
    /// typically you will want to use that rather than accessing the internal event storage.
    pub events: VecDeque<UnifiedInputEvent>,
    /// The index in `events` of the next event to read
    pub cursor: usize,
}

impl UnifiedInput {
    /// Records an `input_event`, making note of the frame and time that it was sent.
    pub fn send(
        &mut self,
        frame: FrameCount,
        time_since_startup: Duration,
        input_event: InputEvent,
    ) {
        self.events.push_back(UnifiedInputEvent {
            frame,
            time_since_startup,
            input_event,
        });
    }

    /// Records an iterable of input events, making note of the frame and time that it was sent.
    pub fn send_multiple(
        &mut self,
        frame: FrameCount,
        time_since_startup: Duration,
        event_stream: impl IntoIterator<Item = impl Into<InputEvent>>,
    ) {
        for event in event_stream.into_iter() {
            self.send(frame, time_since_startup, event.into());
        }
    }

    /// Sorts the input stream by either [`Time::time_since_startup`] or [`FrameCount`].
    pub fn sort(&mut self, strategy: SortingStrategy) {
        let strategy = match strategy {
            SortingStrategy::TimeSinceStartup => |a: &UnifiedInputEvent, b: &UnifiedInputEvent| {
                a.time_since_startup.cmp(&b.time_since_startup)
            },
            SortingStrategy::FrameCount => {
                |a: &UnifiedInputEvent, b: &UnifiedInputEvent| a.frame.cmp(&b.frame)
            }
        };

        self.events.make_contiguous().sort_by(strategy);
    }

    /// Is this [`UnifiedInput`] sorted according to the specified [`SortingStrategy`]?
    pub fn is_sorted(&self, strategy: SortingStrategy) -> bool {
        match strategy {
            SortingStrategy::FrameCount => {
                if self.events.is_empty() {
                    return true;
                }

                let mut last_framecount = FrameCount(0);
                for event in self.events.iter() {
                    let current_framecount = event.frame;
                    if current_framecount < last_framecount {
                        return false;
                    }
                    last_framecount = current_framecount;
                }
                true
            }
            SortingStrategy::TimeSinceStartup => {
                if self.events.is_empty() {
                    return true;
                }

                let mut last_time = Duration::ZERO;
                for event in self.events.iter() {
                    let current_time = event.time_since_startup;
                    if current_time < last_time {
                        return false;
                    }
                    last_time = current_time;
                }
                true
            }
        }
    }

    /// The frame count of the last-read event.
    pub fn last_framecount(&self) -> Option<FrameCount> {
        let last_read = self.events.get(self.cursor - 1)?;
        Some(last_read.frame)
    }

    /// The frame count of the next event to read.
    pub fn next_framecount(&self) -> Option<FrameCount> {
        let next_read = self.events.get(self.cursor)?;
        Some(next_read.frame)
    }

    /// The time since startup of the last-read event.
    pub fn last_time(&self) -> Option<Duration> {
        let last_read = self.events.get(self.cursor - 1)?;
        Some(last_read.time_since_startup)
    }

    /// The time since startup of the next event to read.
    pub fn next_time(&self) -> Option<Duration> {
        let next_read = self.events.get(self.cursor)?;
        Some(next_read.time_since_startup)
    }
}

impl Iterator for UnifiedInput {
    type Item = UnifiedInputEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let event = self.events.get(self.cursor).cloned();
        self.cursor += 1;
        event
    }
}

/// The sorting strategy used for [`UnifiedInput::sort`].
///
/// In all typical cases, these two sorting strategies should agree.
pub enum SortingStrategy {
    /// Sort by ascending frame count
    FrameCount,
    /// Sort by ascending time since startup
    TimeSinceStartup,
}

/// The number of frames that have elapsed since the app started
///
/// Updated in [`time_tracker`] during [`CoreStage::First`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameCount(pub u64);

/// Collects input-relevant events for use in [`UnifiedInput`]
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub enum InputEvent {
    Keyboard(KeyboardInput),
    MouseButton(MouseButtonInput),
    MouseWheel(MouseWheel),
    CursorMoved(CursorMoved),
}

impl From<KeyboardInput> for InputEvent {
    fn from(event: KeyboardInput) -> Self {
        InputEvent::Keyboard(event)
    }
}

impl From<MouseButtonInput> for InputEvent {
    fn from(event: MouseButtonInput) -> Self {
        InputEvent::MouseButton(event)
    }
}

impl From<MouseWheel> for InputEvent {
    fn from(event: MouseWheel) -> Self {
        InputEvent::MouseWheel(event)
    }
}

impl From<CursorMoved> for InputEvent {
    fn from(event: CursorMoved) -> Self {
        InputEvent::CursorMoved(event)
    }
}

/// Captures user inputs from the assorted raw `Event` types
///
/// These are collected into a [`UnifiedInput`] event stream.
pub struct InputCapturePlugin;

impl Plugin for InputCapturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameCount>()
            .add_event::<UnifiedInputEvent>()
            .add_system_to_stage(CoreStage::First, frame_counter)
            .add_system_set_to_stage(
                // Capture any mocked input as well
                CoreStage::Last,
                SystemSet::new()
                    .with_system(capture_mouse_input)
                    .with_system(capture_keyboard_input),
            );
    }
}

/// Increases the value of the [`FrameCount`] resource by 1 every frame
///
/// This system should run during [`CoreStage::First`].
pub fn frame_counter(mut frame_count: ResMut<FrameCount>) {
    frame_count.0 += 1;
}

/// Captures mouse-driven input from the [`MouseButtonInput`] event stream
///
/// Limitations:
///  - the unit of mouse scrolling is discarded; when played back this is assumed to be pixels
///  - mouse inputs performed with a locked window will be lost, as [`MouseMotion`](bevy::input::mouse::MouseMotion) events are not captured
///  - this is not robust to multiple windows; the window that the mouse is on is lost
pub fn capture_mouse_input(
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut unified_input: EventWriter<UnifiedInputEvent>,
    frame_count: Res<FrameCount>,
    time: Res<Time>,
) {
    let time_since_startup = time.time_since_startup();
    let frame = *frame_count;

    // BLOCKED: these events are arbitrarily ordered within a frame,
    // but we have no way to access their order from winit.
    // See https://github.com/bevyengine/bevy/issues/5984
    for input_event in mouse_button_events.iter().cloned() {
        unified_input.send(UnifiedInputEvent {
            frame,
            time_since_startup,
            input_event: input_event.into(),
        })
    }

    for input_event in mouse_wheel_events.iter().cloned() {
        unified_input.send(UnifiedInputEvent {
            frame,
            time_since_startup,
            input_event: input_event.into(),
        })
    }

    for input_event in cursor_moved_events.iter().cloned() {
        unified_input.send(UnifiedInputEvent {
            frame,
            time_since_startup,
            input_event: input_event.into(),
        })
    }
}

/// Captures [`KeyCode`](bevy_input::keyboard::KeyCode) input from the [`MouseButtonInput`] stream
pub fn capture_keyboard_input(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut unified_input: EventWriter<UnifiedInputEvent>,
    frame_count: Res<FrameCount>,
    time: Res<Time>,
) {
    let time_since_startup = time.time_since_startup();
    let frame = *frame_count;

    for input_event in keyboard_events.iter().cloned() {
        unified_input.send(UnifiedInputEvent {
            frame,
            time_since_startup,
            input_event: input_event.into(),
        })
    }
}

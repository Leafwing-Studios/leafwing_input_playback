//! Unifies (and time-stamp) various `bevy_input` and `bevy_window` input events.
//! These are first unified into a [`InputEvent`] enum, then timestamped to create a [`TimestampedInputEvent`].
//! Those timestamped events are finally stored inside of a [`UnifiedInput`] resource, which should be used for input capture and playback.

use bevy_input::keyboard::KeyboardInput;
use bevy_input::mouse::{MouseButtonInput, MouseWheel};
use bevy_utils::Duration;
use bevy_window::CursorMoved;

use crate::frame_counting::FrameCount;

/// A timestamped device-agnostic user-input event
///
/// These are re-emitted as events, and commonly serialized to disk
// BLOCKED: should be PartialEq, but https://github.com/bevyengine/bevy/issues/6024
#[derive(Debug, Clone)]
pub struct TimestampedInputEvent {
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
// BLOCKED: should be PartialEq, but https://github.com/bevyengine/bevy/issues/6024
#[derive(Debug, Clone, Default)]
pub struct UnifiedInput {
    /// The underlying [`UnifiedInputEvent`] data
    ///
    /// New events are pushed to the back of the list.
    /// If input events are recorded immediately, the final list will be sorted,
    /// with older events at the start of the [`Vec`].
    ///
    /// This type implements the [`Iterator`] trait;
    /// typically you will want to use that rather than accessing the internal event storage.
    pub events: Vec<TimestampedInputEvent>,
    /// The index in `events` of the next event to read
    ///
    /// When iterating over this struct, iterate one item at a time, beginning at `cursor + 1`.
    /// When you are done iterating, update this cursor as the last read index.
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
        self.events.push(TimestampedInputEvent {
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

    /// Gets the total length of the event stream
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Checks if the event stream is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Fetches the next available event (if any), and advances the internal `cursor` by one
    pub fn next(&mut self) -> Option<TimestampedInputEvent> {
        if self.cursor >= self.events.len() {
            None
        } else {
            self.cursor += 1;
            Some(self.events[self.cursor - 1].clone())
        }
    }

    /// Returns an iterator over all recorded events, beginning at the start of `events`.
    #[must_use]
    pub fn iter_all(&mut self) -> impl IntoIterator<Item = TimestampedInputEvent> {
        let iterator = self.events.clone();
        self.cursor = self.events.len();
        iterator
    }

    /// Returns an iterator over all recorded events, beginning at the current `cursor`.
    #[must_use]
    pub fn iter_rest(&mut self) -> impl IntoIterator<Item = TimestampedInputEvent> {
        let rest = self.events.split_off(self.cursor);
        self.cursor = self.events.len();
        rest
    }

    /// Returns an iterator over all recorded events up to and including the provided `frame` is reached, beginning at the current `cursor`.
    ///
    /// This method should only be used on [`UnifiedInput`] resources that are sorted by [`SortingStrategy::TimeSinceStartup`].
    #[must_use]
    pub fn iter_until_time(
        &mut self,
        time_since_startup: Duration,
    ) -> impl IntoIterator<Item = TimestampedInputEvent> {
        debug_assert!(self.is_sorted(SortingStrategy::TimeSinceStartup));
        let mut result = Vec::with_capacity(self.events.len() - self.cursor);
        while self.cursor < self.events.len()
            && self.events[self.cursor].time_since_startup <= time_since_startup
        {
            result.push(self.events[self.cursor].clone());
            self.cursor += 1;
        }
        result
    }

    /// Returns an iterator over all recorded events up to and including the provided `time_since_startup`, beginning at the current `cursor`
    ///
    /// This method should only be used on [`UnifiedInput`] resources that are sorted by [`SortingStrategy::FrameCount`].
    #[must_use]
    pub fn iter_until_frame(
        &mut self,
        frame: FrameCount,
    ) -> impl IntoIterator<Item = TimestampedInputEvent> {
        debug_assert!(self.is_sorted(SortingStrategy::TimeSinceStartup));
        let mut result = Vec::with_capacity(self.events.len() - self.cursor);
        while self.cursor < self.events.len()
            && self.events[self.cursor].frame <= frame
            && self.cursor < self.events.len()
        {
            result.push(self.events[self.cursor].clone());
            self.cursor += 1;
        }
        result
    }

    /// Returns an iterator over recorded events starting from (inclusive) the start time,
    /// and until (inclusive) the end time. Note that this can re-read events:
    /// the cursor is reset before searching for the start of the range.
    ///
    /// This method should only be used on [`UnifiedInput`] resources that are sorted by [`SortingStrategy::TimeSinceStartup`].
    #[must_use]
    pub fn iter_between_times(
        &mut self,
        start_time: Duration,
        end_time: Duration,
    ) -> impl IntoIterator<Item = TimestampedInputEvent> {
        debug_assert!(self.is_sorted(SortingStrategy::TimeSinceStartup));
        let mut result = Vec::with_capacity(self.events.len() - self.cursor);
        let mut cursor = 0;
        while self.cursor < self.events.len() && self.events[cursor].time_since_startup <= end_time
        {
            if self.events[cursor].time_since_startup >= start_time {
                result.push(self.events[cursor].clone());
            }
            cursor += 1;
        }
        self.cursor = cursor;
        result
    }

    /// Returns an iterator over recorded events starting from (inclusive) the start frame,
    /// and until (inclusive) the end frame. Note that this can re-read events:
    /// the cursor is reset before searching for the start of the range.
    ///
    /// This method should only be used on [`UnifiedInput`] resources that are sorted by [`SortingStrategy::TimeSinceStartup`].
    #[must_use]
    pub fn iter_between_frames(
        &mut self,
        start_frame: FrameCount,
        end_frame: FrameCount,
    ) -> impl IntoIterator<Item = TimestampedInputEvent> {
        debug_assert!(self.is_sorted(SortingStrategy::FrameCount));
        let mut result = Vec::with_capacity(self.events.len());
        let mut cursor = 0;
        while self.cursor < self.events.len() && self.events[cursor].frame <= end_frame {
            if self.events[cursor].frame >= start_frame {
                result.push(self.events[cursor].clone());
            }
            cursor += 1;
        }
        self.cursor = cursor;
        result
    }

    /// Sorts the input stream by either [`Time::time_since_startup`] or [`FrameCount`].
    pub fn sort(&mut self, strategy: SortingStrategy) {
        let strategy = match strategy {
            SortingStrategy::TimeSinceStartup => {
                |a: &TimestampedInputEvent, b: &TimestampedInputEvent| {
                    a.time_since_startup.cmp(&b.time_since_startup)
                }
            }
            SortingStrategy::FrameCount => {
                |a: &TimestampedInputEvent, b: &TimestampedInputEvent| a.frame.cmp(&b.frame)
            }
        };

        self.events.sort_by(strategy);
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

    /// The [`InputEvent] of the last-read event.
    pub fn last_input(&self) -> Option<InputEvent> {
        if self.cursor == 0 {
            return None;
        }

        let last_read = self.events.get(self.cursor - 1)?;
        Some(last_read.input_event.clone())
    }

    /// The frame count of the next event to read.
    pub fn current_input(&self) -> Option<InputEvent> {
        let next_read = self.events.get(self.cursor)?;
        Some(next_read.input_event.clone())
    }

    /// The frame count of the last-read event.
    pub fn last_framecount(&self) -> Option<FrameCount> {
        if self.cursor == 0 {
            return None;
        }

        let last_read = self.events.get(self.cursor - 1)?;
        Some(last_read.frame)
    }

    /// The frame count of the next event to read.
    pub fn current_framecount(&self) -> Option<FrameCount> {
        let next_read = self.events.get(self.cursor)?;
        Some(next_read.frame)
    }

    /// The time since startup of the last-read event.
    pub fn last_time(&self) -> Option<Duration> {
        if self.cursor == 0 {
            return None;
        }

        let last_read = self.events.get(self.cursor - 1)?;
        Some(last_read.time_since_startup)
    }

    /// The time since startup of the next event to read.
    pub fn current_time(&self) -> Option<Duration> {
        let next_read = self.events.get(self.cursor)?;
        Some(next_read.time_since_startup)
    }
}

/// The sorting strategy used for the [`UnifiedInput::sort`] method.
///
/// In all typical cases, these two sorting strategies should agree.
pub enum SortingStrategy {
    /// Sort by ascending frame count
    FrameCount,
    /// Sort by ascending time since startup
    TimeSinceStartup,
}

/// Collects input-relevant events for use in [`UnifiedInput`]
// BLOCKED: this should be PartialEq, but we're blocked on https://github.com/bevyengine/bevy/issues/6024
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_input::mouse::MouseButton;
    use bevy_input::ButtonState;

    const LEFT_CLICK_PRESS: InputEvent = InputEvent::MouseButton(MouseButtonInput {
        button: MouseButton::Left,
        state: ButtonState::Pressed,
    });

    const LEFT_CLICK_RELEASE: InputEvent = InputEvent::MouseButton(MouseButtonInput {
        button: MouseButton::Left,
        state: ButtonState::Released,
    });

    #[test]
    fn send_event() {
        let mut unified_input = UnifiedInput::default();
        unified_input.send(FrameCount(0), Duration::ZERO, LEFT_CLICK_PRESS);
        assert_eq!(unified_input.len(), 1);
        assert_eq!(unified_input.last_framecount(), None);
        assert_eq!(unified_input.current_framecount(), Some(FrameCount(0)));
        assert_eq!(unified_input.last_time(), None);
        assert_eq!(unified_input.current_time(), Some(Duration::ZERO));
    }

    #[test]
    fn send_multiple_events() {
        let mut unified_input = UnifiedInput::default();
        let events = [LEFT_CLICK_PRESS, LEFT_CLICK_RELEASE];

        // This sends all events received simultaneously
        unified_input.send_multiple(FrameCount(0), Duration::ZERO, events.into_iter());

        assert_eq!(unified_input.len(), 2);
        assert_eq!(unified_input.last_framecount(), None);
        assert_eq!(unified_input.current_framecount(), Some(FrameCount(0)));
        assert_eq!(unified_input.last_time(), None);
        assert_eq!(unified_input.current_time(), Some(Duration::ZERO));

        // Advance by one event
        unified_input.next();

        assert_eq!(unified_input.last_framecount(), Some(FrameCount(0)));
        assert_eq!(unified_input.current_framecount(), Some(FrameCount(0)));
        assert_eq!(unified_input.last_time(), Some(Duration::ZERO));
        assert_eq!(unified_input.current_time(), Some(Duration::ZERO));

        // BLOCKED: we want PartialEq on `InputEvent`, but https://github.com/bevyengine/bevy/issues/6024

        // assert_eq!(unified_input.last_input(), Some(LEFT_CLICK_PRESS));
        // assert_eq!(unified_input.current_input(), Some(LEFT_CLICK_RELEASE));
    }
}

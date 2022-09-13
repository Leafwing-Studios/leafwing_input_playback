//! Input playback data and associated datastructures

use bevy_input::gamepad::GamepadEventRaw;

use winit::event::WindowEvent;

/// A captured input event
#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackData {
    /// A window-related event emitted by [`winit`]
    Window(WindowEvent<'static>),
    /// A gamepad event emitted by `gilrs`
    Gamepad(GamepadEventRaw),
}

/// A vector of [`PlaybackData`], stored as a resource.
#[derive(Debug, Clone)]
pub struct PlaybackBuffer {
    /// The index of this vector represent the frame on which this data was captured
    pub data: Vec<PlaybackData>,
}

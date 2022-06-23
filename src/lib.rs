#![forbid(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

mod display_impl;
pub mod errors;
mod input_mocking;
// Re-export this at the root level for convenience
pub use input_mocking::{MockInput, RegisterGamepads};
pub mod axislike;
pub mod buttonlike;
pub mod input_capture;
pub mod orientation;
pub mod user_input;

/// Everything you need to get started
pub mod prelude {
    pub use crate::input_capture::{InputCapturePlugin, UnifiedInput};
    pub use crate::input_mocking::MockInput;
    pub use crate::user_input::UserInput;
}

#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

pub mod input_capture;
pub mod input_playback;
pub mod serde;
pub mod timestamped_input;

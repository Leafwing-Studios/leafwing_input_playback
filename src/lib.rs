#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

pub mod frame_counting;
pub mod input_capture;
pub mod input_playback;
pub mod unified_input;

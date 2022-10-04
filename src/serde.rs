//! Serialization and deserialization of [`TimestampedInputs`](crate::timestamped_input::TimestampedInputs) data
use std::path::PathBuf;

/// The file path where captured events will be saved to and read from.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PlaybackFilePath {
    /// The stored [`PathBuf`].
    ///
    /// If [`None`], inputs will not be saved to / played back from disk.
    pub path: Option<PathBuf>,
}

impl PlaybackFilePath {
    /// Creates a new [`PlaybackFilePath`] from a quoted string.
    pub fn new(path_str: &str) -> Self {
        PlaybackFilePath {
            path: Some(path_str.into()),
        }
    }
}

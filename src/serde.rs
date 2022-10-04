//! Serialization and deserialization of [`TimestampedInputs`](crate::timestamped_input::TimestampedInputs) data
use std::path::PathBuf;

/// The file path where captured events will be saved to and read from.
///
/// Currently, only `.ron` serialization / deserialization is supported.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PlaybackFilePath {
    /// The stored [`PathBuf`].
    ///
    ///
    /// If [`None`], inputs will not be saved to / played back from disk.
    path: Option<PathBuf>,
}

impl PlaybackFilePath {
    /// Creates a new [`PlaybackFilePath`] from a quoted string.
    ///
    /// # Panics
    ///
    /// Only `.ron` files are supported, and so this method will panic if a path with an incompatible extension is provided.
    pub fn new(path_str: &str) -> Self {
        let path: PathBuf = path_str.into();
        assert_eq!(path.extension().unwrap(), "ron");

        PlaybackFilePath { path: Some(path) }
    }

    /// Retrieves the stored [`PathBuf`].
    ///
    ///
    /// If [`None`], inputs will not be saved to / played back from disk.
    pub fn path(&self) -> &Option<PathBuf> {
        &self.path
    }

    /// Sets the stored [`PathBuf`].
    ///
    ///
    /// If [`None`], inputs will not be saved to / played back from disk.
    pub fn set_path(&mut self, path: Option<PathBuf>) {
        if let Some(actual_path) = &path {
            assert_eq!(actual_path.extension().unwrap(), "ron");
        }

        self.path = path;
    }
}

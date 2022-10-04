# About

An input recording and playback library for the [Bevy] game engine in Rust.
Test your games and applications without breaking a sweat.
Support Tool Assisted Speedruns (TAS) effortlessly.
Make crazy game mechanics where your player's inputs are echoed??

This crate currently captures the following input modes:

- keyboard
- mouse
- gamepad

If you need more, please feel free to file an issue or open a PR!

## Getting started

If you're new to this crate, check out the [`input_playback`](./examples/input_playback.rs) example to get a good overview of how it all works in practice.

## Playback-powered testing

This crate can be used to capture, save to disk, and then play back user inputs in a macro-like fashion.
This serialization-based workflow is a natural (if somewhat brittle) fit for testing UI, app logic and gameplay elements of your Bevy apps.

Simply toggle on `InputCapturePlugin`, set a `PlaybackFilePath`, perform your inputs and then close the app.

Then, when evaluating tests, loop through each saved input sample, and run your app with the `InputPlaybackPlugin`, providing a `PlaybackFilePath` corresponding to the input sample you are testing.
`AppExit` events are also captured, your tests will close down automatically when they're complete.

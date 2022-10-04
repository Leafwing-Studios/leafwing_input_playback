# About

An input recording, mocking and playback library for the [Bevy] game engine in Rust.
Test your games and applications without breaking a sweat.

This crate is designed to work smoothly with [`leafwing-input-manager`](https://crates.io/crates/leafwing-input-manager), a simple but expressive tool to map user inputs to in-game actions.

## Features

- Powerful and easy-to-use input mocking API for integration testing your Bevy applications
  - `app.send_input(KeyCode::B)` or `world.send_input(UserInput::chord([KeyCode::B, KeyCode::E, KeyCode::V, KeyCode::Y])`
- Leafwing Studio's trademark `#![deny(missing_docs)]`

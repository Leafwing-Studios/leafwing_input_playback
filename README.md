# About

A input recording, mocking and playback library for the [Bevy] game engine in Rust.

## Features

- Powerful and easy-to-use input mocking API for integration testing your Bevy applications
  - `app.send_input(KeyCode::B)` or `world.send_input(UserInput::chord([KeyCode::B, KeyCode::E, KeyCode::V, KeyCode::Y])`
- Leafwing Studio's trademark `#![forbid(missing_docs)]`

## Getting started

```rust
use leafwing_input_playback::prelude::*;

#[test]
fn mock_inputs(){
  let app = App::new();

  // It's never been easier to pay your respects
  app.send_input(KeyCode::F);
}
```

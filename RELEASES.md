# Release Notes

## Version 0.3

Migrated to `bevy 0.10`.

Note that `App::update` no longer sends an `AppExit` event: this may affect your tests!

## Version 0.1

### Enhancements

- shamelessly stole input mocking functionality from `leafwing_input_playback`
- added the `RegisterGamepads` trait for easy mocking of specific gamepad inputs

### Docs

- added basic examples of how to perform input mocking for buttonlike inputs

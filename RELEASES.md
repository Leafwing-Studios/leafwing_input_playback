# Release Notes

## Version 0.6 (Draft)

- migrated to Bevy 0.14
- replaced `FrameCount` with its Bevy counterpart and updated system scheduling constraints to match the new `FrameCount`'s behavior, which updates in `Last` rather than `First`

## Version 0.5

- migrated to Bevy 0.13

## Version 0.4

- migrated to Bevy 0.10`
- note that `App::update` no longer sends an `AppExit` event: this may affect your tests!

## Version 0.1

### Enhancements

- shamelessly stole input mocking functionality from `leafwing_input_playback`
- added the `RegisterGamepads` trait for easy mocking of specific gamepad inputs

### Docs

- added basic examples of how to perform input mocking for buttonlike inputs

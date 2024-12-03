use bevy::{color::palettes, prelude::*, window::PrimaryWindow};

use leafwing_input_playback::{
    input_capture::{BeginInputCapture, EndInputCapture, InputCapturePlugin},
    input_playback::{BeginInputPlayback, EndInputPlayback, InputPlaybackPlugin, PlaybackStrategy},
    timestamped_input::TimestampedInputs,
};

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, InputCapturePlugin, InputPlaybackPlugin))
        // Creates a little game that spawns decaying boxes where the player clicks
        .insert_resource(ClearColor(Color::srgb(0.9, 0.9, 0.9)))
        // Toggle between playback and capture by pressing Space
        .insert_resource(InputStrategy::Playback)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (spawn_boxes, decay_boxes, toggle_capture_vs_playback),
        );
    app.run()
}

#[derive(Resource, PartialEq)]
enum InputStrategy {
    Capture,
    Playback,
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

pub fn cursor_pos_as_world_pos(
    current_window: &Window,
    camera_query: &Query<(&GlobalTransform, &Camera)>,
) -> Option<Vec2> {
    let (camera_transform, camera) = camera_query.single();
    current_window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
}

#[derive(Component)]
struct Box;

fn spawn_boxes(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
) {
    const BOX_SCALE: f32 = 50.0;

    if mouse_input.pressed(MouseButton::Left) {
        let primary_window = windows.single();
        // Don't break if we leave the window
        if let Some(cursor_pos) = cursor_pos_as_world_pos(primary_window, &camera_query) {
            commands.spawn((
                Sprite {
                    color: Color::Srgba(palettes::css::DARK_GREEN),
                    ..default()
                },
                Transform {
                    translation: cursor_pos.extend(0.0),
                    scale: Vec3::splat(BOX_SCALE),
                    ..default()
                },
                Box,
            ));
        }
    }
}

fn decay_boxes(mut query: Query<(Entity, &mut Transform), With<Box>>, mut commands: Commands) {
    const MIN_SCALE: f32 = 1.;
    const SHRINK_FACTOR: f32 = 0.95;

    for (entity, mut transform) in query.iter_mut() {
        if transform.scale.x < MIN_SCALE {
            commands.entity(entity).despawn();
        } else {
            transform.scale *= SHRINK_FACTOR;
        }
    }
}

fn toggle_capture_vs_playback(
    mut commands: Commands,
    mut input_strategy: ResMut<InputStrategy>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    timestamped_input: Option<ResMut<TimestampedInputs>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        *input_strategy = match *input_strategy {
            InputStrategy::Capture => {
                // Disable input capture
                commands.trigger(EndInputCapture);
                // Enable input playback
                if let Some((start, end)) =
                    // Play back all recorded inputs at the same rate they were input
                    timestamped_input
                        .and_then(|timestamped_input| timestamped_input.frame_range())
                {
                    commands.trigger(BeginInputPlayback {
                        playback_strategy: PlaybackStrategy::FrameRangeOnce(start, end),
                        ..default()
                    });
                    info!("Now playing back input.");
                } else {
                    info!("No input to replay.");
                }

                InputStrategy::Playback
            }
            InputStrategy::Playback => {
                // Disable input playback, resetting all input data.
                commands.trigger(EndInputPlayback);
                // Enable input capture
                commands.trigger(BeginInputCapture::default());

                info!("Now capturing input.");
                InputStrategy::Capture
            }
        };
    }
}

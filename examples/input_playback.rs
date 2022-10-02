use bevy::prelude::*;

use leafwing_input_playback::{
    input_capture::{InputCapturePlugin, InputModesCaptured},
    input_playback::{InputPlaybackPlugin, PlaybackStrategy},
    unified_input::UnifiedInput,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(InputCapturePlugin)
        .add_plugin(InputPlaybackPlugin)
        // Disable all input capture and playback to start
        .insert_resource(InputModesCaptured::DISABLE_ALL)
        .insert_resource(PlaybackStrategy::Paused)
        // Creates a little game that spawns decaying boxes where the player clicks
        .insert_resource(ClearColor(Color::rgb(0.9, 0.9, 0.9)))
        .add_startup_system(setup)
        .add_system(spawn_boxes)
        .add_system(decay_boxes)
        // Toggle between playback and capture by pressing Space
        .insert_resource(InputStrategy::Playback)
        .add_system(toggle_capture_vs_playback)
        .run()
}

#[derive(PartialEq)]
enum InputStrategy {
    Capture,
    Playback,
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}

pub fn cursor_pos_as_world_pos(
    current_window: &Window,
    camera_query: &Query<(&Transform, &Camera)>,
) -> Option<Vec2> {
    current_window.cursor_position().map(|cursor_pos| {
        let (cam_t, cam) = camera_query.single();
        let window_size = Vec2::new(current_window.width(), current_window.height());

        // Convert screen position [0..resolution] to ndc [-1..1]
        let ndc_to_world = cam_t.compute_matrix() * cam.projection_matrix().inverse();
        let ndc = (Vec2::new(cursor_pos.x, cursor_pos.y) / window_size) * 2.0 - Vec2::ONE;
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
        world_pos.truncate()
    })
}

#[derive(Component)]
struct Box;

fn spawn_boxes(
    mut commands: Commands,
    windows: Res<Windows>,
    mouse_input: Res<Input<MouseButton>>,
    camera_query: Query<(&Transform, &Camera)>,
) {
    const BOX_SCALE: f32 = 50.0;

    if mouse_input.pressed(MouseButton::Left) {
        let primary_window = windows.primary();
        // Don't break if we leave the window
        if let Some(cursor_pos) = cursor_pos_as_world_pos(primary_window, &camera_query) {
            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::DARK_GREEN,
                        ..default()
                    },
                    transform: Transform {
                        translation: cursor_pos.extend(0.0),
                        scale: Vec3::splat(BOX_SCALE),
                        ..default()
                    },
                    ..default()
                })
                .insert(Box);
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
    mut input_modes: ResMut<InputModesCaptured>,
    mut playback_strategy: ResMut<PlaybackStrategy>,
    keyboard_input: Res<Input<KeyCode>>,
    mut unified_input: ResMut<UnifiedInput>,
    mut input_strategy: ResMut<InputStrategy>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        *input_strategy = match *input_strategy {
            InputStrategy::Capture => {
                // Disable input capture
                *input_modes = InputModesCaptured::DISABLE_ALL;
                // Enable input playback
                *playback_strategy = if let Some((start, end)) =
                    // Play back all recorded inputs at the same rate they were input
                    unified_input.frame_range()
                {
                    PlaybackStrategy::FrameRangeOnce(start, end)
                } else {
                    // Do not play back events if none were recorded
                    PlaybackStrategy::Paused
                };

                info!("Now playing back input.");
                InputStrategy::Playback
            }
            InputStrategy::Playback => {
                // Enable input capture
                *input_modes = InputModesCaptured::ENABLE_ALL;
                // Disable input playback
                *playback_strategy = PlaybackStrategy::Paused;

                // Reset all input data, starting a new recording
                *unified_input = UnifiedInput::default();

                info!("Now capturing input.");
                InputStrategy::Capture
            }
        };
    }
}

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
        // Toggle input_capture using Space
        .add_system(toggle_input_capture)
        // Toggle playback using Enter
        .add_system(toggle_input_playback)
        .run()
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

fn toggle_input_capture(
    mut input_modes: ResMut<InputModesCaptured>,
    keyboard_input: Res<Input<KeyCode>>,
    mut unified_input: ResMut<UnifiedInput>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        if !input_modes.mouse_motion {
            *input_modes = InputModesCaptured {
                mouse_buttons: true,
                mouse_motion: true,
                ..default()
            };

            // Reset all input data, starting a new recording
            *unified_input = UnifiedInput::default();

            info!("Input is now being captured.");
        } else {
            *input_modes = InputModesCaptured::DISABLE_ALL;
            info!("Input is no longer being captured.");
        }
    }
}

fn toggle_input_playback(
    keyboard_input: Res<Input<KeyCode>>,
    mut playback_strategy: ResMut<PlaybackStrategy>,
    unified_input: Res<UnifiedInput>,
) {
    if keyboard_input.just_pressed(KeyCode::Return) {
        *playback_strategy = match *playback_strategy {
            PlaybackStrategy::FrameRangeOnce(_, _) => {
                info!("Input playback is now paused.");
                PlaybackStrategy::Paused
            }
            PlaybackStrategy::Paused => {
                info!("Input is playing back.");
                if let Some(start) = unified_input.start_frame() {
                    if let Some(end) = unified_input.end_frame() {
                        PlaybackStrategy::FrameRangeOnce(start, end)
                    } else {
                        PlaybackStrategy::Paused
                    }
                } else {
                    PlaybackStrategy::Paused
                }
            }
            _ => unreachable!(),
        }
    }
}

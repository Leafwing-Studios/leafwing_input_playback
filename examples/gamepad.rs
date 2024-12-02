//! Demonstrates input capture and playback of gamepad inputs
//!
//! This example is modified from https://github.com/bevyengine/bevy/blob/main/examples/tools/gamepad_viewer.rs,
//! which is used here under the MIT License <3

//! Shows a visualization of gamepad buttons, sticks, and triggers

use bevy::prelude::*;
use leafwing_input_playback::{
    input_capture::{BeginInputCapture, EndInputCapture, InputCapturePlugin},
    input_playback::{BeginInputPlayback, EndInputPlayback, InputPlaybackPlugin, PlaybackStrategy},
    timestamped_input::TimestampedInputs,
};

fn main() {
    use gamepad_viewer_example::*;

    let mut app = App::new();
    app.add_plugins((
        // This plugin contains all the code from the original example
        GamepadViewerExample,
        InputCapturePlugin,
        InputPlaybackPlugin,
    ))
    // Toggle between playback and capture using Space
    .insert_resource(InputStrategy::Playback)
    .add_systems(Update, toggle_capture_vs_playback);

    app.run();
}

#[derive(Resource, PartialEq)]
enum InputStrategy {
    Capture,
    Playback,
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
                commands.trigger(BeginInputCapture {
                    filepath: Some("./data/hello_world.ron".to_string()),
                    ..default()
                });

                info!("Now capturing input.");
                InputStrategy::Capture
            }
        };
    }
}

mod gamepad_viewer_example {
    /// This is the main function from the example adapted from
    /// https://github.com/bevyengine/bevy/blob/main/examples/tools/gamepad_viewer.rs
    pub struct GamepadViewerExample;

    impl Plugin for GamepadViewerExample {
        fn build(&self, app: &mut App) {
            app.add_plugins(DefaultPlugins)
                .init_resource::<ButtonMaterials>()
                .init_resource::<ButtonMeshes>()
                .init_resource::<FontHandle>()
                .add_systems(
                    Startup,
                    (setup, setup_sticks, setup_triggers, setup_connected),
                )
                .add_systems(
                    Update,
                    (
                        update_buttons,
                        update_button_values,
                        update_axes,
                        update_connected,
                    ),
                );
        }
    }

    use std::f32::consts::PI;

    use bevy::{
        color::palettes,
        input::gamepad::{GamepadButton, GamepadButtonChangedEvent, GamepadEvent, GamepadSettings},
        prelude::*,
    };

    const BUTTON_RADIUS: f32 = 25.;
    const BUTTON_CLUSTER_RADIUS: f32 = 50.;
    const START_SIZE: Vec2 = Vec2::new(30., 15.);
    const TRIGGER_SIZE: Vec2 = Vec2::new(70., 20.);
    const STICK_BOUNDS_SIZE: f32 = 100.;

    const BUTTONS_X: f32 = 150.;
    const BUTTONS_Y: f32 = 80.;
    const STICKS_X: f32 = 150.;
    const STICKS_Y: f32 = -135.;

    const NORMAL_BUTTON_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);
    const ACTIVE_BUTTON_COLOR: Color = Color::Srgba(palettes::css::PURPLE);
    const LIVE_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);
    const DEAD_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);
    const EXTENT_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);
    const TEXT_COLOR: TextColor = TextColor(Color::WHITE);

    #[derive(Resource)]
    struct DisplayGamepad(Entity);
    #[derive(Component, Deref)]
    struct ReactTo(GamepadButton);
    #[derive(Clone, Copy, Component)]
    enum GamepadStick {
        Left,
        Right,
    }
    #[derive(Component)]
    struct ButtonScale(f32);

    #[derive(Component, Deref)]
    struct TextWithButtonValue(GamepadButton);

    #[derive(Component)]
    struct ConnectedGamepadsText;

    #[derive(Resource)]
    struct ButtonMaterials {
        normal: Handle<ColorMaterial>,
        active: Handle<ColorMaterial>,
    }

    impl FromWorld for ButtonMaterials {
        fn from_world(world: &mut World) -> Self {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
            Self {
                normal: materials.add(ColorMaterial::from(NORMAL_BUTTON_COLOR)),
                active: materials.add(ColorMaterial::from(ACTIVE_BUTTON_COLOR)),
            }
        }
    }
    #[derive(Resource)]
    struct ButtonMeshes {
        circle: Handle<Mesh>,
        triangle: Handle<Mesh>,
        start_pause: Handle<Mesh>,
        trigger: Handle<Mesh>,
    }

    impl FromWorld for ButtonMeshes {
        fn from_world(world: &mut World) -> Self {
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            Self {
                circle: meshes.add(Circle::new(BUTTON_RADIUS).mesh()).into(),
                triangle: meshes
                    .add(RegularPolygon::new(BUTTON_RADIUS, 3).mesh())
                    .into(),
                start_pause: meshes
                    .add(Rectangle::new(START_SIZE.x, START_SIZE.y).mesh())
                    .into(),
                trigger: meshes
                    .add(Rectangle::new(TRIGGER_SIZE.x, TRIGGER_SIZE.y).mesh())
                    .into(),
            }
        }
    }
    #[derive(Resource, Deref)]
    struct FontHandle(Handle<Font>);
    impl FromWorld for FontHandle {
        fn from_world(world: &mut World) -> Self {
            let asset_server = world.resource::<AssetServer>();
            Self(asset_server.load("fonts/FiraSans-Bold.ttf"))
        }
    }

    fn setup(mut commands: Commands, meshes: Res<ButtonMeshes>, materials: Res<ButtonMaterials>) {
        commands.spawn(Camera2d);

        // Buttons

        commands
            .spawn((
                Transform::from_xyz(BUTTONS_X, BUTTONS_Y, 0.),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent
                    .spawn((
                        Mesh2d(meshes.circle.clone()),
                        MeshMaterial2d(materials.normal.clone()),
                        Transform::from_xyz(0., BUTTON_CLUSTER_RADIUS, 0.),
                    ))
                    .insert(ReactTo(GamepadButton::North));
                parent
                    .spawn((
                        Mesh2d(meshes.circle.clone()),
                        MeshMaterial2d(materials.normal.clone()),
                        Transform::from_xyz(0., -BUTTON_CLUSTER_RADIUS, 0.),
                    ))
                    .insert(ReactTo(GamepadButton::South));
                parent
                    .spawn((
                        Mesh2d(meshes.circle.clone()),
                        MeshMaterial2d(materials.normal.clone()),
                        Transform::from_xyz(-BUTTON_CLUSTER_RADIUS, 0., 0.),
                    ))
                    .insert(ReactTo(GamepadButton::West));
                parent
                    .spawn((
                        Mesh2d(meshes.circle.clone()),
                        MeshMaterial2d(materials.normal.clone()),
                        Transform::from_xyz(BUTTON_CLUSTER_RADIUS, 0., 0.),
                    ))
                    .insert(ReactTo(GamepadButton::East));
            });

        // Start and Pause

        commands
            .spawn((
                Mesh2d(meshes.start_pause.clone()),
                MeshMaterial2d(materials.normal.clone()),
                Transform::from_xyz(-30., BUTTONS_Y, 0.),
            ))
            .insert(ReactTo(GamepadButton::Select));

        commands
            .spawn((
                Mesh2d(meshes.start_pause.clone()),
                MeshMaterial2d(materials.normal.clone()),
                Transform::from_xyz(30., BUTTONS_Y, 0.),
            ))
            .insert(ReactTo(GamepadButton::Start));

        // D-Pad

        commands
            .spawn((
                Transform::from_xyz(-BUTTONS_X, BUTTONS_Y, 0.),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent
                    .spawn((
                        Mesh2d(meshes.triangle.clone()),
                        MeshMaterial2d(materials.normal.clone()),
                        Transform::from_xyz(0., BUTTON_CLUSTER_RADIUS, 0.),
                    ))
                    .insert(ReactTo(GamepadButton::DPadUp));
                parent
                    .spawn((
                        Mesh2d(meshes.triangle.clone()),
                        MeshMaterial2d(materials.normal.clone()),
                        Transform::from_xyz(0., -BUTTON_CLUSTER_RADIUS, 0.)
                            .with_rotation(Quat::from_rotation_z(PI)),
                    ))
                    .insert(ReactTo(GamepadButton::DPadDown));
                parent
                    .spawn((
                        Mesh2d(meshes.triangle.clone()),
                        MeshMaterial2d(materials.normal.clone()),
                        Transform::from_xyz(-BUTTON_CLUSTER_RADIUS, 0., 0.)
                            .with_rotation(Quat::from_rotation_z(PI / 2.)),
                    ))
                    .insert(ReactTo(GamepadButton::DPadLeft));
                parent
                    .spawn((
                        Mesh2d(meshes.triangle.clone()),
                        MeshMaterial2d(materials.normal.clone()),
                        Transform::from_xyz(BUTTON_CLUSTER_RADIUS, 0., 0.)
                            .with_rotation(Quat::from_rotation_z(-PI / 2.)),
                    ))
                    .insert(ReactTo(GamepadButton::DPadRight));
            });

        // Triggers

        commands
            .spawn((
                Mesh2d(meshes.trigger.clone()),
                MeshMaterial2d(materials.normal.clone()),
                Transform::from_xyz(-BUTTONS_X, BUTTONS_Y + 115., 0.),
            ))
            .insert(ReactTo(GamepadButton::LeftTrigger));

        commands
            .spawn((
                Mesh2d(meshes.trigger.clone()),
                MeshMaterial2d(materials.normal.clone()),
                Transform::from_xyz(BUTTONS_X, BUTTONS_Y + 115., 0.),
            ))
            .insert(ReactTo(GamepadButton::RightTrigger));
    }

    fn setup_sticks(
        mut commands: Commands,
        meshes: Res<ButtonMeshes>,
        materials: Res<ButtonMaterials>,
        gamepad_settings: Query<&GamepadSettings>,
        font: Res<FontHandle>,
    ) {
        let Ok(gamepad_settings) = gamepad_settings.get_single() else {
            eprintln!("NOOOO");
            return;
        };
        let dead_upper =
            STICK_BOUNDS_SIZE * gamepad_settings.default_axis_settings.deadzone_upperbound();
        let dead_lower =
            STICK_BOUNDS_SIZE * gamepad_settings.default_axis_settings.deadzone_lowerbound();
        let dead_size = dead_lower.abs() + dead_upper.abs();
        let dead_mid = (dead_lower + dead_upper) / 2.0;

        let live_upper =
            STICK_BOUNDS_SIZE * gamepad_settings.default_axis_settings.livezone_upperbound();
        let live_lower =
            STICK_BOUNDS_SIZE * gamepad_settings.default_axis_settings.livezone_lowerbound();
        let live_size = live_lower.abs() + live_upper.abs();
        let live_mid = (live_lower + live_upper) / 2.0;

        let mut spawn_stick = |x_pos, y_pos, stick, button| {
            commands
                .spawn((Transform::from_xyz(x_pos, y_pos, 0.), Visibility::default()))
                .with_children(|parent| {
                    // full extent
                    parent.spawn((Sprite {
                        custom_size: Some(Vec2::splat(STICK_BOUNDS_SIZE * 2.)),
                        color: EXTENT_COLOR,
                        ..default()
                    },));
                    // live zone
                    parent.spawn((
                        Transform::from_xyz(live_mid, live_mid, 2.),
                        Sprite {
                            custom_size: Some(Vec2::new(live_size, live_size)),
                            color: LIVE_COLOR,
                            ..default()
                        },
                    ));
                    // dead zone
                    parent.spawn((
                        Transform::from_xyz(dead_mid, dead_mid, 3.),
                        Sprite {
                            custom_size: Some(Vec2::new(dead_size, dead_size)),
                            color: DEAD_COLOR,
                            ..default()
                        },
                    ));
                    // text
                    let font = TextFont {
                        font_size: 16.,
                        font: font.clone(),
                        ..default()
                    };
                    parent
                        .spawn((
                            Transform::from_xyz(0., STICK_BOUNDS_SIZE + 2., 4.),
                            Text2d::new(""),
                            font,
                            stick,
                        ))
                        .with_child(Text2d::new(format!("{:.3}", 0.)))
                        .with_child(Text2d::new(", ".to_string()))
                        .with_child(Text2d::new(format!("{:.3}", 0.)));
                    // cursor
                    parent.spawn((
                        Mesh2d(meshes.circle.clone()),
                        MeshMaterial2d(materials.normal.clone()),
                        Transform::from_xyz(0., 0., 5.).with_scale(Vec2::splat(0.2).extend(1.)),
                        stick,
                        ButtonScale(STICK_BOUNDS_SIZE),
                        ReactTo(button),
                    ));
                });
        };

        spawn_stick(
            -STICKS_X,
            STICKS_Y,
            GamepadStick::Left,
            GamepadButton::LeftThumb,
        );
        spawn_stick(
            STICKS_X,
            STICKS_Y,
            GamepadStick::Right,
            GamepadButton::RightThumb,
        );
    }

    fn setup_triggers(
        mut commands: Commands,
        meshes: Res<ButtonMeshes>,
        materials: Res<ButtonMaterials>,
        font: Res<FontHandle>,
    ) {
        let mut spawn_trigger = |x, y, button_type| {
            commands
                .spawn((
                    Mesh2d(meshes.trigger.clone()),
                    MeshMaterial2d(materials.normal.clone()),
                    Transform::from_xyz(x, y, 0.),
                ))
                .insert(ReactTo(button_type))
                .with_children(|parent| {
                    parent
                        .spawn((
                            Transform::from_xyz(0., 0., 1.),
                            Text2d::new(format!("{:.3}", 0.)),
                            TextFont {
                                font: font.clone(),
                                font_size: 16.,
                                ..default()
                            },
                            TEXT_COLOR,
                        ))
                        .insert(TextWithButtonValue(button_type));
                });
        };

        spawn_trigger(-BUTTONS_X, BUTTONS_Y + 145., GamepadButton::LeftTrigger2);
        spawn_trigger(BUTTONS_X, BUTTONS_Y + 145., GamepadButton::RightTrigger2);
    }

    fn setup_connected(mut commands: Commands, font: Res<FontHandle>) {
        let font = TextFont {
            font_size: 30.,
            font: font.clone(),
            ..default()
        };
        commands
            .spawn((
                Text2d::new("Connected Gamepads\n".to_string()),
                font.clone(),
            ))
            .insert(ConnectedGamepadsText)
            .with_child(Text2d::new("None"));
    }

    fn update_buttons(
        gamepads: Query<&Gamepad>,
        materials: Res<ButtonMaterials>,
        mut query: Query<(&mut MeshMaterial2d<ColorMaterial>, &ReactTo)>,
    ) {
        for gamepad in gamepads.iter() {
            for (mut handle, react_to) in query.iter_mut() {
                if gamepad.just_pressed(**react_to) {
                    handle.0 = materials.active.clone();
                }
                if gamepad.just_released(**react_to) {
                    handle.0 = materials.normal.clone();
                }
            }
        }
    }

    fn update_button_values(
        mut events: EventReader<GamepadEvent>,
        mut query: Query<(&mut Text2d, &TextWithButtonValue)>,
    ) {
        for event in events.read() {
            if let GamepadEvent::Button(GamepadButtonChangedEvent { button, value, .. }) = event {
                for (mut text, text_with_button_value) in query.iter_mut() {
                    if *button == **text_with_button_value {
                        text.0 = format!("{:3}", value);
                    }
                }
            }
        }
    }

    fn update_axes(
        gamepads: Query<&Gamepad>,
        mut ui_query: Query<(&mut Transform, &GamepadStick, &ButtonScale), Without<Text2d>>,
        text_query: Query<(Entity, &GamepadStick), With<Text2d>>,
        mut text_writer: Text2dWriter,
    ) {
        for gamepad in gamepads.iter() {
            let left_stick = gamepad.left_stick();
            let right_stick = gamepad.right_stick();
            for (mut transform, stick, scale) in ui_query.iter_mut() {
                let stick_pos = match stick {
                    GamepadStick::Left => left_stick,
                    GamepadStick::Right => right_stick,
                };
                transform.translation.x = stick_pos.x * scale.0;
                transform.translation.y = stick_pos.y * scale.0;
            }
            for (text_root, stick) in text_query.iter() {
                let stick_pos = match stick {
                    GamepadStick::Left => left_stick,
                    GamepadStick::Right => right_stick,
                };
                let index = match stick {
                    GamepadStick::Left => 1,
                    GamepadStick::Right => 3,
                };
                let mut text = text_writer.text(text_root, index);
                *text = format!("{:3}", stick_pos.x);
            }
        }
    }

    fn update_connected(
        gamepads: Query<&Gamepad>,
        mut query: Query<&mut Text2d, With<ConnectedGamepadsText>>,
    ) {
        let mut text = query.single_mut();

        let formatted = gamepads
            .iter()
            .map(|g| format!("{:?}", g))
            .collect::<Vec<_>>()
            .join("\n");

        text.0 = if !formatted.is_empty() {
            formatted
        } else {
            "None".to_string()
        }
    }
}

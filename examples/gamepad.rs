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
                    PreUpdate,
                    (setup, setup_sticks, setup_triggers, setup_connected)
                        .run_if(any_with_component::<Gamepad>.and(run_once))
                        .after(InputSystem),
                )
                .add_systems(
                    Update,
                    (
                        update_buttons,
                        update_button_values,
                        update_axes,
                        update_connected.run_if(any_with_component::<ConnectedGamepadsText>),
                    )
                        .run_if(any_with_component::<Gamepad>),
                );
        }
    }

    use std::f32::consts::PI;

    use bevy::{
        color::palettes,
        input::{
            gamepad::{GamepadButton, GamepadButtonChangedEvent, GamepadEvent, GamepadSettings},
            InputSystem,
        },
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

    #[derive(Component, Deref)]
    struct ReactTo(GamepadButton);
    #[derive(Clone, Copy, Component)]
    enum GamepadStick {
        Left,
        Right,
    }
    #[derive(Component)]
    struct GamepadLabel(Entity);
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
        gamepad_settings: Single<&GamepadSettings>,
        font: Res<FontHandle>,
    ) {
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
                    let layout = TextLayout::new(JustifyText::Justified, LineBreak::NoWrap);
                    parent.spawn((
                        Transform::from_xyz(0., STICK_BOUNDS_SIZE + 2., 4.),
                        Text2d::new(format!("{:.3}, {:.3}", 0., 0.)),
                        font,
                        layout,
                        stick,
                    ));
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
                ConnectedGamepadsText,
                Text::new("Connected Gamepads:".to_string()),
                font.clone(),
                TextLayout::new(JustifyText::Left, LineBreak::WordBoundary),
                Node {
                    justify_content: JustifyContent::FlexStart,
                    flex_direction: FlexDirection::Column,
                    top: Val::Percent(2.),
                    left: Val::Percent(2.),
                    width: Val::Px(350.),
                    overflow: Overflow::clip_x(),
                    row_gap: Val::Px(20.),
                    padding: UiRect::top(Val::Px(40.)),
                    ..Default::default()
                },
            ))
            .with_child(Text::new("None"));
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
        mut text_query: Query<(&mut Text2d, &GamepadStick)>,
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
            for (mut text, stick) in text_query.iter_mut() {
                let stick_pos = match stick {
                    GamepadStick::Left => left_stick,
                    GamepadStick::Right => right_stick,
                };
                text.0 = format!("{:.3}, {:.3}", stick_pos.x, stick_pos.y);
            }
        }
    }

    fn update_connected(
        mut commands: Commands,
        gamepads: Query<(Entity, Ref<Gamepad>)>,
        gamepads_text: Single<Entity, With<ConnectedGamepadsText>>,
        labels: Query<(Entity, &GamepadLabel)>,
        mut removed_gamepads: RemovedComponents<Gamepad>,
        mut last_count: Local<usize>,
    ) {
        // if no gamepads exist, remove all text children and add "None"
        if *last_count != 0 && gamepads.iter().len() == 0 {
            commands
                .entity(*gamepads_text)
                .despawn_descendants()
                .with_child(Text::new("None"));
        }
        *last_count = gamepads.iter().len();

        // if some gamepads have been removed/deleted, remove their corresponding label
        for removed_gamepad in removed_gamepads.read() {
            if let Some((label_entity, _)) =
                labels.iter().find(|(_, label)| label.0 == removed_gamepad)
            {
                commands.entity(label_entity).despawn();
            }
        }

        // if no other gamepads have changed, keep everything as-is
        if !gamepads.iter().any(|(_, gamepad)| gamepad.is_changed()) {
            return;
        }

        // otherwise, respawn the whole list
        let gamepad_labels = gamepads
            .iter()
            .map(|(entity, gamepad)| {
                (
                    Text::new(format!(
                        "Gamepad {:?} (Product ID: {})\n",
                        entity,
                        gamepad
                            .product_id()
                            .map(|id| id.to_string())
                            .unwrap_or("None".to_string())
                    )),
                    GamepadLabel(entity),
                )
            })
            .collect::<Vec<_>>();

        commands
            .entity(*gamepads_text)
            .despawn_descendants()
            .with_children(|builder| {
                for bundle in gamepad_labels {
                    builder.spawn(bundle);
                }
            });
    }
}

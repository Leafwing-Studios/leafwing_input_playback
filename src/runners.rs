//! [Custom app runners](https://github.com/bevyengine/bevy/blob/main/examples/app/custom_loop.rs) for Bevy, used to gain access to the raw [`WindowEvent`] data sent by [`winit`]

use bevy_app::App;
use winit_runner_reproduction::core_winit_logic;

/// A custom [`winit`]-based [`App`] runner used to playback user input
pub fn capture_runner(app: App) {
    core_winit_logic(app);
}

/// A custom [`winit`]-based [`App`] runner used to capture user input
pub fn playback_runner(app: App) {
    core_winit_logic(app);
}

/// All code in this module is minimally altered from `bevy_winit` under the MIT License
/// Track https://github.com/bevyengine/bevy/issues/5977 for adding this functionality upstream
/// and https://github.com/bevyengine/bevy/issues/4537 for making this code less nightmarish
mod winit_runner_reproduction {
    use bevy_app::{App, AppExit};
    use bevy_ecs::{
        event::{Events, ManualEventReader},
        world::World,
    };
    use bevy_input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
        touch::TouchInput,
    };
    use bevy_math::{ivec2, DVec2};
    use bevy_utils::{
        tracing::{info, trace, warn},
        Instant,
    };
    use bevy_window::{
        CreateWindow, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, ReceivedCharacter,
        RequestRedraw, WindowBackendScaleFactorChanged, WindowCloseRequested, WindowCreated,
        WindowFocused, WindowMoved, WindowResized, WindowScaleFactorChanged, Windows,
    };

    use winit::{
        event::{self, DeviceEvent, Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    };

    use super::converters::*;
    use bevy_winit::{UpdateMode, WinitSettings, WinitWindows};

    /// Stores state that must persist between frames.
    struct WinitPersistentState {
        /// Tracks whether or not the application is active or suspended.
        active: bool,
        /// Tracks whether or not an event has occurred this frame that would trigger an update in low
        /// power mode. Should be reset at the end of every frame.
        low_power_event: bool,
        /// Tracks whether the event loop was started this frame because of a redraw request.
        redraw_request_sent: bool,
        /// Tracks if the event loop was started this frame because of a `WaitUntil` timeout.
        timeout_reached: bool,
        last_update: Instant,
    }
    impl Default for WinitPersistentState {
        fn default() -> Self {
            Self {
                active: true,
                low_power_event: false,
                redraw_request_sent: false,
                timeout_reached: false,
                last_update: Instant::now(),
            }
        }
    }

    #[derive(Default)]
    struct WinitCreateWindowReader(ManualEventReader<CreateWindow>);

    pub(super) fn core_winit_logic(mut app: App) {
        let mut event_loop = app
            .world
            .remove_non_send_resource::<EventLoop<()>>()
            .unwrap();
        let mut create_window_event_reader = app
            .world
            .remove_resource::<WinitCreateWindowReader>()
            .unwrap()
            .0;
        let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
        let mut redraw_event_reader = ManualEventReader::<RequestRedraw>::default();
        let mut winit_state = WinitPersistentState::default();
        app.world
            .insert_non_send_resource(event_loop.create_proxy());

        let return_from_run = app.world.resource::<WinitSettings>().return_from_run;

        trace!("Entering winit event loop");

        let event_handler = move |event: Event<()>,
                                  event_loop: &EventLoopWindowTarget<()>,
                                  control_flow: &mut ControlFlow| {
            match event {
                event::Event::NewEvents(start) => {
                    let winit_config = app.world.resource::<WinitSettings>();
                    let windows = app.world.resource::<Windows>();
                    let focused = windows.iter().any(|w| w.is_focused());
                    // Check if either the `WaitUntil` timeout was triggered by winit, or that same
                    // amount of time has elapsed since the last app update. This manual check is needed
                    // because we don't know if the criteria for an app update were met until the end of
                    // the frame.
                    let auto_timeout_reached =
                        matches!(start, StartCause::ResumeTimeReached { .. });
                    let now = Instant::now();
                    let manual_timeout_reached = match winit_config.update_mode(focused) {
                        UpdateMode::Continuous => false,
                        UpdateMode::Reactive { max_wait }
                        | UpdateMode::ReactiveLowPower { max_wait } => {
                            now.duration_since(winit_state.last_update) >= *max_wait
                        }
                    };
                    // The low_power_event state and timeout must be reset at the start of every frame.
                    winit_state.low_power_event = false;
                    winit_state.timeout_reached = auto_timeout_reached || manual_timeout_reached;
                }
                event::Event::WindowEvent {
                    event,
                    window_id: winit_window_id,
                    ..
                } => {
                    let world = app.world.cell();
                    let winit_windows = world.non_send_resource_mut::<WinitWindows>();
                    let mut windows = world.resource_mut::<Windows>();
                    let window_id =
                        if let Some(window_id) = winit_windows.get_window_id(winit_window_id) {
                            window_id
                        } else {
                            warn!(
                                "Skipped event for unknown winit Window Id {:?}",
                                winit_window_id
                            );
                            return;
                        };

                    let window = if let Some(window) = windows.get_mut(window_id) {
                        window
                    } else {
                        // If we're here, this window was previously opened
                        info!("Skipped event for closed window: {:?}", window_id);
                        return;
                    };
                    winit_state.low_power_event = true;

                    match event {
                        WindowEvent::Resized(size) => {
                            window.update_actual_size_from_backend(size.width, size.height);
                            let mut resize_events = world.resource_mut::<Events<WindowResized>>();
                            resize_events.send(WindowResized {
                                id: window_id,
                                width: window.width(),
                                height: window.height(),
                            });
                        }
                        WindowEvent::CloseRequested => {
                            let mut window_close_requested_events =
                                world.resource_mut::<Events<WindowCloseRequested>>();
                            window_close_requested_events
                                .send(WindowCloseRequested { id: window_id });
                        }
                        WindowEvent::KeyboardInput { ref input, .. } => {
                            let mut keyboard_input_events =
                                world.resource_mut::<Events<KeyboardInput>>();
                            keyboard_input_events.send(convert_keyboard_input(input));
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            let mut cursor_moved_events =
                                world.resource_mut::<Events<CursorMoved>>();
                            let winit_window = winit_windows.get_window(window_id).unwrap();
                            let inner_size = winit_window.inner_size();

                            // move origin to bottom left
                            let y_position = inner_size.height as f64 - position.y;

                            let physical_position = DVec2::new(position.x, y_position);
                            window.update_cursor_physical_position_from_backend(Some(
                                physical_position,
                            ));

                            cursor_moved_events.send(CursorMoved {
                                id: window_id,
                                position: (physical_position / window.scale_factor()).as_vec2(),
                            });
                        }
                        WindowEvent::CursorEntered { .. } => {
                            let mut cursor_entered_events =
                                world.resource_mut::<Events<CursorEntered>>();
                            cursor_entered_events.send(CursorEntered { id: window_id });
                        }
                        WindowEvent::CursorLeft { .. } => {
                            let mut cursor_left_events = world.resource_mut::<Events<CursorLeft>>();
                            window.update_cursor_physical_position_from_backend(None);
                            cursor_left_events.send(CursorLeft { id: window_id });
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            let mut mouse_button_input_events =
                                world.resource_mut::<Events<MouseButtonInput>>();
                            mouse_button_input_events.send(MouseButtonInput {
                                button: convert_mouse_button(button),
                                state: convert_element_state(state),
                            });
                        }
                        WindowEvent::MouseWheel { delta, .. } => match delta {
                            event::MouseScrollDelta::LineDelta(x, y) => {
                                let mut mouse_wheel_input_events =
                                    world.resource_mut::<Events<MouseWheel>>();
                                mouse_wheel_input_events.send(MouseWheel {
                                    unit: MouseScrollUnit::Line,
                                    x,
                                    y,
                                });
                            }
                            event::MouseScrollDelta::PixelDelta(p) => {
                                let mut mouse_wheel_input_events =
                                    world.resource_mut::<Events<MouseWheel>>();
                                mouse_wheel_input_events.send(MouseWheel {
                                    unit: MouseScrollUnit::Pixel,
                                    x: p.x as f32,
                                    y: p.y as f32,
                                });
                            }
                        },
                        WindowEvent::Touch(touch) => {
                            let mut touch_input_events = world.resource_mut::<Events<TouchInput>>();

                            let mut location = touch.location.to_logical(window.scale_factor());

                            // On a mobile window, the start is from the top while on PC/Linux/OSX from
                            // bottom
                            if cfg!(target_os = "android") || cfg!(target_os = "ios") {
                                let window_height = windows.primary().height();
                                location.y = window_height - location.y;
                            }
                            touch_input_events.send(convert_touch_input(touch, location));
                        }
                        WindowEvent::ReceivedCharacter(c) => {
                            let mut char_input_events =
                                world.resource_mut::<Events<ReceivedCharacter>>();

                            char_input_events.send(ReceivedCharacter {
                                id: window_id,
                                char: c,
                            });
                        }
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                        } => {
                            let mut backend_scale_factor_change_events =
                                world.resource_mut::<Events<WindowBackendScaleFactorChanged>>();
                            backend_scale_factor_change_events.send(
                                WindowBackendScaleFactorChanged {
                                    id: window_id,
                                    scale_factor,
                                },
                            );
                            let prior_factor = window.scale_factor();
                            window.update_scale_factor_from_backend(scale_factor);
                            let new_factor = window.scale_factor();
                            if let Some(forced_factor) = window.scale_factor_override() {
                                // If there is a scale factor override, then force that to be used
                                // Otherwise, use the OS suggested size
                                // We have already told the OS about our resize constraints, so
                                // the new_inner_size should take those into account
                                *new_inner_size = winit::dpi::LogicalSize::new(
                                    window.requested_width(),
                                    window.requested_height(),
                                )
                                .to_physical::<u32>(forced_factor);
                            } else if approx::relative_ne!(new_factor, prior_factor) {
                                let mut scale_factor_change_events =
                                    world.resource_mut::<Events<WindowScaleFactorChanged>>();

                                scale_factor_change_events.send(WindowScaleFactorChanged {
                                    id: window_id,
                                    scale_factor,
                                });
                            }

                            let new_logical_width = new_inner_size.width as f64 / new_factor;
                            let new_logical_height = new_inner_size.height as f64 / new_factor;
                            if approx::relative_ne!(window.width() as f64, new_logical_width)
                                || approx::relative_ne!(window.height() as f64, new_logical_height)
                            {
                                let mut resize_events =
                                    world.resource_mut::<Events<WindowResized>>();
                                resize_events.send(WindowResized {
                                    id: window_id,
                                    width: new_logical_width as f32,
                                    height: new_logical_height as f32,
                                });
                            }
                            window.update_actual_size_from_backend(
                                new_inner_size.width,
                                new_inner_size.height,
                            );
                        }
                        WindowEvent::Focused(focused) => {
                            window.update_focused_status_from_backend(focused);
                            let mut focused_events = world.resource_mut::<Events<WindowFocused>>();
                            focused_events.send(WindowFocused {
                                id: window_id,
                                focused,
                            });
                        }
                        WindowEvent::DroppedFile(path_buf) => {
                            let mut events = world.resource_mut::<Events<FileDragAndDrop>>();
                            events.send(FileDragAndDrop::DroppedFile {
                                id: window_id,
                                path_buf,
                            });
                        }
                        WindowEvent::HoveredFile(path_buf) => {
                            let mut events = world.resource_mut::<Events<FileDragAndDrop>>();
                            events.send(FileDragAndDrop::HoveredFile {
                                id: window_id,
                                path_buf,
                            });
                        }
                        WindowEvent::HoveredFileCancelled => {
                            let mut events = world.resource_mut::<Events<FileDragAndDrop>>();
                            events.send(FileDragAndDrop::HoveredFileCancelled { id: window_id });
                        }
                        WindowEvent::Moved(position) => {
                            let position = ivec2(position.x, position.y);
                            window.update_actual_position_from_backend(position);
                            let mut events = world.resource_mut::<Events<WindowMoved>>();
                            events.send(WindowMoved {
                                id: window_id,
                                position,
                            });
                        }
                        _ => {}
                    }
                }
                event::Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta: (x, y) },
                    ..
                } => {
                    let mut mouse_motion_events = app.world.resource_mut::<Events<MouseMotion>>();
                    mouse_motion_events.send(MouseMotion {
                        delta: DVec2 { x, y }.as_vec2(),
                    });
                }
                event::Event::Suspended => {
                    winit_state.active = false;
                }
                event::Event::Resumed => {
                    winit_state.active = true;
                }
                event::Event::MainEventsCleared => {
                    handle_create_window_events(
                        &mut app.world,
                        event_loop,
                        &mut create_window_event_reader,
                    );
                    let winit_config = app.world.resource::<WinitSettings>();
                    let update = if winit_state.active {
                        let windows = app.world.resource::<Windows>();
                        let focused = windows.iter().any(|w| w.is_focused());
                        match winit_config.update_mode(focused) {
                            UpdateMode::Continuous | UpdateMode::Reactive { .. } => true,
                            UpdateMode::ReactiveLowPower { .. } => {
                                winit_state.low_power_event
                                    || winit_state.redraw_request_sent
                                    || winit_state.timeout_reached
                            }
                        }
                    } else {
                        false
                    };
                    if update {
                        winit_state.last_update = Instant::now();
                        app.update();
                    }
                }
                Event::RedrawEventsCleared => {
                    {
                        let winit_config = app.world.resource::<WinitSettings>();
                        let windows = app.world.resource::<Windows>();
                        let focused = windows.iter().any(|w| w.is_focused());
                        let now = Instant::now();
                        use UpdateMode::*;
                        *control_flow = match winit_config.update_mode(focused) {
                            Continuous => ControlFlow::Poll,
                            Reactive { max_wait } | ReactiveLowPower { max_wait } => {
                                if let Some(instant) = now.checked_add(*max_wait) {
                                    ControlFlow::WaitUntil(instant)
                                } else {
                                    ControlFlow::Wait
                                }
                            }
                        };
                    }
                    // This block needs to run after `app.update()` in `MainEventsCleared`. Otherwise,
                    // we won't be able to see redraw requests until the next event, defeating the
                    // purpose of a redraw request!
                    let mut redraw = false;
                    if let Some(app_redraw_events) =
                        app.world.get_resource::<Events<RequestRedraw>>()
                    {
                        if redraw_event_reader.iter(app_redraw_events).last().is_some() {
                            *control_flow = ControlFlow::Poll;
                            redraw = true;
                        }
                    }
                    if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
                        if app_exit_event_reader.iter(app_exit_events).last().is_some() {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    winit_state.redraw_request_sent = redraw;
                }
                _ => (),
            }
        };

        if return_from_run {
            run_return(&mut event_loop, event_handler);
        } else {
            run(event_loop, event_handler);
        }
    }

    fn run<F>(event_loop: EventLoop<()>, event_handler: F) -> !
    where
        F: 'static + FnMut(Event<'_, ()>, &EventLoopWindowTarget<()>, &mut ControlFlow),
    {
        event_loop.run(event_handler)
    }

    // TODO: It may be worth moving this cfg into a procedural macro so that it can be referenced by
    // a single name instead of being copied around.
    // https://gist.github.com/jakerr/231dee4a138f7a5f25148ea8f39b382e seems to work.
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    fn run_return<F>(event_loop: &mut EventLoop<()>, event_handler: F)
    where
        F: FnMut(Event<'_, ()>, &EventLoopWindowTarget<()>, &mut ControlFlow),
    {
        use winit::platform::run_return::EventLoopExtRunReturn;
        event_loop.run_return(event_handler);
    }

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    fn run_return<F>(_event_loop: &mut EventLoop<()>, _event_handler: F)
    where
        F: FnMut(Event<'_, ()>, &EventLoopWindowTarget<()>, &mut ControlFlow),
    {
        panic!("Run return is not supported on this platform!")
    }

    fn handle_create_window_events(
        world: &mut World,
        event_loop: &EventLoopWindowTarget<()>,
        create_window_event_reader: &mut ManualEventReader<CreateWindow>,
    ) {
        let world = world.cell();
        let mut winit_windows = world.non_send_resource_mut::<WinitWindows>();
        let mut windows = world.resource_mut::<Windows>();
        let create_window_events = world.resource::<Events<CreateWindow>>();
        let mut window_created_events = world.resource_mut::<Events<WindowCreated>>();
        #[cfg(not(any(target_os = "windows", target_feature = "x11")))]
        let mut window_resized_events = world.resource_mut::<Events<WindowResized>>();
        for create_window_event in create_window_event_reader.iter(&create_window_events) {
            let window = winit_windows.create_window(
                event_loop,
                create_window_event.id,
                &create_window_event.descriptor,
            );
            // This event is already sent on windows, x11, and xwayland.
            // TODO: we aren't yet sure about native wayland, so we might be able to exclude it,
            // but sending a duplicate event isn't problematic, as windows already does this.
            #[cfg(not(any(target_os = "windows", target_feature = "x11")))]
            window_resized_events.send(WindowResized {
                id: create_window_event.id,
                width: window.width(),
                height: window.height(),
            });
            windows.add(window);
            window_created_events.send(WindowCreated {
                id: create_window_event.id,
            });

            #[cfg(target_arch = "wasm32")]
            {
                let channel = world.resource_mut::<web_resize::CanvasParentResizeEventChannel>();
                if create_window_event.descriptor.fit_canvas_to_parent {
                    let selector = if let Some(selector) = &create_window_event.descriptor.canvas {
                        selector
                    } else {
                        web_resize::WINIT_CANVAS_SELECTOR
                    };
                    channel.listen_to_selector(create_window_event.id, selector);
                }
            }
        }
    }
}

/// Vendored without alteration from `bevy_winit`.
///
/// This should be pub; tracked at https://github.com/bevyengine/bevy/issues/5980
pub(crate) mod converters {
    use bevy_input::{
        keyboard::{KeyCode, KeyboardInput},
        mouse::MouseButton,
        touch::{ForceTouch, TouchInput, TouchPhase},
        ButtonState,
    };
    use bevy_math::Vec2;

    pub fn convert_keyboard_input(keyboard_input: &winit::event::KeyboardInput) -> KeyboardInput {
        KeyboardInput {
            scan_code: keyboard_input.scancode,
            state: convert_element_state(keyboard_input.state),
            key_code: keyboard_input.virtual_keycode.map(convert_virtual_key_code),
        }
    }

    pub fn convert_element_state(element_state: winit::event::ElementState) -> ButtonState {
        match element_state {
            winit::event::ElementState::Pressed => ButtonState::Pressed,
            winit::event::ElementState::Released => ButtonState::Released,
        }
    }

    pub fn convert_mouse_button(mouse_button: winit::event::MouseButton) -> MouseButton {
        match mouse_button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Other(val) => MouseButton::Other(val),
        }
    }

    pub fn convert_touch_input(
        touch_input: winit::event::Touch,
        location: winit::dpi::LogicalPosition<f32>,
    ) -> TouchInput {
        TouchInput {
            phase: match touch_input.phase {
                winit::event::TouchPhase::Started => TouchPhase::Started,
                winit::event::TouchPhase::Moved => TouchPhase::Moved,
                winit::event::TouchPhase::Ended => TouchPhase::Ended,
                winit::event::TouchPhase::Cancelled => TouchPhase::Cancelled,
            },
            position: Vec2::new(location.x as f32, location.y as f32),
            force: touch_input.force.map(|f| match f {
                winit::event::Force::Calibrated {
                    force,
                    max_possible_force,
                    altitude_angle,
                } => ForceTouch::Calibrated {
                    force,
                    max_possible_force,
                    altitude_angle,
                },
                winit::event::Force::Normalized(x) => ForceTouch::Normalized(x),
            }),
            id: touch_input.id,
        }
    }

    pub fn convert_virtual_key_code(virtual_key_code: winit::event::VirtualKeyCode) -> KeyCode {
        match virtual_key_code {
            winit::event::VirtualKeyCode::Key1 => KeyCode::Key1,
            winit::event::VirtualKeyCode::Key2 => KeyCode::Key2,
            winit::event::VirtualKeyCode::Key3 => KeyCode::Key3,
            winit::event::VirtualKeyCode::Key4 => KeyCode::Key4,
            winit::event::VirtualKeyCode::Key5 => KeyCode::Key5,
            winit::event::VirtualKeyCode::Key6 => KeyCode::Key6,
            winit::event::VirtualKeyCode::Key7 => KeyCode::Key7,
            winit::event::VirtualKeyCode::Key8 => KeyCode::Key8,
            winit::event::VirtualKeyCode::Key9 => KeyCode::Key9,
            winit::event::VirtualKeyCode::Key0 => KeyCode::Key0,
            winit::event::VirtualKeyCode::A => KeyCode::A,
            winit::event::VirtualKeyCode::B => KeyCode::B,
            winit::event::VirtualKeyCode::C => KeyCode::C,
            winit::event::VirtualKeyCode::D => KeyCode::D,
            winit::event::VirtualKeyCode::E => KeyCode::E,
            winit::event::VirtualKeyCode::F => KeyCode::F,
            winit::event::VirtualKeyCode::G => KeyCode::G,
            winit::event::VirtualKeyCode::H => KeyCode::H,
            winit::event::VirtualKeyCode::I => KeyCode::I,
            winit::event::VirtualKeyCode::J => KeyCode::J,
            winit::event::VirtualKeyCode::K => KeyCode::K,
            winit::event::VirtualKeyCode::L => KeyCode::L,
            winit::event::VirtualKeyCode::M => KeyCode::M,
            winit::event::VirtualKeyCode::N => KeyCode::N,
            winit::event::VirtualKeyCode::O => KeyCode::O,
            winit::event::VirtualKeyCode::P => KeyCode::P,
            winit::event::VirtualKeyCode::Q => KeyCode::Q,
            winit::event::VirtualKeyCode::R => KeyCode::R,
            winit::event::VirtualKeyCode::S => KeyCode::S,
            winit::event::VirtualKeyCode::T => KeyCode::T,
            winit::event::VirtualKeyCode::U => KeyCode::U,
            winit::event::VirtualKeyCode::V => KeyCode::V,
            winit::event::VirtualKeyCode::W => KeyCode::W,
            winit::event::VirtualKeyCode::X => KeyCode::X,
            winit::event::VirtualKeyCode::Y => KeyCode::Y,
            winit::event::VirtualKeyCode::Z => KeyCode::Z,
            winit::event::VirtualKeyCode::Escape => KeyCode::Escape,
            winit::event::VirtualKeyCode::F1 => KeyCode::F1,
            winit::event::VirtualKeyCode::F2 => KeyCode::F2,
            winit::event::VirtualKeyCode::F3 => KeyCode::F3,
            winit::event::VirtualKeyCode::F4 => KeyCode::F4,
            winit::event::VirtualKeyCode::F5 => KeyCode::F5,
            winit::event::VirtualKeyCode::F6 => KeyCode::F6,
            winit::event::VirtualKeyCode::F7 => KeyCode::F7,
            winit::event::VirtualKeyCode::F8 => KeyCode::F8,
            winit::event::VirtualKeyCode::F9 => KeyCode::F9,
            winit::event::VirtualKeyCode::F10 => KeyCode::F10,
            winit::event::VirtualKeyCode::F11 => KeyCode::F11,
            winit::event::VirtualKeyCode::F12 => KeyCode::F12,
            winit::event::VirtualKeyCode::F13 => KeyCode::F13,
            winit::event::VirtualKeyCode::F14 => KeyCode::F14,
            winit::event::VirtualKeyCode::F15 => KeyCode::F15,
            winit::event::VirtualKeyCode::F16 => KeyCode::F16,
            winit::event::VirtualKeyCode::F17 => KeyCode::F17,
            winit::event::VirtualKeyCode::F18 => KeyCode::F18,
            winit::event::VirtualKeyCode::F19 => KeyCode::F19,
            winit::event::VirtualKeyCode::F20 => KeyCode::F20,
            winit::event::VirtualKeyCode::F21 => KeyCode::F21,
            winit::event::VirtualKeyCode::F22 => KeyCode::F22,
            winit::event::VirtualKeyCode::F23 => KeyCode::F23,
            winit::event::VirtualKeyCode::F24 => KeyCode::F24,
            winit::event::VirtualKeyCode::Snapshot => KeyCode::Snapshot,
            winit::event::VirtualKeyCode::Scroll => KeyCode::Scroll,
            winit::event::VirtualKeyCode::Pause => KeyCode::Pause,
            winit::event::VirtualKeyCode::Insert => KeyCode::Insert,
            winit::event::VirtualKeyCode::Home => KeyCode::Home,
            winit::event::VirtualKeyCode::Delete => KeyCode::Delete,
            winit::event::VirtualKeyCode::End => KeyCode::End,
            winit::event::VirtualKeyCode::PageDown => KeyCode::PageDown,
            winit::event::VirtualKeyCode::PageUp => KeyCode::PageUp,
            winit::event::VirtualKeyCode::Left => KeyCode::Left,
            winit::event::VirtualKeyCode::Up => KeyCode::Up,
            winit::event::VirtualKeyCode::Right => KeyCode::Right,
            winit::event::VirtualKeyCode::Down => KeyCode::Down,
            winit::event::VirtualKeyCode::Back => KeyCode::Back,
            winit::event::VirtualKeyCode::Return => KeyCode::Return,
            winit::event::VirtualKeyCode::Space => KeyCode::Space,
            winit::event::VirtualKeyCode::Compose => KeyCode::Compose,
            winit::event::VirtualKeyCode::Caret => KeyCode::Caret,
            winit::event::VirtualKeyCode::Numlock => KeyCode::Numlock,
            winit::event::VirtualKeyCode::Numpad0 => KeyCode::Numpad0,
            winit::event::VirtualKeyCode::Numpad1 => KeyCode::Numpad1,
            winit::event::VirtualKeyCode::Numpad2 => KeyCode::Numpad2,
            winit::event::VirtualKeyCode::Numpad3 => KeyCode::Numpad3,
            winit::event::VirtualKeyCode::Numpad4 => KeyCode::Numpad4,
            winit::event::VirtualKeyCode::Numpad5 => KeyCode::Numpad5,
            winit::event::VirtualKeyCode::Numpad6 => KeyCode::Numpad6,
            winit::event::VirtualKeyCode::Numpad7 => KeyCode::Numpad7,
            winit::event::VirtualKeyCode::Numpad8 => KeyCode::Numpad8,
            winit::event::VirtualKeyCode::Numpad9 => KeyCode::Numpad9,
            winit::event::VirtualKeyCode::AbntC1 => KeyCode::AbntC1,
            winit::event::VirtualKeyCode::AbntC2 => KeyCode::AbntC2,
            winit::event::VirtualKeyCode::NumpadAdd => KeyCode::NumpadAdd,
            winit::event::VirtualKeyCode::Apostrophe => KeyCode::Apostrophe,
            winit::event::VirtualKeyCode::Apps => KeyCode::Apps,
            winit::event::VirtualKeyCode::Asterisk => KeyCode::Asterisk,
            winit::event::VirtualKeyCode::Plus => KeyCode::Plus,
            winit::event::VirtualKeyCode::At => KeyCode::At,
            winit::event::VirtualKeyCode::Ax => KeyCode::Ax,
            winit::event::VirtualKeyCode::Backslash => KeyCode::Backslash,
            winit::event::VirtualKeyCode::Calculator => KeyCode::Calculator,
            winit::event::VirtualKeyCode::Capital => KeyCode::Capital,
            winit::event::VirtualKeyCode::Colon => KeyCode::Colon,
            winit::event::VirtualKeyCode::Comma => KeyCode::Comma,
            winit::event::VirtualKeyCode::Convert => KeyCode::Convert,
            winit::event::VirtualKeyCode::NumpadDecimal => KeyCode::NumpadDecimal,
            winit::event::VirtualKeyCode::NumpadDivide => KeyCode::NumpadDivide,
            winit::event::VirtualKeyCode::Equals => KeyCode::Equals,
            winit::event::VirtualKeyCode::Grave => KeyCode::Grave,
            winit::event::VirtualKeyCode::Kana => KeyCode::Kana,
            winit::event::VirtualKeyCode::Kanji => KeyCode::Kanji,
            winit::event::VirtualKeyCode::LAlt => KeyCode::LAlt,
            winit::event::VirtualKeyCode::LBracket => KeyCode::LBracket,
            winit::event::VirtualKeyCode::LControl => KeyCode::LControl,
            winit::event::VirtualKeyCode::LShift => KeyCode::LShift,
            winit::event::VirtualKeyCode::LWin => KeyCode::LWin,
            winit::event::VirtualKeyCode::Mail => KeyCode::Mail,
            winit::event::VirtualKeyCode::MediaSelect => KeyCode::MediaSelect,
            winit::event::VirtualKeyCode::MediaStop => KeyCode::MediaStop,
            winit::event::VirtualKeyCode::Minus => KeyCode::Minus,
            winit::event::VirtualKeyCode::NumpadMultiply => KeyCode::NumpadMultiply,
            winit::event::VirtualKeyCode::Mute => KeyCode::Mute,
            winit::event::VirtualKeyCode::MyComputer => KeyCode::MyComputer,
            winit::event::VirtualKeyCode::NavigateForward => KeyCode::NavigateForward,
            winit::event::VirtualKeyCode::NavigateBackward => KeyCode::NavigateBackward,
            winit::event::VirtualKeyCode::NextTrack => KeyCode::NextTrack,
            winit::event::VirtualKeyCode::NoConvert => KeyCode::NoConvert,
            winit::event::VirtualKeyCode::NumpadComma => KeyCode::NumpadComma,
            winit::event::VirtualKeyCode::NumpadEnter => KeyCode::NumpadEnter,
            winit::event::VirtualKeyCode::NumpadEquals => KeyCode::NumpadEquals,
            winit::event::VirtualKeyCode::OEM102 => KeyCode::Oem102,
            winit::event::VirtualKeyCode::Period => KeyCode::Period,
            winit::event::VirtualKeyCode::PlayPause => KeyCode::PlayPause,
            winit::event::VirtualKeyCode::Power => KeyCode::Power,
            winit::event::VirtualKeyCode::PrevTrack => KeyCode::PrevTrack,
            winit::event::VirtualKeyCode::RAlt => KeyCode::RAlt,
            winit::event::VirtualKeyCode::RBracket => KeyCode::RBracket,
            winit::event::VirtualKeyCode::RControl => KeyCode::RControl,
            winit::event::VirtualKeyCode::RShift => KeyCode::RShift,
            winit::event::VirtualKeyCode::RWin => KeyCode::RWin,
            winit::event::VirtualKeyCode::Semicolon => KeyCode::Semicolon,
            winit::event::VirtualKeyCode::Slash => KeyCode::Slash,
            winit::event::VirtualKeyCode::Sleep => KeyCode::Sleep,
            winit::event::VirtualKeyCode::Stop => KeyCode::Stop,
            winit::event::VirtualKeyCode::NumpadSubtract => KeyCode::NumpadSubtract,
            winit::event::VirtualKeyCode::Sysrq => KeyCode::Sysrq,
            winit::event::VirtualKeyCode::Tab => KeyCode::Tab,
            winit::event::VirtualKeyCode::Underline => KeyCode::Underline,
            winit::event::VirtualKeyCode::Unlabeled => KeyCode::Unlabeled,
            winit::event::VirtualKeyCode::VolumeDown => KeyCode::VolumeDown,
            winit::event::VirtualKeyCode::VolumeUp => KeyCode::VolumeUp,
            winit::event::VirtualKeyCode::Wake => KeyCode::Wake,
            winit::event::VirtualKeyCode::WebBack => KeyCode::WebBack,
            winit::event::VirtualKeyCode::WebFavorites => KeyCode::WebFavorites,
            winit::event::VirtualKeyCode::WebForward => KeyCode::WebForward,
            winit::event::VirtualKeyCode::WebHome => KeyCode::WebHome,
            winit::event::VirtualKeyCode::WebRefresh => KeyCode::WebRefresh,
            winit::event::VirtualKeyCode::WebSearch => KeyCode::WebSearch,
            winit::event::VirtualKeyCode::WebStop => KeyCode::WebStop,
            winit::event::VirtualKeyCode::Yen => KeyCode::Yen,
            winit::event::VirtualKeyCode::Copy => KeyCode::Copy,
            winit::event::VirtualKeyCode::Paste => KeyCode::Paste,
            winit::event::VirtualKeyCode::Cut => KeyCode::Cut,
        }
    }
}

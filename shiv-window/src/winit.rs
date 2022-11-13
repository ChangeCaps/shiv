use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use deref_derive::Deref;
use glam::Vec2;
use shiv::hash_map::HashMap;
use shiv_app::{App, AppRunner, Plugin};
use shiv_input::{
    InputEvent, InputState, Key, MouseButton, MouseMotion, MousePosition, MouseScroll,
};
use winit::{
    event::{DeviceEvent, Event, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::CursorGrabMode,
};

use crate::{
    RawWindowHandle, TextInput, Window, WindowCloseRequested, WindowCreated, WindowId,
    WindowRedrawRequested, WindowResized, Windows,
};

#[inline]
fn convert_element_state(state: winit::event::ElementState) -> InputState {
    match state {
        winit::event::ElementState::Pressed => InputState::Pressed,
        winit::event::ElementState::Released => InputState::Released,
    }
}

#[inline]
fn convert_mouse_button(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Other(other) => MouseButton::Other(other),
    }
}

#[inline]
fn convert_key_code(key_code: winit::event::VirtualKeyCode) -> Option<Key> {
    use winit::event::VirtualKeyCode as K;
    Some(match key_code {
        K::A => Key::A,
        K::B => Key::B,
        K::C => Key::C,
        K::D => Key::D,
        K::E => Key::E,
        K::F => Key::F,
        K::G => Key::G,
        K::H => Key::H,
        K::I => Key::I,
        K::J => Key::J,
        K::K => Key::K,
        K::L => Key::L,
        K::M => Key::M,
        K::N => Key::N,
        K::O => Key::O,
        K::P => Key::P,
        K::Q => Key::Q,
        K::R => Key::R,
        K::S => Key::S,
        K::T => Key::T,
        K::U => Key::U,
        K::V => Key::V,
        K::W => Key::W,
        K::X => Key::X,
        K::Y => Key::Y,
        K::Z => Key::Z,
        K::Key0 => Key::Key0,
        K::Key1 => Key::Key1,
        K::Key2 => Key::Key2,
        K::Key3 => Key::Key3,
        K::Key4 => Key::Key4,
        K::Key5 => Key::Key5,
        K::Key6 => Key::Key6,
        K::Key7 => Key::Key7,
        K::Key8 => Key::Key8,
        K::Key9 => Key::Key9,
        K::Escape => Key::Escape,
        K::F1 => Key::F1,
        K::F2 => Key::F2,
        K::F3 => Key::F3,
        K::F4 => Key::F4,
        K::F5 => Key::F5,
        K::F6 => Key::F6,
        K::F7 => Key::F7,
        K::F8 => Key::F8,
        K::F9 => Key::F9,
        K::F10 => Key::F10,
        K::F11 => Key::F11,
        K::F12 => Key::F12,
        K::F13 => Key::F13,
        K::F14 => Key::F14,
        K::F15 => Key::F15,
        K::F16 => Key::F16,
        K::F17 => Key::F17,
        K::F18 => Key::F18,
        K::F19 => Key::F19,
        K::F20 => Key::F20,
        K::F21 => Key::F21,
        K::F22 => Key::F22,
        K::F23 => Key::F23,
        K::F24 => Key::F24,
        K::Snapshot => Key::Snapshot,
        K::Scroll => Key::Scroll,
        K::Pause => Key::Pause,
        K::Insert => Key::Insert,
        K::Home => Key::Home,
        K::Delete => Key::Delete,
        K::End => Key::End,
        K::PageDown => Key::PageDown,
        K::PageUp => Key::PageUp,
        K::Left => Key::Left,
        K::Up => Key::Up,
        K::Right => Key::Right,
        K::Down => Key::Down,
        K::Back => Key::Back,
        K::Return => Key::Return,
        K::Space => Key::Space,
        K::Numlock => Key::Numlock,
        K::Numpad0 => Key::Numpad0,
        K::Numpad1 => Key::Numpad1,
        K::Numpad2 => Key::Numpad2,
        K::Numpad3 => Key::Numpad3,
        K::Numpad4 => Key::Numpad4,
        K::Numpad5 => Key::Numpad5,
        K::Numpad6 => Key::Numpad6,
        K::Numpad7 => Key::Numpad7,
        K::Numpad8 => Key::Numpad8,
        K::Numpad9 => Key::Numpad9,
        _ => return None,
    })
}

pub struct WinitWindow {
    window: winit::window::Window,
    is_cursor_grabbed: AtomicBool,
    is_cursor_visible: AtomicBool,
}

impl WinitWindow {
    pub fn new(window: winit::window::Window) -> Self {
        Self {
            window,
            is_cursor_grabbed: AtomicBool::new(false),
            is_cursor_visible: AtomicBool::new(true),
        }
    }

    pub fn inner(&self) -> &winit::window::Window {
        &self.window
    }
}

impl Window for WinitWindow {
    fn raw_window_handle(&self) -> &dyn RawWindowHandle {
        &self.window
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn focus(&self) {
        self.window.focus_window();
    }

    fn set_position(&self, x: f32, y: f32) {
        self.window
            .set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
    }

    fn get_position(&self) -> (f32, f32) {
        let pos = self.window.outer_position().unwrap();
        (pos.x as f32, pos.y as f32)
    }

    fn set_size(&self, width: u32, height: u32) {
        self.window
            .set_inner_size(winit::dpi::PhysicalSize::new(width, height));
    }

    fn get_size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }

    fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    fn set_visible(&self, visible: bool) {
        self.window.set_visible(visible);
    }

    fn is_visible(&self) -> bool {
        self.window.is_visible().unwrap_or(true)
    }

    fn set_resizable(&self, resizable: bool) {
        self.window.set_resizable(resizable);
    }

    fn is_resizable(&self) -> bool {
        self.window.is_resizable()
    }

    fn set_minimized(&self, minimized: bool) {
        self.window.set_minimized(minimized);
    }

    fn set_maximized(&self, maximized: bool) {
        self.window.set_maximized(maximized);
    }

    fn is_maximized(&self) -> bool {
        self.window.is_maximized()
    }

    fn set_cursor_grabbed(&self, grab: bool) {
        if grab {
            let locked = self.window.set_cursor_grab(CursorGrabMode::Locked);

            if locked.is_err() {
                let confined = self.window.set_cursor_grab(CursorGrabMode::Confined);

                self.is_cursor_grabbed
                    .store(confined.is_ok(), Ordering::Relaxed);
            } else {
                self.is_cursor_grabbed.store(true, Ordering::Relaxed);
            }
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);

            self.is_cursor_grabbed.store(false, Ordering::Release);
        }
    }

    fn is_cursor_grabbed(&self) -> bool {
        self.is_cursor_grabbed.load(Ordering::Acquire)
    }

    fn set_cursor_visible(&self, visible: bool) {
        self.window.set_cursor_visible(visible);
        self.is_cursor_visible.store(visible, Ordering::Release);
    }

    fn is_cursor_visible(&self) -> bool {
        self.is_cursor_grabbed.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug, Default, Deref)]
struct WindowIds {
    #[deref]
    ids: HashMap<winit::window::WindowId, WindowId>,
    next_id: u32,
}

impl WindowIds {
    pub fn push(&mut self, window_id: winit::window::WindowId) -> WindowId {
        let id = WindowId::from_u32(self.next_id);
        self.next_id += 1;
        self.ids.insert(window_id, id);
        id
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WinitPlugin;

impl Plugin for WinitPlugin {
    fn build(&self, app: &mut App) {
        let event_loop = EventLoop::new();
        let window = winit::window::WindowBuilder::new()
            .with_title("Shiv Window")
            .build(&event_loop)
            .expect("failed to build window");

        let mut window_ids = WindowIds::default();
        let window_id = window_ids.push(window.id());

        let mut windows = Windows::new(window_id);
        windows.insert(window_id, Arc::new(WinitWindow::new(window)));

        app.send_event(WindowCreated { window_id });
        app.insert_resource(windows);

        let runner = WinitRunner {
            event_loop,
            window_ids,
        };
        app.set_runner(runner);
    }
}

pub struct WinitRunner {
    event_loop: EventLoop<()>,
    window_ids: WindowIds,
}

impl AppRunner for WinitRunner {
    fn run(self: Box<Self>, mut app: App) {
        let WinitRunner {
            event_loop,
            window_ids,
        } = *self;

        event_loop.run(move |event, _, control_flow| match event {
            Event::RedrawRequested(id) => {
                let window_id = window_ids[&id];
                app.send_event(WindowRedrawRequested { window_id });
            }
            Event::MainEventsCleared => {
                app.update();

                if app.exit_requested() {
                    *control_flow = ControlFlow::Exit;
                }

                let windows = app.world.resource::<Windows>();

                for window in windows.values() {
                    window.request_redraw();
                }
            }
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    app.send_event(MouseMotion {
                        delta: Vec2::new(delta.0 as f32, delta.1 as f32),
                    });
                }
                DeviceEvent::MouseWheel { delta } => match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        app.send_event(MouseScroll {
                            delta: Vec2::new(x as f32, y as f32),
                        });
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        app.send_event(MouseScroll {
                            delta: Vec2::new(pos.x as f32, pos.y as f32),
                        });
                    }
                },
                DeviceEvent::Key(input) => {
                    let key = match input.virtual_keycode {
                        Some(key) => match convert_key_code(key) {
                            Some(key) => key,
                            None => return,
                        },
                        None => return,
                    };

                    let state = convert_element_state(input.state);

                    app.send_event(InputEvent::new(key, state));
                }
                DeviceEvent::Text { codepoint } => {
                    app.send_event(TextInput { codepoint });
                }
                _ => {}
            },
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::CloseRequested => {
                    let window_id = window_ids[&window_id];
                    app.send_event(WindowCloseRequested { window_id });
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    let window_id = window_ids[&window_id];
                    app.send_event(WindowResized {
                        window_id,
                        width: new_inner_size.width,
                        height: new_inner_size.height,
                    });
                }
                WindowEvent::Resized(size) => {
                    let window_id = window_ids[&window_id];
                    app.send_event(WindowResized {
                        window_id,
                        width: size.width,
                        height: size.height,
                    });
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let button = convert_mouse_button(button);
                    let state = convert_element_state(state);

                    app.send_event(InputEvent::new(button, state));
                }
                WindowEvent::CursorMoved { position, .. } => {
                    app.send_event(MousePosition {
                        position: Vec2::new(position.x as f32, position.y as f32),
                    });
                }
                _ => {}
            },
            _ => {}
        })
    }
}

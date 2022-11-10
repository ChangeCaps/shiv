use crate::{InputState, Key, Mouse, WindowId};

#[derive(Clone, Copy, Debug)]
pub struct CloseRequested {
    pub window_id: WindowId,
}

#[derive(Clone, Copy, Debug)]
pub struct RedrawRequested {
    pub window_id: WindowId,
}

#[derive(Clone, Copy, Debug)]
pub struct WindowCreated {
    pub window_id: WindowId,
}

#[derive(Clone, Copy, Debug)]
pub struct WindowClosed {
    pub window_id: WindowId,
}

#[derive(Clone, Copy, Debug)]
pub struct WindowResized {
    pub window_id: WindowId,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct MouseMotion {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct KeyInput {
    pub key: Key,
    pub state: InputState,
}

#[derive(Clone, Copy, Debug)]
pub struct MouseInput {
    pub button: Mouse,
    pub state: InputState,
}

#[derive(Clone, Copy, Debug)]
pub struct TextInput {
    pub codepoint: char,
}

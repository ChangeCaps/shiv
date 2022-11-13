use crate::WindowId;

#[derive(Clone, Copy, Debug)]
pub struct WindowCloseRequested {
    pub window_id: WindowId,
}

#[derive(Clone, Copy, Debug)]
pub struct WindowRedrawRequested {
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
pub struct TextInput {
    pub codepoint: char,
}

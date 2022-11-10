use std::sync::Arc;

use deref_derive::{Deref, DerefMut};
use downcast_rs::{impl_downcast, Downcast};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use shiv::hash_map::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId {
    id: u32,
}

impl WindowId {
    pub const fn from_u32(id: u32) -> Self {
        Self { id }
    }

    pub const fn as_u32(self) -> u32 {
        self.id
    }
}

pub trait RawWindowHandle: HasRawWindowHandle + HasRawDisplayHandle {}
impl<T: HasRawWindowHandle + HasRawDisplayHandle> RawWindowHandle for T {}

pub trait Window: Downcast + Send + Sync {
    fn raw_window_handle(&self) -> &dyn RawWindowHandle;
    fn request_redraw(&self);
    fn focus(&self);
    fn set_position(&self, x: f32, y: f32);
    fn get_position(&self) -> (f32, f32);
    fn set_size(&self, width: u32, height: u32);
    fn get_size(&self) -> (u32, u32);
    fn set_title(&self, title: &str);
    fn set_visible(&self, visible: bool);
    fn is_visible(&self) -> bool;
    fn set_resizable(&self, resizable: bool);
    fn is_resizable(&self) -> bool;
    fn set_minimized(&self, minimized: bool);
    fn set_maximized(&self, maximized: bool);
    fn is_maximized(&self) -> bool;
    fn set_cursor_grabbed(&self, grab: bool);
    fn is_cursor_grabbed(&self) -> bool;
    fn set_cursor_visible(&self, visible: bool);
    fn is_cursor_visible(&self) -> bool;
}

impl_downcast!(Window);

#[derive(Clone, Deref, DerefMut)]
pub struct Windows {
    primary: WindowId,
    #[deref]
    windows: HashMap<WindowId, Arc<dyn Window>>,
}

impl Windows {
    #[inline]
    pub fn new(primary: WindowId) -> Self {
        Self {
            primary,
            windows: HashMap::default(),
        }
    }

    #[inline]
    pub fn primary_id(&self) -> WindowId {
        self.primary
    }

    #[inline]
    pub fn get_primary(&self) -> Option<&Arc<dyn Window>> {
        self.get(&self.primary)
    }

    #[inline]
    pub fn primary(&self) -> &Arc<dyn Window> {
        self.get_primary().expect("primary window not found")
    }
}

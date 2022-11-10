mod event;
mod key;
mod window;
#[cfg(feature = "winit")]
pub mod winit;

pub use event::*;
pub use key::*;
pub use window::*;

use shiv_app::{App, Plugin, Plugins};

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CloseRequested>();
        app.add_event::<RedrawRequested>();
        app.add_event::<WindowResized>();
        app.add_event::<MouseMotion>();
        app.add_event::<KeyInput>();
        app.add_event::<MouseInput>();
        app.add_event::<TextInput>();
    }

    fn dependencies(&self, plugins: &mut Plugins) {
        #[cfg(feature = "winit")]
        plugins.add(winit::WinitPlugin);
    }
}

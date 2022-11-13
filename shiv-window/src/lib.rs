mod event;
mod window;
#[cfg(feature = "winit")]
pub mod winit;

pub use event::*;
pub use window::*;

use shiv::{
    prelude::{EventReader, EventWriter},
    schedule::{DefaultStage, IntoSystemDescriptor, ShouldRun, SystemLabel},
    system::{Res, ResMut},
};
use shiv_app::{App, AppExit, Plugin, Plugins};
use shiv_input::InputPlugin;

#[derive(Clone, Copy, Debug, Default)]
pub struct ManuallyCloseWindows;

pub fn should_auto_close_windows(manually_close: Option<Res<ManuallyCloseWindows>>) -> ShouldRun {
    manually_close.is_none().into()
}

#[derive(Clone, Copy, Debug, Default, SystemLabel)]
pub struct CloseWindowSystem;

pub fn close_window_system(
    mut events: EventReader<WindowCloseRequested>,
    mut closed: EventWriter<WindowClosed>,
    mut exit: EventWriter<AppExit>,
    mut windows: ResMut<Windows>,
) {
    for event in events.iter() {
        windows.remove(&event.window_id);
        closed.send(WindowClosed {
            window_id: event.window_id,
        });
    }

    if windows.is_empty() {
        exit.send_default();
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WindowCloseRequested>();
        app.add_event::<WindowRedrawRequested>();
        app.add_event::<WindowCreated>();
        app.add_event::<WindowClosed>();
        app.add_event::<WindowResized>();
        app.add_event::<TextInput>();

        app.add_system_to_stage(
            DefaultStage::Last,
            close_window_system
                .label(CloseWindowSystem)
                .with_run_criteria(should_auto_close_windows),
        );
    }

    fn dependencies(&self, plugins: &mut Plugins) {
        #[cfg(feature = "winit")]
        plugins.add(winit::WinitPlugin);

        plugins.add(InputPlugin);
    }
}

use shiv::{
    event::EventSystem,
    schedule::{DefaultStage, IntoSystemDescriptor},
};
use shiv_app::{App, Plugin};

use crate::{Input, Key, Mouse, MouseButton, MouseMotion, MousePosition, MouseScroll};

#[derive(Clone, Copy, Debug, Default)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Input<Key>>();
        app.add_event::<Input<MouseButton>>();
        app.add_event::<MouseMotion>();
        app.add_event::<MousePosition>();
        app.add_event::<MouseScroll>();

        app.init_resource::<Input<Key>>();
        app.init_resource::<Input<MouseButton>>();
        app.init_resource::<Mouse>();

        app.add_system_to_stage(
            DefaultStage::First,
            Input::<Key>::event_system.after(EventSystem),
        );
        app.add_system_to_stage(
            DefaultStage::First,
            Input::<MouseButton>::event_system.after(EventSystem),
        );
        app.add_system_to_stage(DefaultStage::First, Mouse::system.after(EventSystem));

        app.add_system_to_stage(
            DefaultStage::Last,
            Input::<Key>::update_system.after(EventSystem),
        );
        app.add_system_to_stage(
            DefaultStage::Last,
            Input::<MouseButton>::update_system.after(EventSystem),
        );
    }
}

use shiv::prelude::Events;

use crate::{App, AppExit};

pub trait AppRunner: 'static {
    fn run(self: Box<Self>, app: App);
}

pub struct RunOnce;

impl AppRunner for RunOnce {
    fn run(self: Box<Self>, mut app: App) {
        app.schedule.run_once(&mut app.world);
    }
}

pub struct RunLoop;

impl AppRunner for RunLoop {
    fn run(self: Box<Self>, mut app: App) {
        loop {
            app.schedule.run_once(&mut app.world);

            if let Some(events) = app.world.get_resource::<Events<AppExit>>() {
                if !events.is_empty() {
                    break;
                }
            }
        }
    }
}

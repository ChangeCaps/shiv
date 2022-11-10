use crate::App;

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
            app.update();

            if app.exit_requested() {
                break;
            }
        }
    }
}

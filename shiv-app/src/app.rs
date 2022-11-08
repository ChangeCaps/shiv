use std::mem;

use crate::{AppRunner, Plugin, RunOnce};

use shiv::{
    prelude::Event,
    schedule::{IntoSystemDescriptor, Schedule, ShouldRun, Stage, StageLabel, SystemStage},
    world::World,
};

/// An event that when emitted will tell the [`App`] to exit.
pub struct AppExit;

/// Label for the startup [`Schedule`].
#[derive(StageLabel)]
pub struct StartupSchedule;

/// [`Stage`]s that are run once at the start of the [`App`].
#[derive(StageLabel)]
pub enum StartupStage {
    /// Runs before [`StartupStage::Update`].
    PreStartup,
    /// Default stage for startup systems.
    Startup,
    /// Runs after [`StartupStage::Startup`].
    PostStartup,
}

/// Core [`App`] [`Stage`]s.
#[derive(StageLabel)]
pub enum CoreStage {
    /// Runs before [`CoreStage::Update`].
    PreUpdate,
    /// The [`Stage`] responsible for most app logic. Systems are registered here by default.
    Update,
    /// Runs after [`CoreStage::Update`].
    PostUpdate,
}

pub struct App {
    pub world: World,
    pub schedule: Schedule,
    pub runner: Box<dyn AppRunner>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new [`App`] without [`CoreStage`]s.
    pub fn empty() -> Self {
        Self {
            world: World::new(),
            schedule: Schedule::new(),
            runner: Box::new(RunOnce),
        }
    }

    /// Creates a new [`App`] with the default [`CoreStage`]s.
    pub fn new() -> Self {
        let mut app = Self::empty();

        let mut startup_schedule = Schedule::new();

        startup_schedule.add_stage(StartupStage::PreStartup, SystemStage::parallel());
        startup_schedule.add_stage(StartupStage::Startup, SystemStage::parallel());
        startup_schedule.add_stage(StartupStage::PostStartup, SystemStage::parallel());
        startup_schedule.set_run_criteria(ShouldRun::once);

        app.add_stage(StartupSchedule, startup_schedule);

        app.add_stage(CoreStage::PreUpdate, SystemStage::parallel());
        app.add_stage(CoreStage::Update, SystemStage::parallel());
        app.add_stage(CoreStage::PostUpdate, SystemStage::parallel());

        app.add_event::<AppExit>();

        app
    }

    pub fn add_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.build(self);
        self
    }

    /// Adds a [`Stage`] to the [`App`].
    pub fn add_stage(&mut self, label: impl StageLabel, stage: impl Stage) -> &mut Self {
        self.schedule.add_stage(label, stage);
        self
    }

    /// Adds `stage` before `before`.
    pub fn add_stage_after(
        &mut self,
        after: impl StageLabel,
        label: impl StageLabel,
        stage: impl Stage,
    ) -> &mut Self {
        self.schedule.add_stage_after(after, label, stage);
        self
    }

    /// Adds `stage` after `after`.
    pub fn add_stage_before(
        &mut self,
        before: impl StageLabel,
        label: impl StageLabel,
        stage: impl Stage,
    ) -> &mut Self {
        self.schedule.add_stage_before(before, label, stage);
        self
    }

    /// Adds `system` to `stage`.
    pub fn add_system_to_stage<Params>(
        &mut self,
        stage: impl StageLabel,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        self.schedule.add_system_to_stage(stage, system);
        self
    }

    /// Adds `system` to [`CoreStage::Update`].
    pub fn add_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.add_system_to_stage(CoreStage::Update, system);
        self
    }

    /// Gets a reference to the [`Stage`] with `label` and type `T`.
    pub fn get_stage<T: StageLabel>(&self, label: T) -> Option<&SystemStage> {
        self.schedule.get_stage(label)
    }

    /// Gets a mutable reference to the [`Stage`] with `label` and type `T`.
    pub fn get_stage_mut<T: StageLabel>(&mut self, label: T) -> Option<&mut SystemStage> {
        self.schedule.get_stage_mut(label)
    }

    /// Gets a reference to the [`Stage`] with `label` and type `T`.
    #[track_caller]
    pub fn stage<T: Stage>(&mut self, label: impl StageLabel) -> &mut T {
        self.schedule.stage_mut(label)
    }

    /// Gets a mutable reference to the [`Stage`] with `label` and type `T`.
    #[track_caller]
    pub fn stage_mut<T: Stage>(&mut self, label: impl StageLabel) -> &mut T {
        self.schedule.stage_mut(label)
    }

    /// Gets a mutable reference the startup schedule.
    #[track_caller]
    pub fn startup_schedule(&mut self) -> &mut Schedule {
        self.schedule.stage_mut(StartupSchedule)
    }

    /// Adds a [`Stage`] to the [`App`] in startup.
    pub fn add_startup_stage(&mut self, label: impl StageLabel, stage: impl Stage) -> &mut Self {
        self.startup_schedule().add_stage(label, stage);
        self
    }

    /// Adds `stage` before `before` in startup.
    pub fn add_startup_stage_after(
        &mut self,
        after: impl StageLabel,
        label: impl StageLabel,
        stage: impl Stage,
    ) -> &mut Self {
        self.startup_schedule().add_stage_after(after, label, stage);
        self
    }

    /// Adds `stage` after `after` in startup.
    pub fn add_startup_stage_before(
        &mut self,
        before: impl StageLabel,
        label: impl StageLabel,
        stage: impl Stage,
    ) -> &mut Self {
        (self.startup_schedule()).add_stage_before(before, label, stage);
        self
    }

    /// Adds `system` to `stage` in startup.
    pub fn add_startup_system_to_stage<Params>(
        &mut self,
        stage: impl StageLabel,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        self.startup_schedule().add_system_to_stage(stage, system);
        self
    }

    /// Adds `system` to [`StartupStage::Startup`] in startup.
    pub fn add_startup_system<Params>(
        &mut self,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        self.add_startup_system_to_stage(CoreStage::Update, system);
        self
    }

    /// Sets the [`AppRunner`].
    pub fn add_runner(&mut self, runner: impl AppRunner) -> &mut Self {
        self.runner = Box::new(runner);
        self
    }

    pub fn add_event<T: Event>(&mut self) -> &mut Self {
        self.schedule.add_event::<T>();
        self
    }

    /// Runs the [`App`].
    ///
    /// This runs the startup schedule, then the [`App::runner`].
    pub fn run(&mut self) {
        let mut app = mem::take(self);
        let runner = mem::replace(&mut app.runner, Box::new(RunOnce));
        runner.run(app);
    }
}

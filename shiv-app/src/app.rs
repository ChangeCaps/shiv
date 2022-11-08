use std::mem;

use crate::{AppRunner, Plugin, RunOnce};

use shiv::{
    schedule::{IntoSystemDescriptor, Schedule, Stage, StageLabel, SystemStage},
    world::World,
};

/// An event that when emitted will tell the [`App`] to exit.
pub struct AppExit;

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
    pub startup: Schedule,
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
            startup: Schedule::new(),
            runner: Box::new(RunOnce),
        }
    }

    /// Creates a new [`App`] with the default [`CoreStage`]s.
    pub fn new() -> Self {
        let mut app = Self::empty();
        app.add_core_stages();
        app
    }

    pub fn add_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.build(self);
        self
    }

    /// Adds [`CoreStage`]s to the [`App`].
    pub fn add_core_stages(&mut self) -> &mut Self {
        self.add_stage(CoreStage::PreUpdate, SystemStage::parallel());
        self.add_stage(CoreStage::Update, SystemStage::parallel());
        self.add_stage(CoreStage::PostUpdate, SystemStage::parallel());

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

    /// Adds a [`Stage`] to the [`App`] in startup.
    pub fn add_startup_stage(&mut self, label: impl StageLabel, stage: impl Stage) -> &mut Self {
        self.startup.add_stage(label, stage);
        self
    }

    /// Adds `stage` before `before` in startup.
    pub fn add_startup_stage_after(
        &mut self,
        after: impl StageLabel,
        label: impl StageLabel,
        stage: impl Stage,
    ) -> &mut Self {
        self.startup.add_stage_after(after, label, stage);
        self
    }

    /// Adds `stage` after `after` in startup.
    pub fn add_startup_stage_before(
        &mut self,
        before: impl StageLabel,
        label: impl StageLabel,
        stage: impl Stage,
    ) -> &mut Self {
        self.startup.add_stage_before(before, label, stage);
        self
    }

    /// Adds `system` to `stage` in startup.
    pub fn add_startup_system_to_stage<Params>(
        &mut self,
        stage: impl StageLabel,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        self.startup.add_system_to_stage(stage, system);
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

    /// Runs the [`App`].
    ///
    /// This runs the startup schedule, then the [`App::runner`].
    pub fn run(&mut self) {
        self.startup.run_once(&mut self.world);

        let mut app = mem::take(self);
        let runner = mem::replace(&mut app.runner, Box::new(RunOnce));
        runner.run(app);
    }
}

use hyena::TaskPool;

use crate as shiv;
use crate::{
    event::{Event, Events},
    hash_map::HashMap,
    world::World,
};

use super::{
    IntoRunCriteria, IntoSystemDescriptor, RunCriteria, ShouldRun, Stage, StageLabel, StageLabelId,
    SystemStage,
};

/// [`Stage`]s that are automatically added by [`Schedule::new`].
///
/// These stages are reserved for use by the [`Schedule`],
/// and can therefore not be added to the [`Schedule`] manually.
#[derive(StageLabel)]
pub enum DefaultStage {
    /// Always runs before all other stages.
    ///
    /// [`Stage`]s cannot be added before this stage.
    First,
    /// Always runs after all other stages.
    ///
    /// [`Stage`]s cannot be added after this stage.
    Last,
}

/// A schedule is a collection of [`Stage`]s that are executed in order.
///
/// # Examples
/// ```rust
/// use shiv::prelude::*;
///  
/// // define a stage label
/// #[derive(StageLabel)]
/// pub enum MyStage {
///     Foo,
/// }
///
/// // define some system labels
/// #[derive(SystemLabel)]
/// pub enum MySystem {
///     Foo,
///     Bar
/// }
///
/// // define a system
/// fn foo_system(mut resource: ResMutInit<u32>) {
///     *resource = 42;
/// }
///
/// // define another system
/// fn bar_system(mut resource: ResMutInit<u32>) {
///     *resource *= 10;
/// }
///
/// // create a schedule with our stage
/// let mut schedule = Schedule::new()
///     .with_stage(MyStage::Foo, SystemStage::parallel());
///
/// // add our systems to the stage
/// schedule.add_system_to_stage(
///     MyStage::Foo,
///     foo_system.label(MySystem::Foo)
/// );
///
/// // systems can be added in any order
/// // system order is determined by their labels
/// schedule.add_system_to_stage(
///     MyStage::Foo,
///     bar_system.label(MySystem::Bar).after(MySystem::Foo)
/// );
///
/// // create a world
/// let mut world = World::new();
///
/// // run our schedule on out world
/// schedule.run_once(&mut world);
///
/// // get the resource from our world
/// assert_eq!(*world.resource::<u32>(), 420);
/// ```
#[derive(Debug)]
pub struct Schedule {
    stages: HashMap<StageLabelId, Box<dyn Stage>>,
    stage_order: Vec<StageLabelId>,
    run_criteria: RunCriteria,
}

impl Default for Schedule {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    /// Creates a new empty schedule.
    #[inline]
    pub fn empty() -> Self {
        Self {
            stages: HashMap::default(),
            stage_order: Vec::new(),
            run_criteria: RunCriteria::default(),
        }
    }

    /// Creates a new schedule with [`DefaultStage`]s.
    ///
    /// [`DefaultStage::First`] is run before all other stages.
    /// [`DefaultStage::Last`] is run after all other stages.
    #[inline]
    pub fn new() -> Self {
        let mut schedule = Self::empty();

        let task_pool = TaskPool::new().expect("Failed to create task pool");
        schedule.push_stage_internal(
            DefaultStage::First,
            SystemStage::parallel_with_task_pool(task_pool.clone()),
        );
        schedule.push_stage_internal(
            DefaultStage::Last,
            SystemStage::parallel_with_task_pool(task_pool.clone()),
        );

        schedule
    }

    /// Adds a new stage to the schedule just before [`DefaultStage::Last`].
    ///
    /// If [`DefaultStage::Last`] is not present, `stage` will be added at the end.
    ///
    /// # Panics
    /// - A stage with the same `label` already exists.
    /// - `label` is reserved i.e., `label` is [`DefaultStage::First`] or [`DefaultStage::Last`].
    pub fn with_stage(mut self, label: impl StageLabel, stage: impl Stage) -> Self {
        self.add_stage(label, stage);
        self
    }

    /// Adds a new stage to the schedule just before `before`.
    ///
    /// # Panics
    /// - A stage with the same `label` already exists.
    /// - `label` is reserved i.e., `label` is [`DefaultStage::First`] or [`DefaultStage::Last`].
    /// - `before` is [`DefaultStage::First`].
    #[track_caller]
    pub fn with_stage_before(
        mut self,
        before: impl StageLabel,
        label: impl StageLabel,
        stage: impl Stage,
    ) -> Self {
        self.add_stage_before(before, label, stage);
        self
    }

    /// Adds a new stage to the schedule just after `after`.
    ///
    /// # Panics
    /// - A stage with the same `label` already exists.
    /// - `label` is reserved i.e., `label` is [`DefaultStage::First`] or [`DefaultStage::Last`].
    /// - `after` is [`DefaultStage::Last`].
    #[track_caller]
    pub fn with_stage_after(
        mut self,
        after: impl StageLabel,
        label: impl StageLabel,
        stage: impl Stage,
    ) -> Self {
        self.add_stage_after(after, label, stage);
        self
    }

    /// Returns true if the schedule contains a stage with the given `label`.
    pub fn contains_stage(&self, label: impl StageLabel) -> bool {
        self.stages.contains_key(&label.label())
    }

    /// Sets the run criteria for the schedule.
    pub fn set_run_criteria<Marker>(
        &mut self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> &mut Self {
        self.run_criteria = run_criteria.into_run_criteria();
        self
    }

    /// Sets the run criteria for the schedule.
    pub fn with_run_criteria<Marker>(mut self, run_criteria: impl IntoRunCriteria<Marker>) -> Self {
        self.set_run_criteria(run_criteria);
        self
    }

    fn push_stage_internal(&mut self, label: impl StageLabel, stage: impl Stage) -> &mut Self {
        let id = label.label();

        self.stages.insert(id, Box::new(stage));
        self.stage_order.push(id);

        self
    }

    #[inline]
    fn validate_add_stage(&self, label: impl StageLabel) {
        let id = label.label();

        if self.stages.contains_key(&id) {
            panic!("Stage with label `{}` already exists", id);
        }

        if id == DefaultStage::First.label() || id == DefaultStage::Last.label() {
            panic!(
                "Stage with label `{}` is reserved and cannot be added manually. See `Schedule::new`.",
                id
            );
        }
    }

    /// Adds a new stage to the schedule just before [`DefaultStage::Last`].
    ///
    /// If [`DefaultStage::Last`] is not present, `stage` will be added at the end.
    ///
    /// # Panics
    /// - A stage with the same `label` already exists.
    /// - `label` is reserved i.e., `label` is [`DefaultStage::First`] or [`DefaultStage::Last`].
    pub fn add_stage(&mut self, label: impl StageLabel, stage: impl Stage) -> &mut Self {
        let id = label.label();

        self.validate_add_stage(id);

        self.stages.insert(id, Box::new(stage));

        if let Some(index) = self.get_stage_index(DefaultStage::Last.label()) {
            self.stage_order.insert(index, id);
        } else {
            self.stage_order.push(id);
        }

        self
    }

    #[inline]
    fn get_stage_index(&self, label: impl StageLabel) -> Option<usize> {
        let id = label.label();

        self.stage_order.iter().position(|stage_id| stage_id == &id)
    }

    #[inline]
    #[track_caller]
    fn stage_index(&self, label: impl StageLabel) -> usize {
        let id = label.label();
        if let Some(index) = self.get_stage_index(id) {
            index
        } else {
            panic!("Stage with label `{}` does not exist", id);
        }
    }

    /// Adds a new stage to the schedule just before `before`.
    ///
    /// # Panics
    /// - A stage with the same `label` already exists.
    /// - `label` is reserved i.e., `label` is [`DefaultStage::First`] or [`DefaultStage::Last`].
    /// - `before` is [`DefaultStage::First`].
    #[track_caller]
    pub fn add_stage_before(
        &mut self,
        before: impl StageLabel,
        label: impl StageLabel,
        stage: impl Stage,
    ) -> &mut Self {
        let before = before.label();
        let label = label.label();

        self.validate_add_stage(label);

        if before.label() == DefaultStage::First.label() {
            panic!("Cannot add stage before `CoreStage::First`");
        }

        let index = self.stage_index(before);
        self.stages.insert(label, Box::new(stage));
        self.stage_order.insert(index, label);

        self
    }

    /// Adds a new stage to the schedule just after `after`.
    ///
    /// # Panics
    /// - A stage with the same `label` already exists.
    /// - `label` is reserved i.e., `label` is [`DefaultStage::First`] or [`DefaultStage::Last`].
    /// - `after` is [`DefaultStage::Last`].
    #[track_caller]
    pub fn add_stage_after(
        &mut self,
        after: impl StageLabel,
        label: impl StageLabel,
        stage: impl Stage,
    ) -> &mut Self {
        let after = after.label();
        let label = label.label();

        self.validate_add_stage(label);

        if after.label() == DefaultStage::Last.label() {
            panic!("Cannot add stage after CoreStage::Last");
        }

        let index = self.stage_index(after);
        self.stages.insert(label, Box::new(stage));
        self.stage_order.insert(index + 1, label);

        self
    }

    /// Gets the stage with the given `label` and type `T`.
    ///
    /// Returns `None` if the stage does not exist or if the stage is not of type `T`.
    pub fn get_stage<T: Stage>(&self, label: impl StageLabel) -> Option<&T> {
        self.stages.get(&label.label())?.downcast_ref()
    }

    /// Gets the stage with the given `label` and type `T`.
    ///
    /// Returns `None` if the stage does not exist or if the stage is not of type `T`.
    pub fn get_stage_mut<T: Stage>(&mut self, label: impl StageLabel) -> Option<&mut T> {
        self.stages.get_mut(&label.label())?.downcast_mut()
    }

    /// Gets the stage with the given `label` and type `T`.
    ///
    /// # Panics
    /// - The stage does not exist.
    /// - The stage is not of type `T`.
    #[track_caller]
    pub fn stage<T: Stage>(&self, label: impl StageLabel) -> &T {
        let id = label.label();
        let stage = if let Some(stage) = self.stages.get(&id) {
            stage
        } else {
            panic!("Stage with label `{}` does not exist", id);
        };

        stage
            .downcast_ref()
            .expect("Stage is not the correct type.")
    }

    /// Gets the stage with the given `label` and type `T`.
    ///
    /// # Panics
    /// - The stage does not exist.
    /// - The stage is not of type `T`.
    #[track_caller]
    pub fn stage_mut<T: Stage>(&mut self, label: impl StageLabel) -> &mut T {
        let id = label.label();
        let stage = if let Some(stage) = self.stages.get_mut(&id) {
            stage
        } else {
            panic!("Stage with label `{}` does not exist", id);
        };

        stage
            .downcast_mut()
            .expect("Stage is not the correct type.")
    }

    /// Adds a system to the stage with the given `label`.
    ///
    /// # Panics
    /// - The stage does not exist.
    /// - The stage is not of type [`SystemStage`].
    #[track_caller]
    pub fn add_system_to_stage<Params>(
        &mut self,
        label: impl StageLabel,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        let stage = self.stage_mut::<SystemStage>(label);
        stage.add_system(system);

        self
    }

    /// Adds [`Events::update_system`] to [`DefaultStage::First`].
    /// If the stage does not exist, this function does nothing.
    pub fn add_event<E: Event>(&mut self) {
        if let Some(stage) = self.get_stage_mut::<SystemStage>(DefaultStage::First) {
            stage.add_system(Events::<E>::update_system);
        }
    }

    /// Runs the schedule once.
    ///
    /// **Note:** This function will most likely panic if run with two different worlds.
    pub fn run_once(&mut self, world: &mut World) {
        match self.run_criteria.should_run(world) {
            ShouldRun::Yes => {}
            ShouldRun::No => return,
        }

        for stage_id in &self.stage_order {
            #[cfg(feature = "tracing")]
            let _ = tracing::info_span!("stage", name = stage_id.label().to_string()).entered();

            let stage = self.stages.get_mut(stage_id).unwrap();
            stage.run(world);
        }

        world.check_change_ticks();
        world.clear_trackers();
    }
}

impl Stage for Schedule {
    fn run(&mut self, world: &mut World) {
        self.run_once(world);
    }
}

#[cfg(test)]
mod tests {
    use crate as shiv;
    use crate::schedule::{DefaultStage, Schedule, StageLabel, SystemStage};

    #[derive(StageLabel)]
    pub struct TestStage;

    #[test]
    fn default_stages() {
        let schedule = Schedule::new();

        assert!(schedule.contains_stage(DefaultStage::First));
        assert!(schedule.contains_stage(DefaultStage::Last));
    }

    #[test]
    #[should_panic]
    fn reserved_first_stages() {
        let mut schedule = Schedule::empty();
        schedule.add_stage(DefaultStage::First, SystemStage::parallel());
    }

    #[test]
    #[should_panic]
    fn reserved_last_stages() {
        let mut schedule = Schedule::new();
        schedule.add_stage(DefaultStage::Last, SystemStage::parallel());
    }

    #[test]
    #[should_panic]
    fn before_first() {
        let mut schedule = Schedule::new();
        schedule.add_stage_before(DefaultStage::First, TestStage, SystemStage::parallel());
    }

    #[test]
    #[should_panic]
    fn after_last() {
        let mut schedule = Schedule::new();
        schedule.add_stage_after(DefaultStage::Last, TestStage, SystemStage::parallel());
    }
}

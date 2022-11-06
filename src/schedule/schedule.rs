use hyena::TaskPool;

use crate::{
    hash_map::HashMap, IntoSystemDescriptor, Stage, StageLabel, StageLabelId, SystemStage, World,
};

use crate::{self as termite, Event, Events};

#[derive(StageLabel)]
pub enum DefaultStage {
    First,
    Last,
}

#[derive(Debug)]
pub struct Schedule {
    stages: HashMap<StageLabelId, Box<dyn Stage>>,
    stage_order: Vec<StageLabelId>,
}

impl Default for Schedule {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    #[inline]
    pub fn empty() -> Self {
        Self {
            stages: HashMap::default(),
            stage_order: Vec::new(),
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

    pub fn with_stage(mut self, label: impl StageLabel, stage: impl Stage) -> Self {
        self.add_stage(label, stage);
        self
    }

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

    pub fn contains_stage(&self, label: impl StageLabel) -> bool {
        self.stages.contains_key(&label.label())
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

    pub fn get_stage<T: Stage>(&self, label: impl StageLabel) -> Option<&T> {
        self.stages.get(&label.label())?.downcast_ref()
    }

    pub fn get_stage_mut<T: Stage>(&mut self, label: impl StageLabel) -> Option<&mut T> {
        self.stages.get_mut(&label.label())?.downcast_mut()
    }

    #[track_caller]
    pub fn stage<T: Stage>(&self, label: impl StageLabel) -> &T {
        let id = label.label();
        let stage = self
            .stages
            .get(&id)
            .unwrap_or_else(|| panic!("Stage {} does not exist.", id));

        stage
            .downcast_ref()
            .expect("Stage is not the correct type.")
    }

    #[track_caller]
    pub fn stage_mut<T: Stage>(&mut self, label: impl StageLabel) -> &mut T {
        let id = label.label();
        let stage = self
            .stages
            .get_mut(&id)
            .unwrap_or_else(|| panic!("Stage {} does not exist.", id));

        stage
            .downcast_mut()
            .expect("Stage is not the correct type.")
    }

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

    pub fn add_event<E: Event>(&mut self) {
        if let Some(stage) = self.get_stage_mut::<SystemStage>(DefaultStage::First) {
            stage.add_system(Events::<E>::update_system);
        }
    }

    pub fn run_once(&mut self, world: &mut World) {
        for stage_id in &self.stage_order {
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
    use crate as termite;
    use crate::*;

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

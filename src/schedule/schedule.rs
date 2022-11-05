use hyena::TaskPool;

use crate::{
    hash_map::HashMap, IntoSystemDescriptor, Stage, StageLabel, StageLabelId, SystemStage, World,
};

use crate::{self as termite, Event, Events};

#[derive(StageLabel)]
pub enum CoreStage {
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

    /// Creates a new schedule [`CoreStage`]s.
    #[inline]
    pub fn new() -> Self {
        let mut schedule = Self::empty();

        let task_pool = TaskPool::new().expect("Failed to create task pool");
        schedule.add_stage(
            CoreStage::First,
            SystemStage::parallel_with_task_pool(task_pool.clone()),
        );
        schedule.add_stage(
            CoreStage::Last,
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

    pub fn add_stage(&mut self, label: impl StageLabel, stage: impl Stage) -> &mut Self {
        let id = label.label();

        self.stages.insert(id, Box::new(stage));
        self.stage_order.push(id);

        self
    }

    #[inline]
    fn stage_index(&self, label: StageLabelId) -> Option<usize> {
        self.stage_order.iter().position(|id| *id == label)
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

        let index = self.stage_index(before).unwrap_or_else(|| {
            panic!(
                "Stage {} does not exist. Cannot add stage {} before it.",
                before, label
            )
        });

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

        let index = self.stage_index(after).unwrap_or_else(|| {
            panic!(
                "Stage {} does not exist. Cannot add stage {} after it.",
                after, label
            )
        });

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
        if let Some(stage) = self.get_stage_mut::<SystemStage>(CoreStage::First) {
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

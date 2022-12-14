use ahash::HashSet;
use downcast_rs::{impl_downcast, Downcast};
use hyena::TaskPool;

use crate::{
    hash_map::HashMap,
    schedule::SystemLabelId,
    world::{World, WorldId},
};

use super::{
    IntoRunCriteria, IntoSystemDescriptor, ParallelExecutor, RunCriteria, Schedule,
    SequentialExecutor, ShouldRun, SystemContainer, SystemExecutor, SystemLabel,
};

pub trait Stage: Downcast + Send + Sync {
    fn run(&mut self, world: &mut World);
}

impl std::fmt::Debug for dyn Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(system_stage) = self.downcast_ref::<SystemStage>() {
            write!(f, "{system_stage:?}")
        } else if let Some(schedule) = self.downcast_ref::<Schedule>() {
            write!(f, "{schedule:?}")
        } else {
            write!(f, "{{Custom Stage}}")
        }
    }
}

impl_downcast!(Stage);

#[derive(Debug)]
pub struct SystemStage {
    world_id: Option<WorldId>,
    executor: Box<dyn SystemExecutor>,
    run_criteria: RunCriteria,
    exclusive_systems: Vec<SystemContainer>,
    parallel_systems: Vec<SystemContainer>,
    uninitialized_exclusive: Vec<usize>,
    uninitialized_parallel: Vec<usize>,
    systems_modified: bool,
    executor_modified: bool,
}

impl SystemStage {
    pub fn new(executor: impl SystemExecutor) -> Self {
        Self {
            world_id: None,
            executor: Box::new(executor),
            run_criteria: RunCriteria::default(),
            exclusive_systems: Vec::new(),
            parallel_systems: Vec::new(),
            uninitialized_exclusive: Vec::new(),
            uninitialized_parallel: Vec::new(),
            systems_modified: true,
            executor_modified: true,
        }
    }

    pub fn sequential() -> Self {
        Self::new(SequentialExecutor)
    }

    pub fn parallel() -> Self {
        Self::new(ParallelExecutor::new())
    }

    pub fn parallel_with_task_pool(task_pool: TaskPool) -> Self {
        Self::new(ParallelExecutor::new_with_task_pool(task_pool))
    }

    pub fn add_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) {
        let descriptor = system.into_descriptor();
        let container = SystemContainer::from_descriptor(descriptor);

        if container.system().is_exclusive() {
            let index = self.exclusive_systems.len();
            self.exclusive_systems.push(container);
            self.uninitialized_exclusive.push(index);
        } else {
            let index = self.parallel_systems.len();
            self.parallel_systems.push(container);
            self.uninitialized_parallel.push(index);
        }

        self.systems_modified = true;
    }

    #[must_use]
    pub fn with_system<Params>(mut self, system: impl IntoSystemDescriptor<Params>) -> Self {
        self.add_system(system);
        self
    }

    pub fn has_system(&self, label: impl SystemLabel) -> bool {
        let label = label.label();
        self.has_exclusive_system(&label) || self.has_parallel_system(&label)
    }

    fn has_exclusive_system(&self, label: &SystemLabelId) -> bool {
        self.exclusive_systems
            .iter()
            .any(|system| system.labels().contains(label))
    }

    fn has_parallel_system(&self, label: &SystemLabelId) -> bool {
        self.parallel_systems
            .iter()
            .any(|system| system.labels().contains(label))
    }

    pub fn set_run_criteria<Marker>(&mut self, run_criteria: impl IntoRunCriteria<Marker>) {
        self.run_criteria = run_criteria.into_run_criteria();
    }

    #[must_use]
    pub fn with_run_criteria<Marker>(mut self, run_criteria: impl IntoRunCriteria<Marker>) -> Self {
        self.set_run_criteria(run_criteria);
        self
    }

    pub fn apply_buffers(&mut self, world: &mut World) {
        for container in self.parallel_systems.iter_mut() {
            #[cfg(feature = "tracing")]
            let _guard = tracing::info_span!("apply", name = container.name()).entered();

            container.system_mut().apply(world);
        }
    }

    #[inline]
    pub fn parallel_systems(&self) -> &[SystemContainer] {
        &self.parallel_systems
    }

    fn validate_world(&mut self, world: &mut World) {
        if let Some(ref mut world_id) = self.world_id {
            if *world_id != world.id() {
                *world_id = world.id();

                self.reinitialize_systems(world);
            }
        } else {
            self.world_id = Some(world.id());
        }
    }

    fn initialize_systems(&mut self, world: &mut World) {
        for index in self.uninitialized_exclusive.drain(..) {
            let container = &mut self.exclusive_systems[index];
            container.system_mut().init(world);
        }

        for index in self.uninitialized_parallel.drain(..) {
            let container = &mut self.parallel_systems[index];
            container.system_mut().init(world);
        }
    }

    fn reinitialize_systems(&mut self, world: &mut World) {
        for container in self.exclusive_systems.iter_mut() {
            container.system_mut().init(world);
        }

        for container in self.parallel_systems.iter_mut() {
            container.system_mut().init(world);
        }

        self.uninitialized_exclusive.clear();
        self.uninitialized_parallel.clear();
    }

    fn check_change_ticks(&mut self, world: &World) {
        let change_tick = world.change_tick();

        for container in self.exclusive_systems.iter_mut() {
            container.system_mut().check_change_tick(change_tick);
        }

        for container in self.parallel_systems.iter_mut() {
            container.system_mut().check_change_tick(change_tick);
        }
    }

    fn rebuild_systems(&mut self) {
        Self::rebuild_dependency_graph(&mut self.parallel_systems);
        Self::rebuild_dependency_graph(&mut self.exclusive_systems);
    }

    fn rebuild_dependency_graph(systems: &mut Vec<SystemContainer>) {
        let mut labels = HashMap::<SystemLabelId, Vec<usize>>::default();

        for (index, container) in systems.iter().enumerate() {
            for &label in container.labels() {
                labels.entry(label).or_default().push(index);
            }
        }

        let mut graph = HashMap::<usize, HashSet<usize>>::default();

        for (index, container) in systems.iter().enumerate() {
            let dependencies = graph.entry(index).or_default();

            for &label in container.after() {
                for &dependency in labels.get(&label).unwrap_or(&Vec::new()) {
                    dependencies.insert(dependency);
                }
            }

            for &label in container.before() {
                for &dependant in labels.get(&label).unwrap_or(&Vec::new()) {
                    graph.entry(dependant).or_default().insert(index);
                }
            }
        }

        fn visit(
            node: usize,
            graph: &HashMap<usize, HashSet<usize>>,
            sorted: &mut Vec<usize>,
            current: &mut Vec<usize>,
            unvisited: &mut HashSet<usize>,
        ) -> bool {
            if current.contains(&node) {
                return true;
            } else if !unvisited.remove(&node) {
                return false;
            }

            current.push(node);

            for &dependency in graph.get(&node).unwrap() {
                if visit(dependency, graph, sorted, current, unvisited) {
                    return true;
                }
            }

            sorted.push(node);
            current.pop();

            false
        }

        let mut sorted = Vec::with_capacity(graph.len());
        let mut current = Vec::with_capacity(graph.len());
        let mut unvisited = graph.keys().copied().collect::<HashSet<_>>();

        while let Some(index) = unvisited.iter().next().copied() {
            if visit(index, &graph, &mut sorted, &mut current, &mut unvisited) {
                let names = current
                    .iter()
                    .map(|&index| systems[index].meta().name())
                    .collect::<Vec<_>>()
                    .join(", ");

                panic!(
                    "SystemStage contains a dependency cycle between systems: {}",
                    names
                );
            }
        }

        for (index, system) in systems.iter_mut().enumerate() {
            system.dependencies_mut().clear();

            for &dependency in graph.get(&index).unwrap() {
                let dependency = sorted.iter().position(|&i| i == dependency).unwrap();
                system.dependencies_mut().push(dependency);
            }
        }

        let mut temp = systems.drain(..).map(Some).collect::<Vec<_>>();

        for index in sorted {
            systems.push(temp[index].take().unwrap());
        }
    }
}

impl Stage for SystemStage {
    fn run(&mut self, world: &mut World) {
        self.validate_world(world);

        match self.run_criteria.should_run(world) {
            ShouldRun::Yes => {}
            ShouldRun::No => return,
        }

        if self.systems_modified {
            self.systems_modified = false;
            self.executor_modified = false;

            self.initialize_systems(world);
            self.rebuild_systems();

            self.executor.systems_changed(&self.parallel_systems);
        } else if self.executor_modified {
            self.executor_modified = false;

            self.executor.systems_changed(&self.parallel_systems);
        }

        for container in self.exclusive_systems.iter_mut() {
            container.run_criteria_mut().run(world);
        }

        for container in self.parallel_systems.iter_mut() {
            container.run_criteria_mut().run(world);
        }

        // SAFETY:
        // - `world` was validated earlier
        unsafe { self.executor.run_systems(&mut self.parallel_systems, world) };

        self.apply_buffers(world);

        for container in self.exclusive_systems.iter_mut() {
            if container.should_run() {
                #[cfg(feature = "tracing")]
                let guard = tracing::info_span!("system", system = container.name()).entered();
                container.system_mut().run((), world);
                #[cfg(feature = "tracing")]
                drop(guard);

                #[cfg(feature = "tracing")]
                let _guard = tracing::info_span!("apply", system = container.name()).entered();
                container.system_mut().apply(world);
            }
        }

        self.check_change_ticks(world);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate as shiv;
    use crate::{
        query::Query,
        schedule::{IntoSystemDescriptor, SystemLabel},
        system::ResMut,
        world::World,
    };

    use super::{Stage, SystemStage};

    #[derive(SystemLabel)]
    enum TestSystem {
        A,
        B,
        C,
    }

    fn system_a(mut counter: ResMut<u32>) {
        assert_eq!(*counter, 0);
        *counter += 1;
    }

    fn system_b(mut counter: ResMut<u32>) {
        assert_eq!(*counter, 1);
        *counter += 1;
    }

    fn system_c(mut counter: ResMut<u32>) {
        assert_eq!(*counter, 2);
        *counter += 1;
    }

    #[test]
    fn run_before() {
        let mut world = World::new();
        world.init_resource::<u32>();

        let mut stage = SystemStage::sequential();
        stage.add_system(system_b.label(TestSystem::B));
        stage.add_system(system_a.label(TestSystem::A).before(TestSystem::B));

        stage.run(&mut world);
    }

    #[test]
    fn run_after() {
        let mut world = World::new();
        world.init_resource::<u32>();

        let mut stage = SystemStage::sequential();
        stage.add_system(system_b.label(TestSystem::B).after(TestSystem::A));
        stage.add_system(system_a.label(TestSystem::A));

        stage.run(&mut world);
    }

    #[test]
    fn run_ordered() {
        let mut world = World::new();
        world.init_resource::<u32>();

        let mut stage = SystemStage::sequential();
        stage.add_system(
            system_b
                .label(TestSystem::B)
                .before(TestSystem::C)
                .after(TestSystem::A),
        );
        stage.add_system(
            system_c
                .label(TestSystem::C)
                .after(TestSystem::B)
                .after(TestSystem::A),
        );
        stage.add_system(
            system_a
                .label(TestSystem::A)
                .before(TestSystem::B)
                .before(TestSystem::C),
        );

        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn fail_cycle() {
        let mut world = World::new();
        world.init_resource::<u32>();

        let mut stage = SystemStage::sequential();
        stage.add_system(system_a.label(TestSystem::A).before(TestSystem::B));
        stage.add_system(system_b.label(TestSystem::B).before(TestSystem::A));

        stage.run(&mut world);
    }

    #[test]
    fn parallel() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        fn read(query: Query<&i32>) {
            assert!(
                COUNTER.fetch_add(1, Ordering::SeqCst) < usize::MAX,
                "read running at the same time as write",
            );

            for i in query.iter() {
                let _ = *i;
            }

            assert!(COUNTER.fetch_sub(1, Ordering::SeqCst) > 0);
        }

        fn write(mut query: Query<&mut i32>) {
            assert_eq!(
                COUNTER.swap(usize::MAX, Ordering::SeqCst),
                0,
                "write wasn't executed exclusively"
            );

            for mut i in query.iter_mut() {
                *i += 1;
            }

            assert_eq!(
                COUNTER.swap(0, Ordering::SeqCst),
                usize::MAX,
                "write wasn't executed exclusively",
            );
        }

        let mut world = World::new();
        let mut stage = SystemStage::parallel();

        for i in 0..100 {
            world.spawn().insert(i);
        }

        stage.add_system(read);
        stage.add_system(write);
        stage.add_system(read);
        stage.add_system(write);

        stage.run(&mut world);
    }
}

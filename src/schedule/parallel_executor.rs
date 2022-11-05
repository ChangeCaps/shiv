use async_channel::{Receiver, Sender};
use event_listener::Event;
use fixedbitset::FixedBitSet;
use hyena::{Scope, TaskPool};

use crate::{Access, ComponentId, SystemContainer, SystemExecutor, World};

#[derive(Debug)]
struct ParallelSystemMeta {
    start: Event,
    dependants: Vec<usize>,
    dependencies_total: usize,
    dependencies_remaining: usize,
    access: Access<ComponentId>,
}

#[derive(Debug)]
pub struct ParallelExecutor {
    system_meta: Vec<ParallelSystemMeta>,
    finished_sender: Sender<usize>,
    finished_receiver: Receiver<usize>,
    queued: FixedBitSet,
    running: FixedBitSet,
    current_access: Access<ComponentId>,
    task_pool: TaskPool,
}

impl Default for ParallelExecutor {
    #[inline]
    fn default() -> Self {
        let (finished_sender, finished_receiver) = async_channel::unbounded();

        Self {
            system_meta: Vec::new(),
            finished_sender,
            finished_receiver,
            queued: FixedBitSet::new(),
            running: FixedBitSet::new(),
            current_access: Access::default(),
            task_pool: TaskPool::new().expect("Failed to create task pool"),
        }
    }
}

impl ParallelExecutor {
    #[inline]
    fn queued_count(&self) -> usize {
        self.queued.count_ones(..)
    }

    #[inline]
    fn running_count(&self) -> usize {
        self.running.count_ones(..)
    }

    #[inline]
    fn prepare_systems<'a>(
        &mut self,
        scope: &Scope<'_, 'a, ()>,
        systems: &'a mut [SystemContainer],
        world: &'a World,
    ) {
        for (index, (meta, system)) in self.system_meta.iter_mut().zip(systems).enumerate() {
            if !system.should_run() {
                continue;
            }

            let dependencies_run = meta.dependencies_total == 0;
            let access_compatible = meta.access.is_compatible(&self.current_access);
            let can_run = dependencies_run && access_compatible;

            if meta.dependencies_total > 0 {
                meta.dependencies_remaining = meta.dependencies_total;
            }

            if dependencies_run && !access_compatible {
                self.queued.set(index, true);
            }

            let finished_sender = self.finished_sender.clone();
            if can_run {
                let task = async move {
                    unsafe { system.system_mut().run((), world) };
                    finished_sender.send(index).await.unwrap();
                };

                scope.spawn(task);

                self.running.insert(index);
                self.current_access.extend(&meta.access);
            } else {
                let start = meta.start.listen();
                let task = async move {
                    start.await;
                    unsafe { system.system_mut().run((), world) };
                    finished_sender.send(index).await.unwrap();
                };

                scope.spawn(task);
            }
        }
    }

    #[inline]
    fn process_finished_system(&mut self, index: usize) {
        let meta = &self.system_meta[index];
        self.running.set(index, false);

        for dependant in meta.dependants.clone() {
            let dependant_meta = &mut self.system_meta[dependant];
            dependant_meta.dependencies_remaining -= 1;

            if dependant_meta.dependencies_remaining == 0 {
                self.queued.insert(dependant);
            }
        }
    }

    #[inline]
    fn run_queued_systems(&mut self) {
        for index in self.queued.clone().ones() {
            let meta = &self.system_meta[index];

            if meta.access.is_compatible(&self.current_access) {
                self.queued.set(index, false);
                self.running.set(index, true);
                self.current_access.extend(&meta.access);
                meta.start.notify(1);
            }
        }
    }

    #[inline]
    fn rebuild_access(&mut self) {
        self.current_access.clear();

        for index in self.running.ones() {
            let meta = &self.system_meta[index];
            self.current_access.extend(&meta.access);
        }
    }
}

impl SystemExecutor for ParallelExecutor {
    fn systems_changed(&mut self, systems: &[SystemContainer]) {
        self.system_meta.clear();

        self.queued.grow(systems.len());
        self.running.grow(systems.len());

        for container in systems {
            let dependencies_total = container.dependencies().len();
            let meta = container.meta();
            let system_meta = ParallelSystemMeta {
                start: Event::new(),
                dependants: Vec::new(),
                dependencies_total,
                dependencies_remaining: 0,
                access: meta.access.clone(),
            };
            self.system_meta.push(system_meta);
        }

        for (dependant, container) in systems.iter().enumerate() {
            for &dependency in container.dependencies() {
                self.system_meta[dependency].dependants.push(dependant);
            }
        }
    }

    unsafe fn run_systems(&mut self, systems: &mut [SystemContainer], world: &mut World) {
        self.task_pool.clone().scope(|scope| {
            let executor = async {
                self.prepare_systems(scope, systems, world);

                while self.queued_count() + self.running_count() > 0 {
                    if self.running_count() > 0 {
                        let index = self.finished_receiver.recv().await.unwrap();
                        self.process_finished_system(index);

                        while let Ok(index) = self.finished_receiver.try_recv() {
                            self.process_finished_system(index);
                        }

                        self.rebuild_access();
                    }

                    self.run_queued_systems();
                }
            };

            scope.spawn(executor);
        });
    }
}

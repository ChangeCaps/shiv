use downcast_rs::{impl_downcast, Downcast};

use crate::world::World;

use super::{ParallelExecutor, SystemContainer};

pub trait SystemExecutor: Downcast + Send + Sync {
    #[inline]
    fn systems_changed(&mut self, _systems: &[SystemContainer]) {}

    /// # Safety
    /// - `world` must be the same world that each `system` was initialized with.
    unsafe fn run_systems(&mut self, systems: &mut [SystemContainer], world: &mut World);
}

impl std::fmt::Debug for dyn SystemExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(sequential_executor) = self.downcast_ref::<SequentialExecutor>() {
            write!(f, "{sequential_executor:?}")
        } else if let Some(parallel_executor) = self.downcast_ref::<ParallelExecutor>() {
            write!(f, "{parallel_executor:?}")
        } else {
            write!(f, "{{Custom SystemExecutor}}")
        }
    }
}

impl_downcast!(SystemExecutor);

#[derive(Clone, Copy, Debug, Default)]
pub struct SequentialExecutor;

impl SystemExecutor for SequentialExecutor {
    #[inline]
    unsafe fn run_systems(&mut self, systems: &mut [SystemContainer], world: &mut World) {
        for system in systems {
            if system.should_run() {
                #[cfg(feature = "tracing")]
                let _ = tracing::info_span!("system", name = system.name()).entered();

                // SAFETY:
                // - we know that systems are run sequentially,
                // so no two systems will be run at the same time
                // - we know that `world` is the same world that
                // each system was initialized with (see `SystemExecutor::run_systems`)
                unsafe { system.system_mut().run_unchecked((), world) };
            }
        }
    }
}

use crate::system::{BoxedSystem, System, SystemMeta};

use super::{SystemDescriptor, SystemLabelId};

pub struct SystemContainer {
    system: BoxedSystem<(), ()>,
    labels: Vec<SystemLabelId>,
    before: Vec<SystemLabelId>,
    after: Vec<SystemLabelId>,
    dependencies: Vec<usize>,
}

impl SystemContainer {
    #[inline]
    pub fn from_descriptor(descriptor: SystemDescriptor) -> Self {
        Self {
            system: descriptor.system,
            labels: descriptor.labels,
            before: descriptor.before,
            after: descriptor.after,
            dependencies: Vec::new(),
        }
    }

    #[inline]
    pub fn meta(&self) -> &SystemMeta {
        self.system.meta()
    }

    #[inline]
    pub fn system(&self) -> &dyn System<In = (), Out = ()> {
        self.system.as_ref()
    }

    #[inline]
    pub fn system_mut(&mut self) -> &mut dyn System<In = (), Out = ()> {
        self.system.as_mut()
    }

    #[inline]
    pub fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }

    #[inline]
    pub fn dependencies_mut(&mut self) -> &mut Vec<usize> {
        &mut self.dependencies
    }

    #[inline]
    pub fn should_run(&self) -> bool {
        true
    }

    #[inline]
    pub fn labels(&self) -> &[SystemLabelId] {
        &self.labels
    }

    #[inline]
    pub fn before(&self) -> &[SystemLabelId] {
        &self.before
    }

    #[inline]
    pub fn after(&self) -> &[SystemLabelId] {
        &self.after
    }
}

impl std::fmt::Debug for SystemContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}}}", self.system.meta().name())
    }
}

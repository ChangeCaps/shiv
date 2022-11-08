use crate::system::{BoxedSystem, IntoSystem, System};

use super::{IntoRunCriteria, RunCriteria, SystemLabel, SystemLabelId};

pub struct SystemDescriptor {
    pub system: BoxedSystem<(), ()>,
    pub labels: Vec<SystemLabelId>,
    pub before: Vec<SystemLabelId>,
    pub after: Vec<SystemLabelId>,
    pub run_criteria: RunCriteria,
}

impl SystemDescriptor {
    pub fn new<S>(system: S) -> Self
    where
        S: System<In = (), Out = ()>,
    {
        Self {
            system: Box::new(system),
            labels: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            run_criteria: RunCriteria::default(),
        }
    }
}

pub trait IntoSystemDescriptor<Params = ()> {
    fn into_descriptor(self) -> SystemDescriptor;

    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor;

    fn label(self, label: impl SystemLabel) -> SystemDescriptor;

    fn before(self, label: impl SystemLabel) -> SystemDescriptor;

    fn after(self, label: impl SystemLabel) -> SystemDescriptor;
}

impl<S, Params> IntoSystemDescriptor<Params> for S
where
    S: IntoSystem<(), (), Params>,
{
    #[inline]
    fn into_descriptor(self) -> SystemDescriptor {
        SystemDescriptor {
            system: Box::new(self.into_system()),
            labels: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            run_criteria: RunCriteria::default(),
        }
    }

    #[inline]
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        let mut descriptor = self.into_descriptor();
        descriptor.run_criteria = run_criteria.into_run_criteria();
        descriptor
    }

    #[inline]
    fn label(self, label: impl SystemLabel) -> SystemDescriptor {
        let mut descriptor = self.into_descriptor();
        descriptor.labels.push(label.label());
        descriptor
    }

    #[inline]
    fn before(self, label: impl SystemLabel) -> SystemDescriptor {
        let mut descriptor = self.into_descriptor();
        descriptor.before.push(label.label());
        descriptor
    }

    #[inline]
    fn after(self, label: impl SystemLabel) -> SystemDescriptor {
        let mut descriptor = self.into_descriptor();
        descriptor.after.push(label.label());
        descriptor
    }
}

impl IntoSystemDescriptor for SystemDescriptor {
    #[inline]
    fn into_descriptor(self) -> SystemDescriptor {
        self
    }

    #[inline]
    fn with_run_criteria<Marker>(
        mut self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        self.run_criteria = run_criteria.into_run_criteria();
        self
    }

    #[inline]
    fn label(mut self, label: impl SystemLabel) -> SystemDescriptor {
        self.labels.push(label.label());
        self
    }

    #[inline]
    fn before(mut self, label: impl SystemLabel) -> SystemDescriptor {
        self.before.push(label.label());
        self
    }

    #[inline]
    fn after(mut self, label: impl SystemLabel) -> SystemDescriptor {
        self.after.push(label.label());
        self
    }
}

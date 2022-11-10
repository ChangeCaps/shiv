use crate::{
    system::{BoxedSystem, IntoSystem, Local},
    world::{World, WorldId},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShouldRun {
    Yes,
    No,
}

impl ShouldRun {
    pub fn once(mut has_run: Local<bool>) -> Self {
        if *has_run {
            Self::No
        } else {
            *has_run = true;
            Self::Yes
        }
    }
}

impl Into<bool> for ShouldRun {
    #[inline]
    fn into(self) -> bool {
        match self {
            Self::Yes => true,
            Self::No => false,
        }
    }
}

#[derive(Debug, Default)]
pub struct RunCriteria {
    criteria: Option<BoxedSystem<(), ShouldRun>>,
    world_id: Option<WorldId>,
}

impl RunCriteria {
    #[inline]
    pub fn should_run(&mut self, world: &mut World) -> ShouldRun {
        if let Some(ref mut criteria) = self.criteria {
            match self.world_id {
                Some(ref mut world_id) => {
                    if *world_id != world.id() {
                        *world_id = world.id();

                        criteria.init(world);
                    }
                }
                None => {
                    criteria.init(world);
                    self.world_id = Some(world.id());
                }
            }

            // SAFETY:
            // - world has been validated above
            // - world is borrowed mutably
            unsafe { criteria.run_unchecked((), world) }
        } else {
            ShouldRun::Yes
        }
    }
}

#[derive(Debug)]
pub struct RunCriteriaContainer {
    pub(crate) should_run: ShouldRun,
    pub(crate) criteria: RunCriteria,
}

impl RunCriteriaContainer {
    #[inline]
    pub fn new(criteria: RunCriteria) -> Self {
        Self {
            should_run: ShouldRun::Yes,
            criteria,
        }
    }

    #[inline]
    pub fn run(&mut self, world: &mut World) {
        self.should_run = self.criteria.should_run(world);
    }

    #[inline]
    pub fn should_run(&self) -> ShouldRun {
        self.should_run
    }
}

pub trait IntoRunCriteria<Marker> {
    fn into_run_criteria(self) -> RunCriteria;
}

impl<S, Param> IntoRunCriteria<(BoxedSystem<(), ShouldRun>, Param)> for S
where
    S: IntoSystem<(), ShouldRun, Param>,
{
    fn into_run_criteria(self) -> RunCriteria {
        RunCriteria {
            criteria: Some(Box::new(self.into_system())),
            world_id: None,
        }
    }
}

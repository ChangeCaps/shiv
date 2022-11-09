use crate::world::World;

use super::{IntoSystem, System, SystemMeta};

#[derive(Debug)]
pub struct PipeSystem<A, B> {
    system_a: A,
    system_b: B,
    meta: SystemMeta,
}

impl<A, B> System for PipeSystem<A, B>
where
    A: System,
    B: System<In = A::Out>,
{
    type In = A::In;
    type Out = B::Out;

    fn meta(&self) -> &SystemMeta {
        &self.meta
    }

    unsafe fn meta_mut(&mut self) -> &mut SystemMeta {
        &mut self.meta
    }

    fn init(&mut self, world: &mut World) {
        self.system_a.init(world);
        self.system_b.init(world);
    }

    unsafe fn run(&mut self, input: Self::In, world: &World) -> Self::Out {
        let payload = unsafe { self.system_a.run(input, world) };
        let out = unsafe { self.system_b.run(payload, world) };

        self.meta.last_change_tick = self.system_a.meta().last_change_tick;

        out
    }

    fn apply(&mut self, world: &mut World) {
        self.system_a.apply(world);
        self.system_b.apply(world);
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        self.system_a.check_change_tick(change_tick);
        self.system_b.check_change_tick(change_tick);
    }

    fn set_last_change_tick(&mut self, last_change_tick: u32) {
        self.system_a.set_last_change_tick(last_change_tick);
        self.system_b.set_last_change_tick(last_change_tick);
    }
}

pub trait IntoPipeSystem<In, Payload, Out, ParamsA, ParamsB, System>:
    IntoSystem<In, Payload, ParamsA>
where
    System: IntoSystem<Payload, Out, ParamsB>,
{
    fn pipe(self, system: System) -> PipeSystem<Self::System, System::System>;
}

impl<In, Payload, Out, ParamsA, ParamsB, SystemA, SystemB>
    IntoPipeSystem<In, Payload, Out, ParamsA, ParamsB, SystemB> for SystemA
where
    SystemA: IntoSystem<In, Payload, ParamsA>,
    SystemB: IntoSystem<Payload, Out, ParamsB>,
{
    fn pipe(self, system: SystemB) -> PipeSystem<Self::System, SystemB::System> {
        let system_a = self.into_system();
        let system_b = system.into_system();

        let mut access = system_a.meta().access.clone();
        access.extend(&system_b.meta().access);

        let meta = SystemMeta {
            name: format!("{} | {}", system_a.meta().name, system_b.meta().name).into(),
            access,
            last_change_tick: 0,
        };

        PipeSystem {
            system_a,
            system_b,
            meta,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        system::{In, IntoPipeSystem, System},
        world::World,
    };

    #[test]
    fn pipe_system() {
        fn a() -> u32 {
            3
        }

        fn b(input: In<u32>) -> f32 {
            (*input * 3) as f32
        }

        let mut world = World::new();
        let mut system = a.pipe(b);

        system.init(&mut world);
        let out = unsafe { system.run((), &world) };
        system.apply(&mut world);

        assert_eq!(out, 9.0);
    }
}

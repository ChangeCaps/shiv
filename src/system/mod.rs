mod access;
mod command;
mod exclusive;
mod exclusive_param;
mod function;
mod param;
mod system;
mod system_piping;

pub use access::*;
pub use command::*;
pub use exclusive::*;
pub use exclusive_param::*;
pub use function::*;
pub use param::*;
pub use system::*;
pub use system_piping::*;

#[cfg(test)]
mod tests {
    use crate::{
        query::Query,
        system::{IntoSystem, Res, ResMut, System},
        world::World,
    };

    #[test]
    #[should_panic]
    fn conflicting_resources() {
        fn system(_a: Res<i32>, _b: ResMut<i32>) {}

        let mut world = World::new();

        let mut system = system.into_system();
        system.init(&mut world);
    }

    #[test]
    #[should_panic]
    fn conflicting_queries() {
        fn system(_a: Query<&i32>, _b: Query<&mut i32>) {}

        let mut world = World::new();

        let mut system = system.into_system();
        system.init(&mut world);
    }
}

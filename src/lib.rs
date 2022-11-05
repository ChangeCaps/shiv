#![deny(unsafe_op_in_unsafe_fn)]

//! A simple modern Entity Component System (ECS).

mod change_ticks;
mod hash_map;
mod query;
mod schedule;
mod storage;
mod system;
mod world;

pub use change_ticks::*;
pub use query::*;
pub use schedule::*;
pub use storage::*;
pub use system::*;
pub use world::*;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate as termite;
    use crate::*;

    #[derive(StageLabel)]
    enum TestStage {
        A,
        B,
        C,
    }

    fn spawn_system(mut commands: Commands, mut entities: ResMut<HashMap<Entity, i32>>) {
        entities.clear();

        for i in 0..10 {
            let entity = commands.spawn().insert(i).entity();

            entities.insert(entity, i);
        }
    }

    fn despawn_system(mut commands: Commands, query: Query<Entity, With<i32>>) {
        for entity in query.iter() {
            eprintln!("{}", entity);
            commands.entity(entity).despawn();
        }
    }

    fn increment_system(mut query: Query<&mut i32>) {
        for mut item in query.iter_mut() {
            *item += 1;
        }
    }

    fn detect_added_system(added: Query<&i32, Added<i32>>) {
        if added.is_empty() {
            panic!("Added not detected");
        }
    }

    fn detect_changed_system(query: Query<&i32, Changed<i32>>) {
        if query.is_empty() {
            panic!("Changed not detected");
        }

        for item in query.iter() {
            assert_eq!(*item, 3);
        }
    }

    fn default_schedule() -> Schedule {
        Schedule::new()
            .with_stage(TestStage::A, SystemStage::sequential())
            .with_stage(TestStage::B, SystemStage::sequential())
            .with_stage(TestStage::C, SystemStage::sequential())
    }

    #[test]
    fn spawn_systems() {
        let mut world = World::new();
        world.init_resource::<HashMap<Entity, i32>>();

        let mut schedule = default_schedule();

        schedule.add_system_to_stage(TestStage::A, spawn_system);

        schedule.run_once(&mut world);

        let query = world.query::<&i32>();
        let entities = world.resource::<HashMap<Entity, i32>>();

        for (&entity, i) in entities.iter() {
            assert_eq!(query.get(&world, entity).unwrap(), i);
        }
    }

    #[test]
    fn respawn_systems() {
        let mut world = World::new();
        world.init_resource::<HashMap<Entity, i32>>();

        let mut schedule = default_schedule();

        schedule.add_system_to_stage(TestStage::A, spawn_system);
        schedule.add_system_to_stage(TestStage::B, despawn_system);
        schedule.add_system_to_stage(TestStage::C, spawn_system);

        schedule.run_once(&mut world);

        let query = world.query::<&i32>();
        let entities = world.resource::<HashMap<Entity, i32>>();

        for (&entity, i) in entities.iter() {
            assert_eq!(query.get(&world, entity).unwrap(), i);
        }
    }

    #[test]
    fn change_detection() {
        let mut world = World::new();
        let mut schedule = default_schedule();

        world.spawn().insert(2);

        schedule.add_system_to_stage(TestStage::A, increment_system);
        schedule.add_system_to_stage(TestStage::B, detect_added_system);
        schedule.add_system_to_stage(TestStage::B, detect_changed_system);

        schedule.run_once(&mut world);
    }
}

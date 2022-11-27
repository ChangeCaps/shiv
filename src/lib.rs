#![deny(unsafe_op_in_unsafe_fn)]

//! A simple modern Entity Component System (ECS).

pub mod bundle;
pub mod change_detection;
pub mod event;
pub mod hash_map;
#[cfg(feature = "hierarchy")]
pub mod hierarchy;
pub mod query;
pub mod schedule;
pub mod storage;
pub mod system;
pub mod world;

pub mod tasks {
    //! A re-export of [`hyena`].

    pub use hyena::*;
}

pub mod prelude {
    //! `use shiv::prelude::*;` imports the most commonly used types and traits.

    pub use crate::bundle::Bundle;
    pub use crate::change_detection::Mut;
    pub use crate::event::{Event, EventId, EventReader, EventWriter, Events};
    #[cfg(feature = "hierarchy")]
    pub use crate::hierarchy::{Children, Parent};
    pub use crate::query::{Added, Changed, Or, Query, QueryIter, QueryState, With, Without};
    pub use crate::schedule::{
        DefaultStage, IntoSystemDescriptor, Schedule, Stage, StageLabel, SystemLabel, SystemStage,
    };
    pub use crate::storage::{DenseStorage, Resource};
    pub use crate::system::{
        Command, Commands, EntityCommands, IntoPipeSystem, Local, Res, ResInit, ResMut, ResMutInit,
        SystemParam,
    };
    pub use crate::world::{Component, Entity, EntityMut, EntityRef, FromWorld, World};
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate as shiv;
    use crate::query::Or;
    use crate::{
        query::{Added, Changed, Query, With},
        schedule::{IntoSystemDescriptor, Schedule, ShouldRun, StageLabel, SystemStage},
        system::{Commands, Local, ResMut},
        world::{Entity, World},
    };

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

        let entity = world.spawn().insert(2).entity();

        schedule.add_system_to_stage(
            TestStage::B,
            detect_added_system.with_run_criteria(ShouldRun::once),
        );
        schedule.add_system_to_stage(TestStage::B, detect_changed_system);
        schedule.run_once(&mut world);

        world.entity_mut(entity).insert(3);
        schedule.run_once(&mut world);

        schedule.add_system_to_stage(TestStage::A, increment_system);
        schedule.run_once(&mut world);
    }

    #[test]
    fn run_criteria() {
        fn only_once(mut has_run: Local<bool>) {
            if *has_run {
                panic!("System ran twice");
            }

            *has_run = true;
        }

        let mut world = World::new();
        let mut schedule = default_schedule();

        schedule.add_system_to_stage(TestStage::A, only_once.with_run_criteria(ShouldRun::once));

        schedule.run_once(&mut world);
        schedule.run_once(&mut world);
    }

    #[test]
    fn different_worlds() {
        let mut world_a = World::new();
        let mut world_b = World::new();

        world_a.init_resource::<HashMap<Entity, i32>>();
        world_b.init_resource::<HashMap<Entity, i32>>();

        let mut schedule = default_schedule();

        schedule.add_system_to_stage(TestStage::A, spawn_system);
        schedule.add_system_to_stage(TestStage::B, despawn_system);
        schedule.add_system_to_stage(TestStage::C, spawn_system);

        schedule.run_once(&mut world_a);
        schedule.run_once(&mut world_b);
    }

    #[test]
    fn or_filter() {
        let mut world = World::new();

        world.spawn().insert(2i32).entity();
        world.spawn().insert(3i32).insert(false).entity();
        world.spawn().insert(0.4f32).entity();

        let mut schedule = default_schedule();

        fn system(query: Query<(&i32, Changed<bool>), Or<(Changed<bool>, Changed<f32>)>>) {
            for _ in query.iter() {}
        }

        schedule.add_system_to_stage(TestStage::A, system);

        schedule.run_once(&mut world);
        schedule.run_once(&mut world);
    }
}

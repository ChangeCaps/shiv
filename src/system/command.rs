use std::marker::PhantomData;

use crate::{Component, Entity, World};

#[derive(Default)]
pub struct CommandQueue {
    queue: Vec<Box<dyn Command>>,
}

impl CommandQueue {
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        world.flush();

        for command in self.queue.drain(..) {
            command.apply(world);
        }
    }
}

pub trait Command: Send + Sync + 'static {
    fn apply(self: Box<Self>, world: &mut World);
}

pub struct Commands<'w, 's> {
    queue: &'s mut CommandQueue,
    world: &'w World,
}

impl<'w, 's> Commands<'w, 's> {
    #[inline]
    pub fn new(queue: &'s mut CommandQueue, world: &'w World) -> Self {
        Self { queue, world }
    }

    #[inline]
    pub fn world(&self) -> &'w World {
        self.world
    }

    #[inline]
    pub fn spawn<'a>(&'a mut self) -> EntityCommands<'w, 's, 'a> {
        let entity = self.world.reserve_entity();
        EntityCommands::new(self, entity)
    }

    #[inline]
    pub fn get_or_spawn<'a>(&'a mut self, entity: Entity) -> EntityCommands<'w, 's, 'a> {
        self.add(GetOrSpawn { entity });
        EntityCommands::new(self, entity)
    }

    #[inline]
    pub fn get_entity<'a>(&'a mut self, entity: Entity) -> Option<EntityCommands<'w, 's, 'a>> {
        if self.world().contains_entity(entity) {
            Some(EntityCommands::new(self, entity))
        } else {
            None
        }
    }

    #[inline]
    #[track_caller]
    pub fn entity<'a>(&'a mut self, entity: Entity) -> EntityCommands<'w, 's, 'a> {
        self.get_entity(entity).unwrap_or_else(|| {
            panic!(
                "Attempting to create an EntityCommands for entity {}, which doesn't exist.",
                entity
            )
        })
    }

    #[inline]
    pub fn add(&mut self, command: impl Command) {
        self.queue.queue.push(Box::new(command));
    }
}

pub struct EntityCommands<'w, 's, 'a> {
    commands: &'a mut Commands<'w, 's>,
    entity: Entity,
}

impl<'w, 's, 'a> EntityCommands<'w, 's, 'a> {
    #[inline]
    fn new(commands: &'a mut Commands<'w, 's>, entity: Entity) -> Self {
        Self { commands, entity }
    }

    #[inline]
    pub fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    pub fn insert<T: Component>(&mut self, component: T) -> &mut Self {
        self.commands.add(Insert {
            entity: self.entity,
            component,
        });

        self
    }

    #[inline]
    pub fn remove<T: Component>(&mut self) -> &mut Self {
        self.commands.add(Remove {
            entity: self.entity,
            marker: PhantomData::<T>,
        });

        self
    }

    #[inline]
    pub fn despawn(&mut self) {
        self.commands.add(Despawn {
            entity: self.entity,
        });
    }
}

pub struct Insert<T> {
    pub entity: Entity,
    pub component: T,
}

impl<T: Component> Command for Insert<T> {
    fn apply(self: Box<Self>, world: &mut World) {
        world.entity_mut(self.entity).insert(self.component);
    }
}

pub struct Remove<T> {
    pub entity: Entity,
    pub marker: PhantomData<T>,
}

impl<T: Component> Command for Remove<T> {
    fn apply(self: Box<Self>, world: &mut World) {
        world.entity_mut(self.entity).remove::<T>();
    }
}

pub struct Despawn {
    pub entity: Entity,
}

impl Command for Despawn {
    fn apply(self: Box<Self>, world: &mut World) {
        world.despawn(self.entity);
    }
}

pub struct GetOrSpawn {
    entity: Entity,
}

impl Command for GetOrSpawn {
    fn apply(self: Box<Self>, world: &mut World) {
        world.get_or_spawn(self.entity);
    }
}

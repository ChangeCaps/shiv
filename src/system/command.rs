use std::marker::PhantomData;

use crate::{
    bundle::Bundle,
    storage::Resource,
    world::{Entity, FromWorld, World},
};

#[derive(Debug, Default)]
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

    #[inline]
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

impl std::fmt::Debug for dyn Command {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{command:{}}}", self.name())
    }
}

#[derive(Debug)]
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
    pub fn add_command(&mut self, command: impl Command) -> &mut Self {
        self.queue.queue.push(Box::new(command));
        self
    }

    #[inline]
    pub fn spawn<'a>(&'a mut self) -> EntityCommands<'w, 's, 'a> {
        let entity = self.world.reserve_entity();
        EntityCommands::new(self, entity)
    }

    #[inline]
    pub fn get_or_spawn<'a>(&'a mut self, entity: Entity) -> EntityCommands<'w, 's, 'a> {
        self.add_command(GetOrSpawn { entity });
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
    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.add_command(InsertResource { resource });
    }

    pub fn init_resource<T: Resource + FromWorld>(&mut self) {
        self.add_command(InitResource {
            marker: PhantomData::<T>,
        });
    }

    #[inline]
    pub fn remove_resource<T: Resource>(&mut self) {
        self.add_command(RemoveResource {
            marker: PhantomData::<T>,
        });
    }
}

#[derive(Debug)]
pub struct EntityCommands<'w, 's, 'a> {
    pub(crate) commands: &'a mut Commands<'w, 's>,
    pub(crate) entity: Entity,
}

impl<'w, 's, 'a> EntityCommands<'w, 's, 'a> {
    #[inline]
    fn new(commands: &'a mut Commands<'w, 's>, entity: Entity) -> Self {
        Self { commands, entity }
    }

    #[inline]
    pub fn commands(&mut self) -> &mut Commands<'w, 's> {
        self.commands
    }

    #[inline]
    pub fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    pub fn add_command(&mut self, command: impl Command) -> &mut Self {
        self.commands.add_command(command);
        self
    }

    #[inline]
    pub fn insert<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        self.commands.add_command(Insert {
            entity: self.entity,
            bundle,
        });

        self
    }

    #[inline]
    pub fn remove<T: Bundle>(&mut self) -> &mut Self {
        self.commands.add_command(Remove {
            entity: self.entity,
            marker: PhantomData::<T>,
        });

        self
    }

    #[inline]
    pub fn despawn(&mut self) {
        self.commands.add_command(Despawn {
            entity: self.entity,
        });
    }
}

#[derive(Debug)]
pub struct Insert<T> {
    pub entity: Entity,
    pub bundle: T,
}

impl<T: Bundle> Command for Insert<T> {
    fn apply(self: Box<Self>, world: &mut World) {
        world.entity_mut(self.entity).insert(self.bundle);
    }
}

#[derive(Debug)]
pub struct Remove<T> {
    pub entity: Entity,
    pub marker: PhantomData<T>,
}

impl<T: Bundle> Command for Remove<T> {
    fn apply(self: Box<Self>, world: &mut World) {
        world.entity_mut(self.entity).remove::<T>();
    }
}

#[derive(Debug)]
pub struct Despawn {
    pub entity: Entity,
}

impl Command for Despawn {
    fn apply(self: Box<Self>, world: &mut World) {
        world.despawn(self.entity);
    }
}

#[derive(Debug)]
pub struct GetOrSpawn {
    entity: Entity,
}

impl Command for GetOrSpawn {
    fn apply(self: Box<Self>, world: &mut World) {
        world.get_or_spawn(self.entity);
    }
}

#[derive(Debug)]
pub struct InsertResource<T> {
    resource: T,
}

impl<T: Resource> Command for InsertResource<T> {
    fn apply(self: Box<Self>, world: &mut World) {
        world.insert_resource(self.resource);
    }
}

#[derive(Debug)]
pub struct RemoveResource<T> {
    marker: PhantomData<T>,
}

impl<T: Resource> Command for RemoveResource<T> {
    fn apply(self: Box<Self>, world: &mut World) {
        world.remove_resource::<T>();
    }
}

#[derive(Debug)]
pub struct InitResource<T> {
    marker: PhantomData<T>,
}

impl<T: Resource + FromWorld> Command for InitResource<T> {
    fn apply(self: Box<Self>, world: &mut World) {
        world.init_resource::<T>();
    }
}

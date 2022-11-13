use std::{mem, ops::Deref};

use crate::{
    prelude::{Command, Commands, EntityCommands},
    storage::DenseStorage,
    world::{Component, Entity, EntityMut, World},
};

/// A reference to this components parent.
///
/// **Note:** This should not manually be removed or inserted, use [`World::set_parent`] or
/// [`EntityCommands::set_parent`] instead.
#[derive(Clone, Copy, Debug)]
pub struct Parent {
    pub(crate) entity: Entity,
}

impl Component for Parent {
    type Storage = DenseStorage;
}

impl Parent {
    #[inline]
    pub const fn entity(&self) -> Entity {
        self.entity
    }
}

/// A reference to this components children.
///
/// **Note:** This should not manually be removed or inserted, use [`World::set_parent`] or
/// [`EntityCommands::add_child`] instead.
#[derive(Clone, Debug, Default)]
pub struct Children {
    pub(crate) entities: Vec<Entity>,
}

impl Children {
    #[inline]
    pub fn remove(&mut self, entity: Entity) -> bool {
        let index = self.entities.iter().position(|e| *e == entity);

        if let Some(index) = index {
            self.entities.swap_remove(index);

            true
        } else {
            false
        }
    }
}

impl Component for Children {
    type Storage = DenseStorage;
}

impl Deref for Children {
    type Target = [Entity];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.entities
    }
}

impl World {
    #[inline]
    pub fn set_parent(&mut self, child: Entity, new_parent: Entity) {
        if !self.contains_entity(child) || !self.contains_entity(new_parent) {
            return;
        }

        if let Some(mut parent) = self.get_mut::<Parent>(child) {
            let previous = parent.entity;
            parent.entity = new_parent;

            let mut children = self
                .get_mut::<Children>(previous)
                .expect("hierarchy is corrupt");

            children.remove(child);
        } else {
            self.entity_mut(child).insert(Parent { entity: new_parent });
        }

        if let Some(mut children) = self.get_mut::<Children>(new_parent) {
            if !children.contains(&child) {
                children.entities.push(child);
            }
        } else {
            let mut children = Children::default();
            children.entities.push(child);
            self.entity_mut(new_parent).insert(children);
        }
    }

    #[inline]
    pub fn remove_parent(&mut self, child: Entity) {
        if let Some(parent) = self.remove::<Parent>(child) {
            let mut children = self
                .get_mut::<Children>(parent.entity)
                .expect("hierarchy corrupted");

            children.remove(child);
        }
    }

    #[inline]
    pub fn remove_child(&mut self, parent: Entity, child: Entity) {
        if let Some(mut children) = self.get_mut::<Children>(parent) {
            if children.remove(child) {
                self.remove::<Parent>(child);
            }
        }
    }

    #[inline]
    pub fn remove_children(&mut self, parent: Entity) {
        if let Some(mut children) = self.get_mut::<Children>(parent) {
            for child in mem::take(&mut children.entities) {
                self.remove::<Parent>(child);
            }
        }
    }

    #[inline]
    pub fn despawn_recursive(&mut self, entity: Entity) {
        if let Some(parent) = self.get::<Parent>(entity) {
            if let Some(mut children) = self.get_mut::<Children>(parent.entity) {
                children.remove(entity);
            }
        }

        self.despawn_recursive_internal(entity);
    }

    #[inline]
    pub fn despawn_children(&mut self, entity: Entity) {
        if let Some(mut children) = self.get_mut::<Children>(entity) {
            for child in mem::take(&mut children.entities) {
                self.despawn_recursive_internal(child);
            }
        }
    }

    #[inline]
    fn despawn_recursive_internal(&mut self, entity: Entity) {
        if let Some(mut children) = self.get_mut::<Children>(entity) {
            for child in mem::take(&mut children.entities) {
                self.despawn_recursive_internal(child);
            }
        }

        self.despawn(entity);
    }
}

pub struct WorldChildBuilder<'w> {
    world: &'w mut World,
    parent: Entity,
}

impl<'w> WorldChildBuilder<'w> {
    #[inline]
    pub fn spawn(&mut self) -> EntityMut<'_> {
        let mut entity = self.world.spawn();
        entity.set_parent(self.parent);
        entity
    }

    #[inline]
    pub fn parent_entity(&self) -> Entity {
        self.parent
    }
}

impl<'w> EntityMut<'w> {
    #[inline]
    pub fn add_children<T>(&mut self, f: impl FnOnce(&mut WorldChildBuilder<'_>) -> T) -> T {
        let mut builder = WorldChildBuilder {
            world: self.world,
            parent: self.entity,
        };

        f(&mut builder)
    }

    #[inline]
    pub fn with_children(&mut self, f: impl FnOnce(&mut WorldChildBuilder<'_>)) -> &mut Self {
        self.add_children(f);
        self
    }

    #[inline]
    pub fn set_parent(&mut self, new_parent: Entity) {
        self.world.set_parent(self.entity, new_parent);
    }

    #[inline]
    pub fn add_child(&mut self, child: Entity) {
        self.world.set_parent(child, self.entity);
    }

    #[inline]
    pub fn remove_parent(&mut self) {
        self.world.remove_parent(self.entity);
    }

    #[inline]
    pub fn remove_child(&mut self, child: Entity) {
        self.world.remove_child(self.entity, child);
    }

    #[inline]
    pub fn remove_children(&mut self) {
        self.world.remove_children(self.entity);
    }

    #[inline]
    pub fn despawn_recursive(&mut self) {
        self.world.despawn_recursive(self.entity);
    }

    #[inline]
    pub fn despawn_children(&mut self) {
        self.world.despawn_children(self.entity);
    }
}

pub struct SetParent {
    pub child: Entity,
    pub new_parent: Entity,
}

impl Command for SetParent {
    fn apply(self: Box<Self>, world: &mut World) {
        world.set_parent(self.child, self.new_parent);
    }
}

pub struct RemoveParent {
    pub child: Entity,
}

impl Command for RemoveParent {
    fn apply(self: Box<Self>, world: &mut World) {
        world.remove_parent(self.child);
    }
}

pub struct RemoveChild {
    pub parent: Entity,
    pub child: Entity,
}

impl Command for RemoveChild {
    fn apply(self: Box<Self>, world: &mut World) {
        world.remove_child(self.parent, self.child);
    }
}

pub struct RemoveChildren {
    pub parent: Entity,
}

impl Command for RemoveChildren {
    fn apply(self: Box<Self>, world: &mut World) {
        world.remove_children(self.parent);
    }
}

pub struct ChildBuilder<'w, 's, 'a> {
    commands: &'a mut Commands<'w, 's>,
    parent: Entity,
}

impl<'w, 's, 'a> ChildBuilder<'w, 's, 'a> {
    #[inline]
    pub fn add_command<C: Command>(&mut self, command: C) -> &mut Self {
        self.commands.add_command(command);
        self
    }

    #[inline]
    pub fn spawn(&mut self) -> EntityCommands<'w, 's, '_> {
        let mut entity = self.commands.spawn();
        entity.set_parent(self.parent);
        entity
    }

    #[inline]
    pub fn parent_entity(&self) -> Entity {
        self.parent
    }
}

impl<'w, 's, 'a> EntityCommands<'w, 's, 'a> {
    #[inline]
    pub fn add_children<T>(&mut self, f: impl FnOnce(&mut ChildBuilder<'w, 's, '_>) -> T) -> T {
        let mut builder = ChildBuilder {
            commands: self.commands,
            parent: self.entity,
        };

        f(&mut builder)
    }

    #[inline]
    pub fn with_children(&mut self, f: impl FnOnce(&mut ChildBuilder<'w, 's, '_>)) -> &mut Self {
        self.add_children(f);
        self
    }

    #[inline]
    pub fn set_parent(&mut self, new_parent: Entity) -> &mut Self {
        self.add_command(SetParent {
            child: self.entity,
            new_parent,
        })
    }

    #[inline]
    pub fn add_child(&mut self, child: Entity) -> &mut Self {
        self.add_command(SetParent {
            child,
            new_parent: self.entity,
        })
    }

    #[inline]
    pub fn remove_parent(&mut self) -> &mut Self {
        self.add_command(RemoveParent { child: self.entity })
    }

    #[inline]
    pub fn remove_child(&mut self, child: Entity) -> &mut Self {
        self.add_command(RemoveChild {
            parent: self.entity,
            child,
        })
    }

    #[inline]
    pub fn remove_children(&mut self) -> &mut Self {
        self.add_command(RemoveChildren {
            parent: self.entity,
        })
    }
}

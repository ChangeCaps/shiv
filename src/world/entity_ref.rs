use crate::{bundle::Bundle, change_detection::Mut};

use super::{Component, Entity, World};

#[derive(Clone, Copy, Debug)]
pub struct EntityRef<'w> {
    pub(crate) world: &'w World,
    pub(crate) entity: Entity,
}

impl<'w> EntityRef<'w> {
    #[inline]
    pub(crate) fn new(world: &'w World, entity: Entity) -> Self {
        Self { world, entity }
    }

    #[inline]
    pub fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    pub fn world(&self) -> &'w World {
        self.world
    }

    #[inline]
    pub fn contains<T: Component>(&self) -> bool {
        self.world.contains::<T>(self.entity)
    }

    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'w T> {
        self.world.get(self.entity)
    }
}

#[derive(Debug)]
pub struct EntityMut<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) entity: Entity,
}

impl<'w> EntityMut<'w> {
    #[inline]
    pub(crate) fn new(world: &'w mut World, entity: Entity) -> Self {
        Self { world, entity }
    }

    #[inline]
    pub fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    pub fn world(&self) -> &World {
        self.world
    }

    #[inline]
    pub fn world_mut(&mut self) -> &mut World {
        self.world
    }

    #[inline]
    pub fn contains<T: Component>(&self) -> bool {
        self.world.contains::<T>(self.entity)
    }

    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'_ T> {
        self.world.get(self.entity)
    }

    #[inline]
    pub fn get_mut<T: Component>(&mut self) -> Option<Mut<'_, T>> {
        self.world.get_mut(self.entity)
    }

    #[inline]
    pub fn insert<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        let change_tick = self.world.change_tick();
        let bundle_info = self
            .world
            .bundles
            .init_bundle::<T>(&mut self.world.components);

        unsafe {
            bundle_info.insert(
                self.entity,
                bundle,
                &mut self.world.components,
                &mut self.world.storage,
                change_tick,
            )
        };
        self
    }

    #[inline]
    pub fn remove<T: Bundle>(&mut self) -> Option<T> {
        self.world.remove(self.entity)
    }

    #[inline]
    pub fn despawn(self) {
        self.world.despawn(self.entity);
    }
}

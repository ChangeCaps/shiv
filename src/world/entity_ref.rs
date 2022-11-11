use crate::{
    bundle::Bundle,
    change_detection::{Mut, Ticks},
    storage::ComponentStorage,
};

use super::{Component, Entity, Storage, World};

#[derive(Clone, Copy, Debug)]
pub struct EntityRef<'w> {
    world: &'w World,
    entity: Entity,
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
        let id = if let Some(id) = self.world.components.get_component::<T>() {
            id
        } else {
            return false;
        };

        let storage_sets = <T::Storage as Storage>::get(&self.world.storage);
        let storage = unsafe { storage_sets.get_unchecked(id) };

        storage.contains(self.entity)
    }

    #[inline]
    pub fn get<T: Component>(&self) -> Option<&T> {
        let id = self.world.components.get_component::<T>()?;

        let storage_sets = <T::Storage as Storage>::get(&self.world.storage);
        let storage = unsafe { storage_sets.get_unchecked(id) };

        let ptr = storage.get(self.entity)?;
        Some(unsafe { &*(ptr as *const T) })
    }
}

#[derive(Debug)]
pub struct EntityMut<'w> {
    world: &'w mut World,
    entity: Entity,
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
    pub fn contains<T: Component>(&self) -> bool {
        let id = if let Some(id) = self.world.components.get_component::<T>() {
            id
        } else {
            return false;
        };

        let storage_sets = <T::Storage as Storage>::get(&self.world.storage);
        let storage = unsafe { storage_sets.get_unchecked(id) };

        storage.contains(self.entity)
    }

    #[inline]
    pub fn get<T: Component>(&self) -> Option<&T> {
        let id = self.world.components.get_component::<T>()?;

        let storage_sets = <T::Storage as Storage>::get(&self.world.storage);
        let storage = unsafe { storage_sets.get_unchecked(id) };

        let ptr = storage.get(self.entity)?;
        Some(unsafe { &*(ptr as *const T) })
    }

    #[inline]
    pub fn get_mut<T: Component>(&mut self) -> Option<Mut<'_, T>> {
        let id = self.world.components.get_component::<T>()?;

        let storage_sets = <T::Storage as Storage>::get_mut(&mut self.world.storage);
        let storage = unsafe { storage_sets.get_unchecked_mut(id) };

        let ptr = storage.get(self.entity)?;
        let ticks = unsafe { storage.get_ticks_unchecked(self.entity) };

        Some(Mut {
            value: unsafe { &mut *(ptr as *mut T) },
            ticks: Ticks {
                ticks: unsafe { &mut *ticks.get() },
                last_change_tick: self.world.last_change_tick(),
                change_tick: self.world.change_tick(),
            },
        })
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
        let bundle_info = self
            .world
            .bundles
            .init_bundle::<T>(&mut self.world.components);

        unsafe {
            bundle_info.remove::<T>(
                self.entity,
                &mut self.world.components,
                &mut self.world.storage,
            )
        }
    }
}

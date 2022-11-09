use std::mem::{self, MaybeUninit};

use crate::{
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
    pub fn insert<T: Component>(&mut self, mut component: T) -> &mut Self {
        let id = self.world.init_component::<T>();

        let change_tick = self.world.change_tick();

        let storage_sets = <T::Storage as Storage>::get_mut(&mut self.world.storage);
        let storage = unsafe { storage_sets.get_unchecked_mut(id) };

        unsafe {
            storage.insert(
                self.entity,
                &mut component as *mut T as *mut u8,
                change_tick,
            )
        };

        mem::forget(component);

        self
    }

    #[inline]
    pub fn remove<T: Component>(&mut self) -> Option<T> {
        let id = self.world.init_component::<T>();

        let storage_sets = <T::Storage as Storage>::get_mut(&mut self.world.storage);
        let storage = unsafe { storage_sets.get_unchecked_mut(id) };

        if !storage.contains(self.entity) {
            return None;
        }

        let mut component = MaybeUninit::<T>::uninit();
        unsafe { storage.remove_unchecked(self.entity, component.as_mut_ptr() as *mut u8) }
        Some(unsafe { component.assume_init() })
    }
}

use std::{
    any::TypeId,
    mem::{self, MaybeUninit},
};

use crate::{hash_map::HashMap, Component, ComponentDescriptor, Entity, SparseStorage};

pub struct StorageSets<T> {
    types: HashMap<TypeId, T>,
}

impl<T> Default for StorageSets<T> {
    #[inline]
    fn default() -> Self {
        Self {
            types: HashMap::default(),
        }
    }
}

impl<T> StorageSets<T>
where
    T: StorageSet,
{
    #[inline]
    pub fn get<U: Component>(&self) -> Option<&T> {
        self.types.get(&TypeId::of::<U>())
    }

    #[inline]
    pub fn get_mut<U: Component>(&mut self) -> Option<&mut T> {
        self.types.get_mut(&TypeId::of::<U>())
    }

    #[inline]
    pub fn get_or_insert<U: Component>(&mut self) -> &mut T {
        self.types
            .entry(TypeId::of::<U>())
            .or_insert_with(|| T::new(ComponentDescriptor::new::<U>(), 0))
    }

    #[inline]
    pub fn get_raw_ptr<U: Component>(&self, entity: Entity) -> Option<*mut U> {
        self.get::<U>()?.get(entity).map(|x| x as *mut U)
    }

    #[inline]
    pub fn get_component<U: Component>(&self, entity: Entity) -> Option<&U> {
        self.get_raw_ptr::<U>(entity).map(|x| unsafe { &*x })
    }

    #[inline]
    pub fn get_component_mut<U: Component>(&mut self, entity: Entity) -> Option<&mut U> {
        self.get_raw_ptr::<U>(entity).map(|x| unsafe { &mut *x })
    }

    #[inline]
    pub fn insert_component<U: Component>(
        &mut self,
        entity: Entity,
        mut component: U,
        change_tick: u32,
    ) {
        let ptr = &mut component as *mut U;
        unsafe {
            self.get_or_insert::<U>()
                .insert(entity, ptr as *mut u8, change_tick)
        };
        mem::forget(component);
    }

    #[inline]
    pub fn remove_component<U: Component>(&mut self, entity: Entity) -> Option<U> {
        let storage = self.get_mut::<U>()?;

        if !storage.contains(entity) {
            return None;
        }

        let mut component = MaybeUninit::<U>::uninit();

        unsafe { storage.remove_unchecked(entity, component.as_mut_ptr() as *mut u8) };

        Some(unsafe { component.assume_init() })
    }

    #[inline]
    pub fn remove_and_drop<U: Component>(&mut self, entity: Entity) {
        if let Some(storage) = self.get_mut::<U>() {
            storage.remove_and_drop(entity);
        }
    }

    #[inline]
    pub fn remove(&mut self, entity: Entity) {
        for storage in self.types.values_mut() {
            storage.remove_and_drop(entity);
        }
    }
}

#[derive(Default)]
pub struct ComponentStorage {
    sparse: StorageSets<SparseStorage>,
}

impl ComponentStorage {
    #[inline]
    pub fn sparse(&self) -> &StorageSets<SparseStorage> {
        &self.sparse
    }

    #[inline]
    pub fn sparse_mut(&mut self) -> &mut StorageSets<SparseStorage> {
        &mut self.sparse
    }

    #[inline]
    pub fn remove(&mut self, entity: Entity) {
        self.sparse.remove(entity);
    }

    #[inline]
    pub fn insert<U: Component>(&mut self, entity: Entity, component: U, change_tick: u32) {
        self.sparse.insert_component(entity, component, change_tick);
    }

    #[inline]
    pub fn remove_component<U: Component>(&mut self, entity: Entity) -> Option<U> {
        self.sparse.remove_component(entity)
    }

    #[inline]
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.sparse.get_component(entity)
    }

    #[inline]
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.sparse.get_component_mut(entity)
    }

    #[inline]
    pub fn get_raw_ptr<T: Component>(&self, entity: Entity) -> Option<*mut u8> {
        self.sparse.get::<T>()?.get(entity)
    }
}

pub trait StorageSet: Send + Sync + 'static {
    /// Create a new storage for the given component type `T`.
    fn new(desc: ComponentDescriptor, capacity: usize) -> Self;

    /// Returns `true` if the storage contains a component for the given entity.
    fn contains(&self, entity: Entity) -> bool;

    /// Inserts a component for the given entity.
    ///
    /// # Safety
    /// - The storage must be able to store components of type `T`.
    unsafe fn insert(&mut self, entity: Entity, component: *mut u8, change_ticks: u32);

    /// Removes a component for the given entity.
    ///
    /// # Safety
    /// - `entity` must be contained in the storage.
    /// - `component` must be a valid pointer.
    unsafe fn remove_unchecked(&mut self, entity: Entity, component: *mut u8);

    /// Removes a component for the given entity.
    fn remove_and_drop(&mut self, entity: Entity);

    /// Returns a pointer to the component for the given entity.
    ///
    /// # Safety
    /// - `entity` must be contained in the storage.
    unsafe fn get_unchecked(&self, entity: Entity) -> *mut u8;

    /// Returns a pointer to the component for the given entity.
    #[inline]
    fn get(&self, entity: Entity) -> Option<*mut u8> {
        if self.contains(entity) {
            unsafe { Some(self.get_unchecked(entity)) }
        } else {
            None
        }
    }
}

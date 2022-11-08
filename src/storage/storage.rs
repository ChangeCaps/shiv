use std::cell::UnsafeCell;

use crate::{
    change_detection::ChangeTicks,
    world::{ComponentDescriptor, ComponentId, ComponentInfo, Entity, EntityIdSet},
};

use super::{DenseStorage, SparseArray};

pub struct StorageSet<T> {
    storage_sets: SparseArray<T>,
}

impl<T> Default for StorageSet<T> {
    #[inline]
    fn default() -> Self {
        Self {
            storage_sets: SparseArray::new(),
        }
    }
}

impl<T> StorageSet<T>
where
    T: ComponentStorage,
{
    #[inline]
    pub fn get(&self, id: ComponentId) -> Option<&T> {
        self.storage_sets.get(id.index())
    }

    #[inline]
    pub fn get_mut(&mut self, id: ComponentId) -> Option<&mut T> {
        self.storage_sets.get_mut(id.index())
    }

    #[inline]
    pub unsafe fn get_unchecked(&self, id: ComponentId) -> &T {
        unsafe { self.storage_sets.get_unchecked(id.index()) }
    }

    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, id: ComponentId) -> &mut T {
        unsafe { self.storage_sets.get_unchecked_mut(id.index()) }
    }

    #[inline]
    pub fn initialize(&mut self, info: &ComponentInfo) {
        if !self.storage_sets.contains(info.id().index()) {
            let set = ComponentStorage::new(*info.descriptor(), 0);
            self.storage_sets.insert(info.id().index(), set);
        }
    }

    #[inline]
    pub fn remove_and_drop(&mut self, entity: Entity, id: ComponentId) {
        if let Some(storage) = self.get_mut(id) {
            storage.remove_and_drop(entity);
        }
    }

    #[inline]
    pub fn remove(&mut self, entity: Entity) {
        for (_, storage) in self.storage_sets.iter_mut() {
            storage.remove_and_drop(entity);
        }
    }
}

#[derive(Default)]
pub struct Storages {
    sparse: StorageSet<DenseStorage>,
}

impl Storages {
    #[inline]
    pub fn sparse(&self) -> &StorageSet<DenseStorage> {
        &self.sparse
    }

    #[inline]
    pub fn sparse_mut(&mut self) -> &mut StorageSet<DenseStorage> {
        &mut self.sparse
    }

    #[inline]
    pub fn remove(&mut self, entity: Entity) {
        self.sparse.remove(entity);
    }

    #[inline]
    pub fn contains(&self, id: ComponentId, entity: Entity) -> bool {
        if let Some(storage) = self.sparse.get(id) {
            return storage.contains(entity);
        }

        false
    }

    #[inline]
    pub fn entity_ids(&self, id: ComponentId) -> EntityIdSet {
        if let Some(sparse) = self.sparse.get(id) {
            return sparse.entity_ids();
        }

        EntityIdSet::default()
    }

    #[inline]
    pub fn check_change_ticks(&mut self, tick: u32) {
        for (_, storage) in self.sparse.storage_sets.iter_mut() {
            storage.check_change_ticks(tick);
        }
    }
}

pub trait ComponentStorage: Send + Sync + 'static {
    /// Create a new storage for the given component type `T`.
    fn new(desc: ComponentDescriptor, capacity: usize) -> Self;

    /// Returns `true` if the storage contains a component for the given entity.
    fn contains(&self, entity: Entity) -> bool;

    fn entity_ids(&self) -> EntityIdSet;

    /// Inserts a component for the given entity.
    ///
    /// # Safety
    /// - The storage must be able to store components of type `T`.
    unsafe fn insert(&mut self, entity: Entity, data: *mut u8, change_ticks: u32);

    /// Removes a component for the given entity.
    ///
    /// # Safety
    /// - `entity` must be contained in the storage.
    /// - `component` must be a valid pointer.
    unsafe fn remove_unchecked(&mut self, entity: Entity, data: *mut u8);

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

    /// Returns change ticks for the given entity.
    ///
    /// # Safety
    /// - `entity` must be contained in the storage.
    unsafe fn get_ticks_unchecked(&self, entity: Entity) -> &UnsafeCell<ChangeTicks>;

    /// Returns a pointer to the component for the given entity and it's change ticks.
    ///
    /// # Safety
    /// - `entity` must be contained in the storage.
    unsafe fn get_with_ticks_unchecked(
        &self,
        entity: Entity,
    ) -> (*mut u8, &UnsafeCell<ChangeTicks>);
}

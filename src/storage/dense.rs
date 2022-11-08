use std::cell::UnsafeCell;

use crate::{
    change_detection::ChangeTicks,
    world::{ComponentDescriptor, Entity, EntityIdSet},
};

use super::{Column, ComponentStorage, SparseArray};

pub struct DenseStorage {
    dense: Column,
    entities: Vec<u32>,
    sparse: SparseArray<u32>,
}

impl DenseStorage {
    #[inline]
    pub fn new(desc: ComponentDescriptor, capacity: usize) -> Self {
        Self {
            dense: Column::with_capacity(&desc, capacity),
            entities: Vec::new(),
            sparse: SparseArray::new(),
        }
    }

    #[inline]
    pub fn check_change_ticks(&mut self, change_tick: u32) {
        self.dense.check_change_ticks(change_tick);
    }

    #[inline]
    unsafe fn swap(&mut self, index: usize) {
        self.entities.swap_remove(index);
        if index != self.dense.len() {
            let swapped = self.entities[index];
            unsafe { *self.sparse.get_unchecked_mut(swapped as usize) = index as u32 };
        }
    }
}

impl ComponentStorage for DenseStorage {
    #[inline]
    fn new(desc: ComponentDescriptor, capacity: usize) -> Self {
        Self::new(desc, capacity)
    }

    #[inline]
    fn contains(&self, entity: Entity) -> bool {
        self.sparse.contains(entity.index() as usize)
    }

    #[inline]
    fn entity_ids(&self) -> EntityIdSet {
        self.sparse.iter().map(|(id, _)| id).collect()
    }

    #[inline]
    unsafe fn insert(&mut self, entity: Entity, data: *mut u8, change_tick: u32) {
        if let Some(&index) = self.sparse.get(entity.index() as usize) {
            unsafe { self.dense.replace(index as usize, data, change_tick) };
        } else {
            let dense_index = self.dense.len() as u32;

            unsafe { self.dense.push(data, ChangeTicks::new(change_tick)) };
            self.sparse.insert(entity.index() as usize, dense_index);
            self.entities.push(entity.index());
        }
    }

    #[inline]
    unsafe fn remove_unchecked(&mut self, entity: Entity, data: *mut u8) {
        let index = unsafe { self.sparse.remove_unchecked(entity.index() as usize) };
        unsafe { self.dense.swap_remove_unchecked(index as usize, data) };
        unsafe { self.swap(index as usize) };
    }

    #[inline]
    fn remove_and_drop(&mut self, entity: Entity) {
        if let Some(index) = self.sparse.remove(entity.index() as usize) {
            // SAFETY: `index` is a valid index into `self.dense`.
            unsafe { self.dense.swap_remove_and_drop_unchecked(index as usize) };
            unsafe { self.swap(index as usize) };
        }
    }

    #[inline]
    unsafe fn get_unchecked(&self, entity: Entity) -> *mut u8 {
        // SAFETY: `entity` is contained in self as per safety requirement.
        let &index = unsafe { self.sparse.get_unchecked(entity.index() as usize) };
        unsafe { self.dense.get_data_unchecked(index as usize) }
    }

    #[inline]
    fn get(&self, entity: Entity) -> Option<*mut u8> {
        let index = self.sparse.get(entity.index() as usize)?;
        self.dense.get_data(*index as usize)
    }

    #[inline]
    unsafe fn get_ticks_unchecked(&self, entity: Entity) -> &UnsafeCell<ChangeTicks> {
        let &index = unsafe { self.sparse.get_unchecked(entity.index() as usize) };
        unsafe { self.dense.get_ticks_unchecked(index as usize) }
    }

    #[inline]
    unsafe fn get_with_ticks_unchecked(
        &self,
        entity: Entity,
    ) -> (*mut u8, &UnsafeCell<ChangeTicks>) {
        // SAFETY: `entity` is contained in self as per safety requirement.
        let &index = unsafe { self.sparse.get_unchecked(entity.index() as usize) };
        let data = unsafe { self.dense.get_data_unchecked(index as usize) };
        let ticks = unsafe { self.dense.get_ticks_unchecked(index as usize) };

        (data, ticks)
    }
}

use std::cell::UnsafeCell;

use crate::{ChangeTicks, Column, ComponentDescriptor, Entity, EntityIdSet, StorageSet};

#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SparseArray<T> {
    data: Vec<Option<T>>,
}

impl<T> Default for SparseArray<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SparseArray<T> {
    #[inline]
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    #[inline]
    pub fn contains(&self, index: usize) -> bool {
        self.data.get(index).map_or(false, Option::is_some)
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)?.as_ref()
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)?.as_mut()
    }

    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        unsafe { self.data.get_unchecked(index).as_ref().unwrap_unchecked() }
    }

    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        unsafe {
            self.data
                .get_unchecked_mut(index)
                .as_mut()
                .unwrap_unchecked()
        }
    }

    #[inline]
    pub fn insert(&mut self, index: usize, value: T) {
        if index >= self.data.len() {
            self.data.resize_with(index + 1, Default::default);
        }

        self.data[index] = Some(value)
    }

    #[inline]
    pub fn remove(&mut self, index: usize) -> Option<T> {
        self.data.get_mut(index)?.take()
    }

    #[inline]
    pub unsafe fn remove_unchecked(&mut self, index: usize) -> T {
        unsafe { self.data.get_unchecked_mut(index).take().unwrap_unchecked() }
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.data
            .iter()
            .enumerate()
            .filter_map(|(index, value)| value.as_ref().map(|value| (index, value)))
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut T)> {
        self.data
            .iter_mut()
            .enumerate()
            .filter_map(|(index, value)| value.as_mut().map(|value| (index, value)))
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data.clear()
    }
}

pub struct SparseStorage {
    dense: Column,
    sparse: SparseArray<u32>,
}

impl SparseStorage {
    #[inline]
    pub fn new(desc: ComponentDescriptor, capacity: usize) -> Self {
        Self {
            dense: Column::with_capacity(&desc, capacity),
            sparse: SparseArray::new(),
        }
    }
}

impl StorageSet for SparseStorage {
    #[inline]
    fn new(desc: ComponentDescriptor, capacity: usize) -> Self {
        Self::new(desc, capacity)
    }

    #[inline]
    fn contains(&self, entity: Entity) -> bool {
        self.sparse.contains(entity.index() as usize)
    }

    #[inline]
    fn entities(&self) -> EntityIdSet {
        self.sparse.iter().map(|(id, _)| id).collect()
    }

    #[inline]
    unsafe fn insert(&mut self, entity: Entity, component: *mut u8, change_tick: u32) {
        if let Some(&index) = self.sparse.get(entity.index() as usize) {
            unsafe { self.dense.replace(index as usize, component, change_tick) };
        } else {
            let dense_index = self.dense.len() as u32;
            unsafe { self.dense.push(component, ChangeTicks::new(change_tick)) };
            self.sparse.insert(entity.index() as usize, dense_index);
        }
    }

    #[inline]
    unsafe fn remove_unchecked(&mut self, entity: Entity, component: *mut u8) {
        let index = unsafe { self.sparse.remove_unchecked(entity.index() as usize) };
        unsafe { self.dense.swap_remove_unchecked(index as usize, component) };
    }

    #[inline]
    fn remove_and_drop(&mut self, entity: Entity) {
        if let Some(index) = self.sparse.remove(entity.index() as usize) {
            // SAFETY: `index` is a valid index into `self.dense`.
            unsafe { self.dense.swap_remove_and_drop_unchecked(index as usize) };
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

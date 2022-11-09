use std::cell::UnsafeCell;

use crate::{change_detection::ChangeTicks, world::ComponentId};

use super::SparseArray;

pub trait Resource: Send + Sync + 'static {}

impl<T> Resource for T where T: Send + Sync + 'static {}

impl dyn Resource {
    #[inline]
    pub unsafe fn downcast_ref<T: Resource>(&self) -> &T {
        unsafe { &*(self as *const dyn Resource as *const T) }
    }

    #[inline]
    pub unsafe fn downcast_mut<T: Resource>(&mut self) -> &mut T {
        unsafe { &mut *(self as *mut dyn Resource as *mut T) }
    }
}

pub struct ResourceData {
    data: *mut dyn Resource,
    change_ticks: UnsafeCell<ChangeTicks>,
}

impl ResourceData {
    #[inline]
    pub fn new<T: Resource>(data: T, change_tick: u32) -> Self {
        Self {
            data: Box::into_raw(Box::new(data)),
            change_ticks: UnsafeCell::new(ChangeTicks::new(change_tick)),
        }
    }

    #[inline]
    pub fn from_boxed(data: Box<dyn Resource>, change_tick: u32) -> Self {
        Self {
            data: Box::into_raw(data),
            change_ticks: UnsafeCell::new(ChangeTicks::new(change_tick)),
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut dyn Resource {
        self.data
    }

    #[inline]
    pub fn into_ptr(self) -> *mut dyn Resource {
        let data = self.data;

        std::mem::forget(self);

        data
    }

    #[inline]
    pub fn change_ticks(&self) -> &UnsafeCell<ChangeTicks> {
        &self.change_ticks
    }

    #[inline]
    pub fn change_ticks_mut(&mut self) -> &mut ChangeTicks {
        self.change_ticks.get_mut()
    }
}

impl Drop for ResourceData {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: `self.data` was crated from a Box.
        unsafe { Box::from_raw(self.data) };
    }
}

impl std::fmt::Debug for ResourceData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceData")
            .field("data", &self.data)
            .field("change_ticks", &self.change_ticks)
            .finish()
    }
}

#[derive(Default)]
pub struct Resources {
    resources: SparseArray<ResourceData>,
}

impl Resources {
    #[inline]
    pub fn len(&self) -> usize {
        self.resources.iter().count()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn contains(&self, id: ComponentId) -> bool {
        self.resources.contains(id.index())
    }

    #[inline]
    pub unsafe fn insert(
        &mut self,
        id: ComponentId,
        resource: Box<dyn Resource>,
        change_tick: u32,
    ) {
        let data = ResourceData::from_boxed(resource, change_tick);
        self.resources.insert(id.index(), data);
    }

    #[inline]
    pub fn remove(&mut self, id: ComponentId) -> Option<*mut dyn Resource> {
        Some(self.resources.remove(id.index())?.into_ptr())
    }

    #[inline]
    pub fn get(&self, id: ComponentId) -> Option<*mut dyn Resource> {
        Some(self.resources.get(id.index())?.as_ptr())
    }

    #[inline]
    pub fn get_with_ticks(&self, id: ComponentId) -> Option<(*mut dyn Resource, *mut ChangeTicks)> {
        let data = self.resources.get(id.index())?;
        Some((data.as_ptr(), data.change_ticks.get()))
    }
}

impl std::fmt::Debug for Resources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resources")
            .field("len", &self.len())
            .finish()
    }
}

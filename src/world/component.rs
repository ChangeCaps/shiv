use std::{alloc::Layout, any::TypeId};

use crate::{hash_map::HashMap, ComponentStorage, SparseStorage, StorageSet, StorageSets};

pub trait Component: Send + Sync + 'static {
    type Storage: Storage;
}

pub trait Storage: StorageSet + Sized {
    fn get(storage: &ComponentStorage) -> &StorageSets<Self>;
    fn get_mut(storage: &mut ComponentStorage) -> &mut StorageSets<Self>;
}

impl Storage for SparseStorage {
    #[inline]
    fn get(storage: &ComponentStorage) -> &StorageSets<Self> {
        storage.sparse()
    }

    #[inline]
    fn get_mut(storage: &mut ComponentStorage) -> &mut StorageSets<Self> {
        storage.sparse_mut()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ComponentDescriptor {
    layout: Layout,
    drop: Option<unsafe fn(*mut u8)>,
}

impl ComponentDescriptor {
    #[inline]
    pub const fn new<T: Component>() -> Self {
        Self {
            layout: Layout::new::<T>(),
            drop: Some(|ptr| unsafe { std::ptr::drop_in_place(ptr as *mut T) }),
        }
    }

    #[inline]
    pub const fn layout(&self) -> Layout {
        self.layout
    }

    #[inline]
    pub const fn drop(&self) -> Option<unsafe fn(*mut u8)> {
        self.drop
    }
}

#[derive(Clone, Debug)]
pub struct ComponentInfo {
    pub id: ComponentId,
    pub descriptor: ComponentDescriptor,
}

impl ComponentInfo {
    #[inline]
    pub const fn id(&self) -> ComponentId {
        self.id
    }

    #[inline]
    pub const fn descriptor(&self) -> &ComponentDescriptor {
        &self.descriptor
    }

    #[inline]
    pub const fn layout(&self) -> Layout {
        self.descriptor.layout
    }

    #[inline]
    pub const fn drop(&self) -> Option<unsafe fn(*mut u8)> {
        self.descriptor.drop
    }

    #[inline]
    fn new(id: ComponentId, descriptor: ComponentDescriptor) -> Self {
        Self { id, descriptor }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComponentId(usize);

impl ComponentId {
    #[inline]
    pub const fn new(index: usize) -> ComponentId {
        ComponentId(index)
    }

    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

pub struct Components {
    components: Vec<ComponentInfo>,
    indices: HashMap<TypeId, usize>,
    resource_indices: HashMap<TypeId, usize>,
}

impl Components {}

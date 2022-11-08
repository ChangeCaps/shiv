use std::{alloc::Layout, any::TypeId};

use crate::{
    hash_map::HashMap,
    storage::{ComponentStorage, DenseStorage, Resource, StorageSet, Storages},
};

pub use shiv_macro::Component;

pub trait Component: Send + Sync + 'static {
    type Storage: Storage;
}

pub trait Storage: ComponentStorage + Sized {
    fn get(storage: &Storages) -> &StorageSet<Self>;
    fn get_mut(storage: &mut Storages) -> &mut StorageSet<Self>;
}

impl Storage for DenseStorage {
    #[inline]
    fn get(storage: &Storages) -> &StorageSet<Self> {
        storage.sparse()
    }

    #[inline]
    fn get_mut(storage: &mut Storages) -> &mut StorageSet<Self> {
        storage.sparse_mut()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ComponentDescriptor {
    name: &'static str,
    layout: Layout,
    drop: Option<unsafe fn(*mut u8)>,
}

impl ComponentDescriptor {
    #[inline]
    pub fn new<T>() -> Self {
        Self {
            name: std::any::type_name::<T>(),
            layout: Layout::new::<T>(),
            drop: Some(|ptr| unsafe { std::ptr::drop_in_place(ptr as *mut T) }),
        }
    }

    #[inline]
    pub const fn name(&self) -> &'static str {
        self.name
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
    pub const fn name(&self) -> &'static str {
        self.descriptor.name()
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

impl Into<usize> for ComponentId {
    #[inline]
    fn into(self) -> usize {
        self.0
    }
}

impl From<usize> for ComponentId {
    #[inline]
    fn from(index: usize) -> ComponentId {
        ComponentId(index)
    }
}

#[derive(Clone, Default)]
pub struct Components {
    components: Vec<ComponentInfo>,
    indices: HashMap<TypeId, usize>,
    resource_indices: HashMap<TypeId, usize>,
}

impl Components {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn init_component<T: Component>(&mut self) -> ComponentId {
        let type_id = TypeId::of::<T>();

        if let Some(index) = self.indices.get(&type_id) {
            return ComponentId::new(*index);
        }

        let index = self.components.len();
        self.components.push(ComponentInfo::new(
            ComponentId::new(index),
            ComponentDescriptor::new::<T>(),
        ));
        self.indices.insert(type_id, index);

        ComponentId::new(index)
    }

    #[inline]
    pub fn init_resource<T: Resource>(&mut self) -> ComponentId {
        let type_id = TypeId::of::<T>();

        if let Some(index) = self.resource_indices.get(&type_id) {
            return ComponentId::new(*index);
        }

        let index = self.components.len();
        self.components.push(ComponentInfo::new(
            ComponentId::new(index),
            ComponentDescriptor::new::<T>(),
        ));
        self.resource_indices.insert(type_id, index);

        ComponentId::new(index)
    }

    #[inline]
    pub fn get_component<T: Component>(&self) -> Option<ComponentId> {
        let type_id = TypeId::of::<T>();

        if let Some(index) = self.indices.get(&type_id) {
            Some(ComponentId::new(*index))
        } else {
            None
        }
    }

    #[inline]
    pub fn get_resource<T: Resource>(&self) -> Option<ComponentId> {
        let type_id = TypeId::of::<T>();

        if let Some(index) = self.resource_indices.get(&type_id) {
            Some(ComponentId::new(*index))
        } else {
            None
        }
    }

    #[inline]
    pub fn contains_component<T: Component>(&self) -> bool {
        self.indices.contains_key(&TypeId::of::<T>())
    }

    #[inline]
    pub fn contains_resource<T: Resource>(&self) -> bool {
        self.resource_indices.contains_key(&TypeId::of::<T>())
    }

    #[inline]
    pub fn get(&self, id: ComponentId) -> Option<&ComponentInfo> {
        self.components.get(id.index())
    }

    #[inline]
    pub unsafe fn get_unchecked(&self, id: ComponentId) -> &ComponentInfo {
        unsafe { self.components.get_unchecked(id.index()) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.components.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

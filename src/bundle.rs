use std::{
    any::TypeId,
    mem::{self, MaybeUninit},
};

use crate::{
    storage::{ComponentStorage, StorageType, Storages},
    world::{Component, ComponentId, Components, Entity},
};

use ahash::HashMap;
pub use shiv_macro::Bundle;

pub unsafe trait Bundle: Send + Sync + 'static {
    type Iter: Iterator<Item = *mut u8>;

    fn components(components: &mut Components) -> Vec<ComponentId>;

    fn get_components(bundle: *mut Self) -> Self::Iter;
}

unsafe impl<T: Component> Bundle for T {
    type Iter = std::iter::Once<*mut u8>;

    #[inline]
    fn components(components: &mut Components) -> Vec<ComponentId> {
        vec![components.init_component::<T>()]
    }

    #[inline]
    fn get_components(bundle: *mut Self) -> Self::Iter {
        std::iter::once(bundle as *mut u8)
    }
}

#[derive(Clone, Debug)]
pub struct BundleInfo {
    component_ids: Vec<ComponentId>,
}

impl BundleInfo {
    /// # Safety
    /// - `components` must be the same as the `Components` used to create this `BundleInfo`.
    /// - `bundle` must be a valid instance of the bundle type `self` was created for.
    #[inline]
    pub unsafe fn insert<T: Bundle>(
        &self,
        entity: Entity,
        mut bundle: T,
        components: &mut Components,
        storages: &mut Storages,
        change_tick: u32,
    ) {
        for (i, data) in T::get_components(&mut bundle).enumerate() {
            let component_id = unsafe { self.component_ids.get_unchecked(i) };
            let info = unsafe { components.get_unchecked(*component_id) };

            match info.storage_type() {
                StorageType::Dense => {
                    let storage = storages.dense.get_or_init(info);
                    unsafe { storage.insert(entity, data, change_tick) };
                }
                _ => unreachable!(),
            }
        }

        mem::forget(bundle);
    }

    /// # Safety
    /// - `components` must be the same as the `Components` used to create this `BundleInfo`.
    /// - `T` must be the same as the bundle type `self` was created for.
    pub unsafe fn remove<T: Bundle>(
        &self,
        entity: Entity,
        components: &mut Components,
        storages: &mut Storages,
    ) -> Option<T> {
        for &component_id in self.component_ids.iter() {
            if !storages.contains(component_id, entity) {
                return None;
            }
        }

        let mut bundle = MaybeUninit::<T>::uninit();

        for (i, data) in T::get_components(bundle.as_mut_ptr()).enumerate() {
            let component_id = unsafe { *self.component_ids.get_unchecked(i) };
            let info = unsafe { components.get_unchecked(component_id) };

            match info.storage_type() {
                StorageType::Dense => {
                    let storage = storages.dense.get_mut(component_id)?;
                    unsafe { storage.remove_unchecked(entity, data) };
                }
                _ => unreachable!(),
            }
        }

        Some(unsafe { bundle.assume_init() })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Bundles {
    bundles: HashMap<TypeId, BundleInfo>,
}

impl Bundles {
    #[inline]
    pub fn get(&self, type_id: TypeId) -> Option<&BundleInfo> {
        self.bundles.get(&type_id)
    }

    #[inline]
    pub fn init_bundle<T: Bundle>(&mut self, components: &mut Components) -> &BundleInfo {
        let id = TypeId::of::<T>();
        let component_ids = T::components(components);
        self.bundles.insert(id, BundleInfo { component_ids });
        self.bundles.get(&id).unwrap()
    }
}

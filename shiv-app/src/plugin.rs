use std::{
    any::TypeId,
    ops::{Deref, Range},
};

use downcast_rs::{impl_downcast, Downcast};

use crate::App;

pub trait PluginId: Downcast + Send + Sync {
    fn eq(&self, other: &dyn PluginId) -> bool;
}

impl_downcast!(PluginId);

impl PartialEq for dyn PluginId {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.eq(other)
    }
}

impl Eq for dyn PluginId {}

impl<T: PartialEq + Send + Sync + 'static> PluginId for T {
    fn eq(&self, other: &dyn PluginId) -> bool {
        if let Some(other) = other.downcast_ref::<T>() {
            self == other
        } else {
            false
        }
    }
}

pub trait Plugin: Downcast + Send + Sync + 'static {
    #[inline]
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn build(&self, app: &mut App);

    #[inline]
    fn dependencies(&self, _plugins: &mut Plugins) {}

    #[inline]
    fn id(&self) -> Box<dyn PluginId> {
        Box::new(TypeId::of::<Self>())
    }
}

impl_downcast!(Plugin);

#[derive(Default)]
pub struct Plugins {
    current: Vec<usize>,
    plugins: Vec<Box<dyn Plugin>>,
    ids: Vec<(Box<dyn PluginId>, usize)>,
}

impl Plugins {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    #[inline]
    fn get_index(&self, id: &dyn PluginId) -> Option<usize> {
        self.ids
            .iter()
            .find(|(plugin_id, _)| (*plugin_id).deref() == id)
            .map(|(_, index)| *index)
    }

    #[inline]
    pub fn add<T: Plugin>(&mut self, plugin: T) {
        let id = plugin.id();

        if let Some(index) = self.get_index(id.as_ref()) {
            if self.current.contains(&index) {
                panic!("Circular plugin dependency detected!");
            }
        } else {
            let index = self.plugins.len();

            let id_index = self.ids.len();
            self.ids.push((id, index));

            self.current.push(index);

            plugin.dependencies(self);

            let index = self.plugins.len();
            self.plugins.push(Box::new(plugin));
            self.ids[id_index].1 = index;

            self.current.pop();
        }
    }

    #[inline]
    pub fn contains<T: Plugin>(&self, plugin: T) -> bool {
        self.get_index(&plugin.id()).is_some()
    }

    #[inline]
    pub fn get<T: Plugin>(&self) -> Option<&T> {
        let id = TypeId::of::<T>();

        if let Some(index) = self.get_index(&id as &dyn PluginId) {
            self.plugins[index].downcast_ref()
        } else {
            None
        }
    }

    #[inline]
    pub fn build(&self, app: &mut App) {
        for plugin in self.plugins.iter() {
            plugin.build(app);
        }
    }

    #[inline]
    pub fn build_range(&self, app: &mut App, range: Range<usize>) {
        if let Some(plugins) = self.plugins.get(range) {
            for plugin in plugins.iter() {
                plugin.build(app);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn selv_dependency() {
        struct PluginA;

        impl Plugin for PluginA {
            fn build(&self, _app: &mut App) {}

            fn dependencies(&self, plugins: &mut Plugins) {
                plugins.add(PluginA);
            }
        }

        let mut plugins = Plugins::new();

        plugins.add(PluginA);
    }

    #[test]
    #[should_panic]
    fn cyclic_dependency() {
        struct PluginA;

        impl Plugin for PluginA {
            fn build(&self, _app: &mut App) {}

            fn dependencies(&self, plugins: &mut Plugins) {
                plugins.add(PluginB);
            }
        }

        struct PluginB;

        impl Plugin for PluginB {
            fn build(&self, _app: &mut App) {}

            fn dependencies(&self, plugins: &mut Plugins) {
                plugins.add(PluginA);
            }
        }

        let mut plugins = Plugins::new();

        plugins.add(PluginA);
        plugins.add(PluginB);
    }

    #[test]
    fn order() {
        struct PluginA;

        impl Plugin for PluginA {
            fn build(&self, app: &mut App) {
                app.insert_resource(5i32);
            }
        }

        struct PluginB;

        impl Plugin for PluginB {
            fn build(&self, app: &mut App) {
                assert_eq!(app.remove_resource::<i32>(), Some(5));
            }

            fn dependencies(&self, plugins: &mut Plugins) {
                plugins.add(PluginA);
            }
        }

        let mut plugins = Plugins::new();
        plugins.add(PluginB);

        let mut app = App::new();
        plugins.build(&mut app);
    }
}

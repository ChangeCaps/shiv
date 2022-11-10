use shiv::{
    hash_map::HashMap,
    query::{Changed, Query, With, Without},
    schedule::{IntoSystemDescriptor, SystemLabel},
    system::Commands,
    world::{Component, Entity},
};
use shiv_app::{App, CoreStage, Plugin, Plugins, StartupStage};

use crate::{Children, GlobalTransform, Parent, Transform};

/// An internal component used to maintain the hierarchy.
///
/// This should **never** be modified directly.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreviousParent(Entity);

pub fn update_parent_system(
    mut commands: Commands,
    removed_parent_query: Query<Entity, (Without<Parent>, With<PreviousParent>)>,
    parent_query: Query<(Entity, &Parent), Changed<Parent>>,
    mut previous_parent_query: Query<&mut PreviousParent>,
    mut children_query: Query<&mut Children>,
) {
    for entity in removed_parent_query.iter() {
        if let Some(mut children) = children_query.get_mut(entity) {
            let previous_parent = previous_parent_query.get(entity).unwrap();
            children.remove(previous_parent.0);
            commands.entity(entity).remove::<PreviousParent>();
        }
    }

    let mut new_children = HashMap::<Entity, Children>::default();

    for (entity, parent) in parent_query.iter() {
        if let Some(mut previous_parent) = previous_parent_query.get_mut(entity) {
            if previous_parent.0 == parent.0 {
                continue;
            }

            if let Some(mut children) = children_query.get_mut(previous_parent.0) {
                children.remove(entity);
            }

            previous_parent.0 = parent.0;
        } else {
            commands.entity(entity).insert(PreviousParent(parent.0));
        }

        if let Some(children) = new_children.get_mut(&parent.0) {
            children.push(entity);
        } else {
            new_children.entry(parent.0).or_default().push(entity);
        }
    }

    for (entity, children) in new_children {
        commands.entity(entity).insert(children);
    }
}

pub fn update_transform_system(
    mut root_query: Query<
        (
            Entity,
            Option<(&Children, Changed<Children>)>,
            &Transform,
            Changed<Transform>,
            &mut GlobalTransform,
        ),
        Without<Parent>,
    >,
    mut transform_query: Query<(
        &Transform,
        Changed<Transform>,
        &mut GlobalTransform,
        &Parent,
    )>,
    children_query: Query<(&Children, Changed<Children>), (With<Parent>, With<GlobalTransform>)>,
) {
    for (entity, children, transform, transform_changed, mut global_transform) in &mut root_query {
        let mut changed = transform_changed;
        if transform_changed {
            *global_transform = transform.into();
        }

        if let Some((children, children_changed)) = children {
            changed |= children_changed;
            for &child in children.iter() {
                propagate_recursive(
                    child,
                    entity,
                    changed,
                    *global_transform,
                    &mut transform_query,
                    &children_query,
                );
            }
        }
    }
}

#[inline]
fn propagate_recursive(
    entity: Entity,
    expected_parent: Entity,
    mut changed: bool,
    parent: GlobalTransform,
    transform_query: &mut Query<(
        &Transform,
        Changed<Transform>,
        &mut GlobalTransform,
        &Parent,
    )>,
    children_query: &Query<(&Children, Changed<Children>), (With<Parent>, With<GlobalTransform>)>,
) -> Option<()> {
    let (&transform, transform_changed, mut global_transform, child_parent) =
        transform_query.get_mut(entity)?;

    assert_eq!(child_parent.0, expected_parent, "Malformed hierarchy");

    changed |= transform_changed;
    if changed {
        *global_transform = parent * transform;
    }

    let global_transform = global_transform.clone();

    let (children, children_changed) = children_query.get(entity)?;
    changed |= children_changed;

    for &child in children.iter() {
        propagate_recursive(
            child,
            entity,
            changed,
            global_transform,
            transform_query,
            children_query,
        );
    }

    Some(())
}

#[derive(SystemLabel)]
pub enum TransformSystem {
    /// Update the [`Children`] component of the parent entity.
    UpdateParent,
    /// Update the [`GlobalTransform`] component of the entity.
    UpdateTransform,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HiracyPlugin;

impl Plugin for HiracyPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system_to_stage(
            StartupStage::PostStartup,
            update_parent_system.label(TransformSystem::UpdateParent),
        );

        app.add_system_to_stage(
            CoreStage::PostUpdate,
            update_parent_system.label(TransformSystem::UpdateParent),
        );
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system_to_stage(
            StartupStage::PostStartup,
            update_transform_system
                .label(TransformSystem::UpdateTransform)
                .after(TransformSystem::UpdateParent),
        );

        app.add_system_to_stage(
            CoreStage::PostUpdate,
            update_transform_system
                .label(TransformSystem::UpdateTransform)
                .after(TransformSystem::UpdateParent),
        );
    }

    fn dependencies(&self, plugins: &mut Plugins) {
        plugins.add(HiracyPlugin);
    }
}

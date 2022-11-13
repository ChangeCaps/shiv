use shiv::{
    hierarchy::{Children, Parent},
    query::{Changed, Query, With, Without},
    schedule::{IntoSystemDescriptor, SystemLabel},
    world::Entity,
};
use shiv_app::{App, CoreStage, Plugin, StartupStage};

use crate::{GlobalTransform, Transform};

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

    assert_eq!(
        child_parent.entity(),
        expected_parent,
        "Malformed hierarchy"
    );

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
pub struct TransformSystem;

#[derive(Clone, Copy, Debug, Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system_to_stage(
            StartupStage::PostStartup,
            update_transform_system.label(TransformSystem),
        );

        app.add_system_to_stage(
            CoreStage::PostUpdate,
            update_transform_system.label(TransformSystem),
        );
    }
}

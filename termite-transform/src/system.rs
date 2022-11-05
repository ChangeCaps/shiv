use termite::{Changed, Commands, Entity, Query, SystemLabel, With, Without};

use crate::{Children, GlobalTransform, Parent, Transform};

#[derive(SystemLabel)]
pub enum TransformSystem {
    AddGlobalTransform,
    UpdateGlobalTransform,
}

pub fn add_global_transform_system(
    mut commands: Commands,
    query: Query<Entity, (With<Transform>, Without<GlobalTransform>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(GlobalTransform::IDENTITY);
    }
}

pub fn transform_propagate_system(
    root_query: Query<
        (Entity, &Transform, Option<&Children>),
        (Without<Parent>, With<GlobalTransform>),
    >,
    transform_query: Query<(Entity, &Transform), (With<Parent>, With<GlobalTransform>)>,
    mut global_transform_query: Query<&mut GlobalTransform>,
    changed_transform_query: Query<Entity, Changed<Transform>>,
    children_query: Query<&Children, (With<Parent>, With<GlobalTransform>)>,
) {
    for (entity, transform, children) in root_query.iter() {
        let mut changed = false;
        let mut global_transform = global_transform_query.get_mut(entity).unwrap();

        if changed_transform_query.contains(entity) {
            *global_transform = transform.into();
            changed = true;
        }

        let global_transform = global_transform.clone();

        if let Some(children) = children {
            for &child in children.iter() {
                propagate_recursive(
                    global_transform,
                    &changed_transform_query,
                    &transform_query,
                    &mut global_transform_query,
                    &children_query,
                    child,
                    changed,
                );
            }
        }
    }
}

fn propagate_recursive(
    parent: GlobalTransform,
    changed_transform_query: &Query<Entity, Changed<Transform>>,
    transform_query: &Query<(Entity, &Transform), (With<Parent>, With<GlobalTransform>)>,
    global_transform_query: &mut Query<&mut GlobalTransform>,
    children_query: &Query<&Children, (With<Parent>, With<GlobalTransform>)>,
    entity: Entity,
    mut changed: bool,
) {
    changed |= children_query.contains(entity);

    let global_transform = if let Some((_entity, &transform)) = transform_query.get(entity) {
        let mut global_transform = global_transform_query.get_mut(entity).unwrap();

        if changed {
            *global_transform = parent * transform;
        }

        global_transform.clone()
    } else {
        return;
    };

    if let Some(children) = children_query.get(entity) {
        for child in children.iter() {
            propagate_recursive(
                global_transform,
                changed_transform_query,
                transform_query,
                global_transform_query,
                children_query,
                *child,
                changed,
            )
        }
    }
}

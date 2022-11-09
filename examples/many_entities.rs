use shiv::prelude::*;

#[allow(dead_code)]
#[derive(Component)]
struct User {
    id: i32,
    name: u32,
}

fn spawn(world: &mut World) {
    for i in 0..2000 {
        world.spawn().insert(User {
            id: i,
            name: u32::MAX - i as u32,
        });
    }
}

fn query(world: &mut World) {
    let query = world.query::<&User>();

    for user in query.iter(&world) {
        let _ = user.clone();
    }
}

fn main() {
    let mut world = World::new();

    spawn(&mut world);
    query(&mut world);
}

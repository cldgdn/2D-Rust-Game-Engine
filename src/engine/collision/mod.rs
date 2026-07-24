pub mod shape;

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use glam::Vec2;
use crate::engine::component::{Multi, SlotID};
use crate::engine::component::collider::Collider;
use crate::engine::component::transform::Transform;
use crate::engine::EntityID;
use crate::engine::world::World;

pub type CollisionLayer = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CollisionType {
    Solid(CollisionLayer),
    Trigger(CollisionLayer),
}

#[derive(Clone, Copy, Debug)]
pub struct CollisionInfo {
    pub collision_type: CollisionType,
    ///Direction to push the A collider during resolution
    pub normal: Vec2,
    ///Distance to push the A collider during resolution
    pub penetration: f32,   
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ColliderID {
    pub entity: EntityID,
    pub collider: SlotID,
}

#[derive(Clone, Debug)]
pub struct Collision {
    pub a: ColliderID,
    pub b: ColliderID,
    pub collision_info: CollisionInfo,
}

#[derive(Clone, Debug)]
pub struct CollisionEvent {
    pub a: ColliderID,
    pub b: ColliderID,
    pub collision_type: CollisionType,
    pub normal: Vec2,
    pub penetration: f32,
}

pub struct CollisionSystem {
    sections_static: HashMap<(i32, i32), Vec<ColliderID>>,
    sections_dynamic: HashMap<(i32, i32), Vec<ColliderID>>,
}

pub trait Collides {
    fn submit_colliders(
        &self, 
        collision_system: &mut CollisionSystem, 
        world: &mut World
    );
    fn react_to_collision(
        &mut self,
        collision: CollisionEvent,
        world: &mut World
    ) {
        ()
    }
}

impl CollisionSystem {
    pub fn new() -> CollisionSystem {
        CollisionSystem {
            sections_static: HashMap::new(),
            sections_dynamic: HashMap::new(),
        }
    }
    
    pub fn run_collisions(&mut self, world: &World) -> (HashSet<Collision>, u32) {
        let mut collisions = HashSet::new();
        let mut checks = 0;

        let multis: Vec<(EntityID, &Multi<Collider>)> = world.get_all_components::<Multi<Collider>>().collect();
        let mut colliders = Vec::new();

        for (_, multi) in multis.iter() {
            for (_, collider) in multi.iter() {
                colliders.push(collider);
            }
        }

        for collider in colliders.iter() {
            if !collider.is_active || collider.is_static {
                continue;
            }

            let sections = collider.get_sections(world);

            for section in sections {
                let section_colliders = self.sections_dynamic.entry(section).or_insert(Vec::new());

                section_colliders.push(ColliderID {
                    entity: collider.entity,
                    collider: collider.id,
                });
            }
        }

        for (section, colliders) in self.sections_dynamic.iter() {
            let static_colliders = self.sections_static.get(section);

            for (i, collider_id) in colliders.iter().enumerate() {
                let a = world
                    .get_component::<Multi<Collider>>(collider_id.entity)
                    .unwrap()
                    .get(&collider_id.collider)
                    .unwrap();

                if let Some(static_colliders) = static_colliders {
                    for static_collider in static_colliders {
                        let b = world
                            .get_component::<Multi<Collider>>(static_collider.entity)
                            .unwrap()
                            .get(&static_collider.collider)
                            .unwrap();

                        let t = a.collide_against(&b, world).unwrap();
                        if let Some(collision_info) = t {
                            collisions.insert(Collision {
                                a: collider_id.clone(),
                                b: static_collider.clone(),
                                collision_info
                            });
                        }

                        checks += 1;
                    }
                }

                for j in (i + 1)..colliders.len() {
                    let other = colliders.get(j).unwrap();

                    if collider_id.entity == other.entity {
                        continue;
                    } else {
                        checks += 2;
                    }

                    let b = world
                        .get_component::<Multi<Collider>>(other.entity)
                        .unwrap()
                        .get(&other.collider)
                        .unwrap();

                    let t = a.collide_against(&b, world).unwrap();
                    if let Some(collision_info) = t {
                        collisions.insert(Collision {
                            a: collider_id.clone(),
                            b: other.clone(),
                            collision_info
                        });
                    }

                    let t = b.collide_against(&a, world).unwrap();
                    if let Some(collision_info) = t {
                        collisions.insert(Collision {
                            a: other.clone(),
                            b: collider_id.clone(),
                            collision_info
                        });
                    }
                }
            }

        }

        self.sections_dynamic.clear();

        (collisions, checks)
    }

    pub fn resolve_collisions(&self, collisions: HashSet<Collision>, world: &mut World) -> Vec<CollisionEvent> {
        let mut events = Vec::new();

        for collision in collisions {
            let b_collider = world
                .get_component::<Multi<Collider>>(collision.b.entity)
                .unwrap()
                .get(&collision.b.collider)
                .unwrap();
            let info = collision.collision_info;

            if let CollisionType::Solid(_) = info.collision_type {
                let translation = if !b_collider.is_static {
                    info.normal * (info.penetration * 0.5)
                } else {
                    info.normal * info.penetration
                };

                let transform = world
                    .get_component_mut::<Transform>(collision.a.entity)
                    .unwrap();

                transform.translate(translation);
            }

            let event = CollisionEvent {
                a: collision.a,
                b: collision.b,
                collision_type: info.collision_type,
                penetration: info.penetration,
                normal: info.normal,
            };

            events.push(event);
        }

        events
    }

    pub fn add_static_colliders(&mut self, entity: EntityID, world: &World) {
        let colliders = world.get_component::<Multi<Collider>>(entity).unwrap();

        for (_, collider) in colliders.iter() {
            if collider.is_static {
                let sections = collider.get_sections(world);

                for section in sections {
                    let section_colliders = self.sections_static.entry(section).or_insert(Vec::new());
                    section_colliders.push(ColliderID {
                        entity,
                        collider: collider.id,
                    });
                }
            }
        }
    }

    pub fn remove_static_colliders(&mut self, entity: EntityID, world: &mut World) {
        let colliders = world.get_component::<Multi<Collider>>(entity).unwrap();

        for (_, collider) in colliders.iter() {
            if collider.is_static {
                let sections = collider.get_sections(world);

                for section in sections {
                    if let Some(section_colliders) = self.sections_static.get_mut(&section) {
                        section_colliders.retain(|x| x.entity != entity);
                    }
                }
            }
        }
    }

    pub fn debug_draw(&self, world: &World) {
        let debug_shader = world.debug_shader();
        let camera = world.camera();
        let view_projection = camera.get_view_projection();

        debug_shader.use_program();

        debug_shader.set_mat4("uViewProjection", &view_projection);

        let multis: Vec<(EntityID, &Multi<Collider>)> = world.get_all_components::<Multi<Collider>>().collect();
        let mut colliders = Vec::new();

        for (_, multi) in multis.iter() {
            for (_, collider) in multi.iter() {
                colliders.push(collider);
            }
        }

        for collider in colliders {
            collider.draw(world.gl(), debug_shader, world);
        }
    }
}

impl PartialEq for Collision {
    fn eq(&self, other: &Collision) -> bool {
        self.a == other.a && self.b == other.b
    }
}

impl Eq for Collision {}

impl Hash for Collision {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.a.hash(state);
        self.b.hash(state);
    }
}
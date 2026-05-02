use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use glam::Vec2;
use collider::Collider;
use crate::engine::collision::collider::ColliderType;
use crate::engine::EntityID;
use crate::engine::world::World;

pub mod collider;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CollisionType {
    Solid(u32),
    Trigger(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct CollisionInfo {
    pub collision_type: CollisionType,
    pub normal: Vec2,       // direction to push the A collider during resolution
    pub penetration: Vec2,  // overlap on each axis
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ColliderID {
    pub entity: EntityID,
    pub collider: String,
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
    pub penetration: Vec2,
}

pub struct CollisionSystem {
    colliders: HashMap<EntityID, HashMap<String, Collider>>,
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
            colliders: HashMap::new(),
            sections_static: HashMap::new(),
            sections_dynamic: HashMap::new(),
        }
    }
    
    pub fn run_collisions(&mut self, world: &World) -> (HashSet<Collision>, u32) {
        let mut collisions = HashSet::new();
        let mut checks = 0;

        for (_, colliders) in self.colliders.iter() {
            for collider in colliders.values() {
                if !collider.is_active || collider.is_static {
                    continue;
                }
                
                let sections = collider.get_sections(world);
                
                for section in sections {
                    let section_colliders = self.sections_dynamic.entry(section).or_insert(Vec::new());
                    
                    section_colliders.push(ColliderID {
                        entity: collider.entity,
                        collider: collider.id.clone(),
                    });
                }
            }
        }

        for (section, colliders) in self.sections_dynamic.iter() {
            let static_colliders = self.sections_static.get(section);

            for (i, collider) in colliders.iter().enumerate() {
                let a = self.get_collider(
                    collider.entity,
                    collider.collider.as_str()
                ).unwrap();

                if let Some(static_colliders) = static_colliders {
                    for static_collider in static_colliders {
                        let b = self.get_collider(
                            static_collider.entity,
                            static_collider.collider.as_str(),
                        ).unwrap();

                        let t = a.collide_against(&b, world).unwrap();
                        if let Some(collision_info) = t {
                            collisions.insert(Collision {
                                a: collider.clone(),
                                b: static_collider.clone(),
                                collision_info
                            });
                        }

                        checks += 1;
                    }
                }

                for j in (i + 1)..colliders.len() {
                    let other = colliders.get(j).unwrap();

                    if collider.entity == other.entity {
                        continue;
                    } else {
                        checks += 2;
                    }

                    let b = self.get_collider(
                        other.entity,
                        other.collider.as_str()
                    ).unwrap();

                    let t = a.collide_against(&b, world).unwrap();
                    if let Some(collision_info) = t {
                        collisions.insert(Collision {
                            a: collider.clone(),
                            b: other.clone(),
                            collision_info
                        });
                    }

                    let t = b.collide_against(&a, world).unwrap();
                    if let Some(collision_info) = t {
                        collisions.insert(Collision {
                            a: other.clone(),
                            b: collider.clone(),
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
            let b_collider = self.get_collider(
                collision.b.entity,
                collision.b.collider.as_str()
            ).unwrap();
            let info = collision.collision_info;

            if let CollisionType::Solid(_) = info.collision_type {
                let mut translation = Vec2::new(0.0, 0.0);

                if !b_collider.is_static {
                    translation.x = info.normal.x * info.penetration.x * 0.5;
                    translation.y = info.normal.y * info.penetration.y * 0.5;
                } else {
                    translation.x = info.normal.x * info.penetration.x;
                    translation.y = info.normal.y * info.penetration.y;
                }

                let transform = world
                    .get_mut_transform(collision.a.entity)
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

    pub fn add_collider(&mut self, entity: EntityID, collider: Collider, world: &World) {
        let colliders = self.colliders.entry(entity).or_insert(HashMap::new());
        
        if collider.is_static {
            let sections = collider.get_sections(world);
            
            for section in sections {
                let section_colliders = self.sections_static.entry(section).or_insert(Vec::new());
                section_colliders.push(ColliderID {
                    entity,
                    collider: collider.id.clone(),
                });
            }
        }
        
        colliders.insert(collider.id.clone(), collider);
    }

    pub fn get_collider(&self, entity: EntityID, id: &str) -> Option<&Collider> {
        let colliders = self.colliders.get(&entity)?;

        colliders.get(id)
    }

    pub fn get_collider_mut(&mut self, entity: EntityID, id: &str) -> Option<&mut Collider> {
        let colliders = self.colliders.get_mut(&entity)?;

        colliders.get_mut(id)
    }

    pub fn remove_colliders(&mut self, entity: EntityID) {
        let colliders = self.colliders.get_mut(&entity).unwrap();
        
        //TODO: Make it so that when removing static colliders, they get removed from their respective static sections
        
        colliders.clear();
    }

    pub fn debug_draw(&self, world: &World) {
        let debug_shader = world.debug_shader();
        let camera = world.camera();
        let view_projection = camera.get_view_projection();

        debug_shader.use_program();

        debug_shader.set_mat4("uViewProjection", &view_projection);
        debug_shader.set_vec2("uCameraPos", &camera.position);

        for entity_colliders in self.colliders.values() {
            for collider in entity_colliders.values() {
                collider.draw(world.gl(), debug_shader, world);
            }
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
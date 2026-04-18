use std::collections::{HashMap, HashSet};
use glam::Vec2;
use collider::Collider;
use crate::engine::EntityID;
use crate::engine::world::World;

pub mod collider;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CollisionType {
    Solid(u32),
    Trigger(u32),
    None
}

pub struct CollisionInfo {
    pub collision_type: CollisionType,
    pub normal: Vec2,       // direction to push the collider during resolution
    pub penetration: Vec2,  // overlap on each axis
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ColliderID {
    pub entity: EntityID,
    pub collider: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Collision {
    pub a: ColliderID,
    pub b: ColliderID,
    pub collision_type: CollisionType,
}

pub struct CollisionSystem {
    colliders: HashMap<EntityID, HashMap<String, Collider>>,
    sections_static: HashMap<(i32, i32), Vec<ColliderID>>,
    sections_dynamic: HashMap<(i32, i32), Vec<ColliderID>>,
}

pub trait Collides {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem);
    fn react_to_collision(
        &mut self,
        collision: Collision,
        world: &mut World
    );
}

impl CollisionSystem {
    pub fn new() -> CollisionSystem {
        CollisionSystem {
            colliders: HashMap::new(),
            sections_static: HashMap::new(),
            sections_dynamic: HashMap::new(),
        }
    }
    
    pub fn run_collisions(&mut self, world: &World) -> HashSet<Collision> {
        let mut collisions = HashSet::new();

        for (_, colliders) in self.colliders.iter() {
            for collider in colliders.values() {
                if !collider.is_active {
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

                        if let CollisionType::None = t {
                            continue;
                        }

                        collisions.insert(Collision {
                            a: collider.clone(),
                            b: static_collider.clone(),
                            collision_type: t
                        });
                    }
                }

                for j in (i + 1)..colliders.len() {
                    let other = colliders.get(j).unwrap();

                    if collider.entity == other.entity {
                        continue;
                    }

                    let b = self.get_collider(
                        other.entity,
                        other.collider.as_str()
                    ).unwrap();

                    let t = a.collide_against(&b, world).unwrap();

                    if let CollisionType::Trigger(_) | CollisionType::Solid(_) = t {
                        collisions.insert(Collision {
                            a: collider.clone(),
                            b: other.clone(),
                            collision_type: t
                        });
                    }

                    let t = b.collide_against(&a, world).unwrap();

                    if let CollisionType::Trigger(_) | CollisionType::Solid(_) = t {
                        collisions.insert(Collision {
                            a: other.clone(),
                            b: collider.clone(),
                            collision_type: t
                        });
                    }
                }
            }

        }

        self.sections_dynamic.clear();

        collisions
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

    pub fn remove_colliders(&mut self, entity: EntityID, world: &World) {
        let colliders = self.colliders.get_mut(&entity).unwrap();
        
        //TODO: Make it so that when removing static colliders, they get removed from their respective static sections
        
        colliders.clear();
    }
}
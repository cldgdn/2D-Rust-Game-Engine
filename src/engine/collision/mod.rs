use std::collections::HashMap;
use collider::Collider;
use crate::engine::EntityID;

pub mod collider;


pub enum CollisionType {
    Solid,
    Trigger,
    None
}

pub struct CollisionSystem {
    next_id: EntityID,
    colliders: HashMap<EntityID, Vec<Collider>>,
}

pub trait Collides {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem);
    fn create_colliders(&self, collision_system: &mut CollisionSystem);
}

impl CollisionSystem {
    pub fn new() -> CollisionSystem {
        CollisionSystem {
            next_id: 0,
            colliders: HashMap::new(),
        }
    }

    pub fn add_collider(&mut self, entity: EntityID, collider: Collider) {
        let vec = self.colliders.entry(entity).or_insert(Vec::new());
        vec.push(collider);
    }

    pub fn get_collider(&self, entity: EntityID, id: &str) -> Option<&Collider> {
        let vec = self.colliders.get(&entity)?;

        for collider in vec.iter() {
            if collider.id() == id {
                return Some(collider);
            }
        }

        None
    }

    pub fn get_mut_collider(&mut self, entity: EntityID, id: &str) -> Option<&mut Collider> {
        let vec = self.colliders.get_mut(&entity)?;

        for collider in vec.iter_mut() {
            if collider.id() == id {
                return Some(collider);
            }
        }

        None
    }

    pub fn remove_colliders(&mut self, entity: EntityID) {
        let vec = self.colliders.get_mut(&entity).unwrap();
        vec.clear();
    }
}
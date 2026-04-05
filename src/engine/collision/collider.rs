use std::cmp::PartialEq;
use glam::Vec2;
use crate::engine::EntityID;
use crate::engine::collision::CollisionType;
use crate::engine::world::World;

pub enum LayerType {
    On(u32),
    Collided(u32)
}

#[derive(Copy, Clone)]
pub enum ColliderType {
    Solid,
    Trigger,
}

pub struct Collider {
    entity: EntityID,
    id: String,
    layers_on: u32,
    layers_collided: u32,
    collider_type: ColliderType,
    is_static: bool,
    is_active: bool,
    width: u32,
    height: u32,
    offset: Vec2,
}

impl Collider {
    pub fn id(&self) -> &String {
        &self.id
    }
    
    pub fn add_layer(&mut self, mask: LayerType) {
        match mask {
            LayerType::Collided(mask) => {
                self.layers_collided = self.layers_collided | mask;
            }
            LayerType::On(mask) => {
                self.layers_on = self.layers_on | mask;
            }
        }
    }

    pub fn set_layer(&mut self, mask: LayerType) {
        match mask {
            LayerType::Collided(mask) => {
                self.layers_collided = mask;
            }
            LayerType::On(mask) => {
                self.layers_on = mask;
            }
        }
    }

    pub fn set_active(&mut self, state: bool) {
        self.is_active = state;
    }

    pub fn collide_against(&self, other: &Collider, world: &World) -> Result<CollisionType, String> {
        if (self.layers_collided & other.layers_on) == 0  || !other.is_active {
            return Ok(CollisionType::None);
        }

        let self_t;
        let other_t;

        if let Some(t) = world.get_transform(self.entity) {
            self_t = t;
        } else {
            return Err(format!("Could not get transform for entity {}", self.entity));
        }

        if let Some(t) = world.get_transform(other.entity) {
            other_t = t;
        } else {
            return Err(format!("Could not get transform for entity {}", other.entity));
        }

        let self_pos = if self.is_static {
            self.offset
        } else {
            self_t.position() + self.offset
        };
        let other_pos = if other.is_static {
            other.offset
        } else {
            other_t.position() + other.offset
        };

        let is_right = self_pos.x >= (other_pos.x + other.width as f32);
        let is_left = (self_pos.x + self.width as f32) <= other_pos.x;
        let is_above = self_pos.y >= (other_pos.y + other.height as f32);
        let is_below = (self_pos.y + self.height as f32) <= other_pos.y;

        if !(is_right || is_left || is_above || is_below) {
            return match (self.collider_type, other.collider_type) {
                (ColliderType::Solid, ColliderType::Solid) => {
                    Ok(CollisionType::Solid)
                }
                _ => Ok(CollisionType::Trigger),
            }
        }

        Ok(CollisionType::None)
    }
}
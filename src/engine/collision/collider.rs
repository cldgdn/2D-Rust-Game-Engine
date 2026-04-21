use crate::engine::collision::{CollisionInfo, CollisionType};
use crate::engine::world::World;
use crate::engine::EntityID;
use glam::{UVec2, Vec2, Vec3};
use crate::engine::config::config;

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
    pub entity: EntityID,
    pub id: String,
    pub layers_on: u32,
    pub layers_collided: u32,
    pub collider_type: ColliderType,
    pub is_static: bool,
    pub is_active: bool,
    pub width: u32,
    pub height: u32,
    pub offset: Vec2,
}

impl Collider {
    pub fn get_sections(&self, world: &World) -> Vec<(i32, i32)> {
        let transform = world.get_transform(self.entity).unwrap();
        let mut sections = Vec::new();
        
        let section_size = (config().collision_sector_size.0 as i32, config().collision_sector_size.1 as i32);
        let pos = transform.position_2d() + self.offset;
        
        let bl = pos;
        let tr = Vec2::new(pos.x + self.width as f32, pos.y + self.height as f32);
        
        let bl_section = (bl.x as i32 / section_size.0, bl.y as i32 / section_size.1);
        let tr_section = (tr.x as i32 / section_size.0, tr.y as i32 / section_size.1);
        
        let dx = bl_section.0 - tr_section.0;
        let dy = bl_section.1 - tr_section.1;
        
        for i in 0..=dx.abs() {
            for j in 0..=dy.abs() {
                sections.push((bl_section.0 + i, bl_section.1 + j));
            }
        }
        
        sections
    }
    
    pub fn collide_against(&self, other: &Collider, world: &World) -> Result<Option<CollisionInfo>, String> {
        if (self.layers_collided & other.layers_on) == 0 || !other.is_active || !self.is_active {
            return Ok(None);
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

        let self_pos = self_t.position_2d() + self.offset;
        let other_pos = other_t.position_2d() + other.offset;

        let dx1 = (other_pos.x + other.width as f32) - self_pos.x;
        let dx2 = (self_pos.x + self.width as f32) - other_pos.x;

        let dy1 = (other_pos.y + other.height as f32) - self_pos.y;
        let dy2 = (self_pos.y + self.height as f32) - other_pos.y;

        let overlap_x = dx1.min(dx2);
        let overlap_y = dy1.min(dy2);

        if overlap_x <= 0.0 || overlap_y <= 0.0 {
            return Ok(None);
        }

        let normal;
        if overlap_x < overlap_y {
            if self_pos.x < other_pos.x {
                normal = Vec2::new(-1.0, 0.0);
            } else {
                normal = Vec2::new(1.0, 0.0);
            }
        } else {
            if self_pos.y < other_pos.y {
                normal = Vec2::new(0.0, -1.0);
            } else {
                normal = Vec2::new(0.0, 1.0);
            }
        }

        let penetration = Vec2::new(overlap_x, overlap_y);

        let collision_type = match (self.collider_type, other.collider_type) {
            (ColliderType::Solid, ColliderType::Solid) => {
                CollisionType::Solid(self.layers_collided & other.layers_on)
            }
            _ => CollisionType::Trigger(self.layers_collided & other.layers_on)
        };

        Ok(Some(CollisionInfo {
            collision_type,
            normal,
            penetration
        }))
    }
}
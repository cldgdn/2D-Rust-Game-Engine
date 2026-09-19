use glam::Vec2;
use crate::engine::component::transform::Transform;
use crate::engine::component::UpdatingFixed;
use crate::engine::EntityID;
use crate::engine::world::World;

pub struct GroundProperties {
    pub velocity: Vec2,
}

impl GroundProperties {
    pub fn new() -> GroundProperties {
        Self {
            velocity: Vec2::ZERO,
        }
    }
}
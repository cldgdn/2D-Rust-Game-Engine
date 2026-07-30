use glam::{Vec2, Vec4};
use crate::engine::{Entity, EntityID};
use crate::engine::collision::{ColliderID, LayerMask};
use crate::engine::component::collider::ColliderType;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RaycastMode {
    Nearest,
    All
}

#[derive(Clone, Debug)]
pub struct RaycastRequest {
    pub source_id: EntityID,
    pub origin: Vec2,
    pub direction: Vec2,
    pub max_distance: f32,
    pub layers_collided: LayerMask,
    pub mode: RaycastMode,
    pub debug_color: Option<Vec4>
}

#[derive(Clone, Copy, Debug)]
pub struct RaycastHit {
    pub collider: ColliderID,
    pub collider_type: ColliderType,
    pub point: Vec2,
    pub normal: Vec2,
    pub distance: f32,
}
use glam::{Mat4, Vec2};

pub mod shader;
pub mod batcher;

pub struct RenderContext {
    pub camera_pos: Vec2,
    pub view_projection: Mat4,
}
use glam::{Mat4, Vec2, Vec4};
use crate::engine::EntityID;
use crate::engine::rendering::shader::ShaderID;

pub mod shader;
pub mod batcher;

pub struct RenderItem {
    pub entity_id: EntityID,
    pub shader_id: ShaderID,
    pub texture_id: glow::Texture,
    pub tex_offset: Vec2,
    pub region_size: Vec2,
    pub max_uvs: Vec2,
    pub z_index: i32,
    pub color_tint: Vec4,
    pub extended_data: Option<Vec<u8>>
}
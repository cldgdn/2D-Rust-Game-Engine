use glam::{Vec2, Vec4};
use crate::engine::EntityID;
use crate::engine::rendering::shader::ShaderID;

pub mod shader;
pub mod batched_renderer;
pub mod texture;
pub mod pass;
pub mod post_process;
pub mod framebuffer;
pub mod post_process_renderer;
pub mod renderer;

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

pub struct UVData {
    pub tex_offset: Vec2,
    pub region_size: Vec2,
    pub region_size_uv: Vec2,
}

pub struct TextureRenderData {
    pub texture_id: glow::Texture,
    pub tex_offset: Vec2,
    pub region_size: Vec2,
    pub max_uvs: Vec2,
}

pub trait TextureProvider {
    fn get_texture_render_data(&self) -> Option<TextureRenderData>;
}

impl TextureRenderData {
    pub fn flip_x(&mut self) {
        self.tex_offset.x += self.region_size.x;
        self.region_size.x *= -1.0;
    }

    pub fn flip_y(&mut self) {
        self.tex_offset.y += self.region_size.y;
        self.region_size.y *= -1.0;
    }
}
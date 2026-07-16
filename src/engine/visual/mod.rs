use glam::Vec2;

pub mod animation;
pub mod texture;
pub mod textured_quad;

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


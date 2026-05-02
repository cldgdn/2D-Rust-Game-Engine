use glam::Vec2;

pub mod animation;
pub mod texture;
pub mod textured_quad;

pub struct UVData {
    pub tex_offset: Vec2,
    pub region_size: Vec2,
    pub region_size_uv: Vec2,
}
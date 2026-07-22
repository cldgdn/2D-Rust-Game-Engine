use std::rc::Rc;
use glam::Vec2;
use crate::engine::config::config;
use crate::engine::rendering::{TextureProvider, TextureRenderData, UVData};
use crate::engine::rendering::texture::Texture2D;

pub struct TexturedQuad {
    uv_data: UVData,
    width: u32,
    height: u32,
    texture: Rc<Texture2D>,
}

impl TexturedQuad {
    pub fn new (uv_data: UVData, width: u32, height: u32, texture: Rc<Texture2D>) -> Self {
        Self {
            width, height, texture, uv_data,
        }
    }
    
    pub fn texture(&self) -> &Texture2D { &self.texture }
}

impl TextureProvider for TexturedQuad {
    fn get_texture_render_data(&self) -> Option<TextureRenderData> {
        Some(TextureRenderData {
            texture_id: self.texture.id(),
            tex_offset: self.uv_data.tex_offset,
            region_size: self.uv_data.region_size_uv,
            max_uvs: Vec2::new(
                (self.width * config().unit_pixel_size) as f32 / self.uv_data.region_size.x,
                (self.height * config().unit_pixel_size) as f32 / self.uv_data.region_size.y,
            )
        })
    }
}
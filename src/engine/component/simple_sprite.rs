use std::path::Path;
use std::rc::Rc;
use glam::Vec2;
use crate::engine::rendering::{TextureProvider, TextureRenderData};
use crate::engine::rendering::texture::Texture2D;
use crate::engine::world::World;

pub enum WrapMode {
    Tile(Vec2),
    Stretch
}

pub struct SimpleSprite {
    texture: Rc<Texture2D>,
    trd: TextureRenderData,
}

impl SimpleSprite {
    pub fn new(world: &mut World, path: impl AsRef<Path>, texture_offset: Vec2, size: Vec2) -> Result<Self, String> {
        let gl = world.gl();
        let texture_manager = world.texture_manager_mut();
        let texture = texture_manager.load(gl, path)?;
        let tex_width = texture.width();
        let tex_height = texture.height();

        let texture_render_data = TextureRenderData {
            texture_id: texture.id(),
            tex_offset: Vec2::new(
                texture_offset.x / tex_width as f32,
                texture_offset.y / tex_height as f32
            ),
            region_size: Vec2::new(
                size.x / tex_width as f32,
                size.y / tex_height as f32
            ),
            max_uvs: Vec2::ONE
        };

        Ok(Self { texture, trd: texture_render_data })
    }

    pub fn set_wrapping_mode(&mut self, mode: WrapMode) {
        match mode {
            WrapMode::Tile(repeat) => {
                self.trd.max_uvs = repeat;
            },
            WrapMode::Stretch => {
                self.trd.max_uvs = Vec2::ONE;
            }
        }
    }

    pub fn texture_size(&self) -> Vec2 {
        Vec2::new(
            self.texture.width() as f32,
            self.texture.height() as f32
        )
    }
    
    pub fn pixel_region_size(&self) -> Vec2 {
        Vec2::new(
            self.texture.width() as f32 * self.trd.region_size.x,
            self.texture.height() as f32 * self.trd.region_size.y,
        )
    }
}

impl TextureProvider for SimpleSprite {
    fn get_texture_render_data(&self) -> Option<TextureRenderData> {
        Some(self.trd.clone())
    }
}
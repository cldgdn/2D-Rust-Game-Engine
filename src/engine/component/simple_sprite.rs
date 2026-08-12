use std::path::Path;
use std::rc::Rc;
use glam::Vec2;
use crate::engine::rendering::{TextureProvider, TextureRenderData};
use crate::engine::rendering::texture::Texture2D;
use crate::engine::world::World;

/// Defines the behavior of the drawn texture when rendering
pub enum WrapMode {
    /// (r) -> Makes the texture repeat r.x times horizontally, while r.y times vertically, 
    /// across the area given by the Entity's transform.
    Tile(Vec2),
    /// Ensures the texture is drawn without repeating across the area given by the Entity's transform,
    /// potentially stretching or squishing along either axis if necessary.
    Stretch
}
/// Used to draw a non-animated sprite, pulling from a texture.
pub struct SimpleSprite {
    texture: Rc<Texture2D>,
    trd: TextureRenderData,
}

impl SimpleSprite {
    /// Creates the sprite by asking the texture manager for the texture identified by the given path.
    /// 
    /// texture_offset represents the distance in pixels from the bottom-left corner of the image to 
    /// the bottom-left corner of the portion of it that needs to be drawn.
    /// 
    /// size represents the size in pixels of the subregion of the texture that needs to be drawn.
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
    /// Sets the wrap mode of the texture. Default is Stretch
    pub fn with_wrap_mode(mut self, mode: WrapMode) -> Self {
        match mode {
            WrapMode::Tile(repeat) => {
                self.trd.max_uvs = repeat;
            },
            WrapMode::Stretch => {
                self.trd.max_uvs = Vec2::ONE;
            }
        }
        
        self
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
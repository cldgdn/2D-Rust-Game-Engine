use glam::{Vec2};
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
pub mod text;

pub struct LocalTransform {
    pub offset: Vec2,
    pub rotation: f32,
    pub pixel_size: Vec2,
}

pub struct RenderItem {
    /// ID of the Entity which issued this item.
    pub entity_id: EntityID,
    /// ID of the requested shader for this item on this frame.
    pub shader_id: ShaderID,
    /// GL texture binding
    pub texture_id: glow::Texture,
    /// UV offset at which to start sampling in the texture (becomes local (0.0, 0.0))
    pub tex_offset: Vec2,
    /// Extent of the UV subregion of the texture that is sampled
    pub region_size: Vec2,
    /// Local UV values at the TR corner of the quad.
    /// Values > 1.0 begin repeating the subregion across the quad
    pub max_uvs: Vec2,
    pub z_index: i32,
    pub extended_data: Option<Vec<u8>>,
    /// Optional local transform information for the RenderItem, useful for objects which
    /// submit multiple items.
    pub local_transform: Option<LocalTransform>,
}

#[derive(Copy, Clone, Debug)]
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
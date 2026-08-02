use glam::{Vec2, Vec4};
use crate::engine::component::transform::Transform;
use crate::engine::{mask_bit, Drawable, Entity, EntityID, Updatable};
use crate::engine::input::Input;
use crate::engine::rendering::{RenderItem, TextureProvider, UVData};
use crate::engine::rendering::shader::ShaderID;
use crate::engine::component::textured_quad::TexturedQuad;
use crate::engine::rendering::pass::PassMask;
use crate::engine::world::{Intention, PostFxIntention, RenderPassIntention, World};

pub struct Background {
    id: EntityID,
    texture: TexturedQuad,
    current_shader: ShaderID,
    pub z_index: i32,
}

impl Background {
    pub fn new(world: &mut World, image_path: &str, width: u32, height: u32) -> Background {
        let id = world.get_next_entity_id();
        let mut transform = Transform::new();

        transform.set_size(Vec2::new(width as f32, height as f32));

        world.add_component(id, transform);

        let gl = world.gl().clone();
        let tex = world.texture_manager_mut()
            .load(
                gl.clone(),
                image_path,
            ).unwrap();

        let tex_offset_pixels = Vec2::new(0.0, 0.0);

        let uv_data = UVData {
            tex_offset: Vec2::new(
                tex_offset_pixels.x / tex.width() as f32,
                tex_offset_pixels.y / tex.height() as f32,
            ),
            region_size: Vec2::new(
                64.0,
                64.0,
            ),
            region_size_uv: Vec2::new(
                64.0 / tex.width() as f32,
                64.0 / tex.height() as f32,
            ),
        };

        let quad = TexturedQuad::new(
            uv_data,
            width, height,
            tex
        );

        let current_shader = world
            .shader_id("default")
            .expect("Failed to get shader id.");

        Background {
            id,
            texture: quad,
            current_shader,
            z_index: 0,
        }
    }
}

impl Entity for Background {
    fn id(&self) -> EntityID {
        self.id
    }

    fn as_updatable_mut(&mut self) -> Option<&mut dyn Updatable> {
        Some(self)
    }

    fn as_drawable(&self) -> Option<&dyn Drawable> {
        Some(self)
    }
}

impl Updatable for Background {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        let action = input.is_action_pressed("interact").active;
        if action {
            world.intention_queue.push(
                Intention::RenderPass(
                    RenderPassIntention::PostFx(
                        "world".to_string(),
                        PostFxIntention::ToggleEnabled("greenshift".to_string())
                    )
                )
            );
        }
    }
}

impl Drawable for Background {
    fn entity_id(&self) -> EntityID {
        self.id
    }

    fn shader_id(&self) -> ShaderID {
        self.current_shader
    }

    fn z_index(&self) -> i32 {
        self.z_index
    }

    fn get_render_item(&self, world: &World) -> Option<RenderItem> {
        let _transform = world.get_component::<Transform>(self.id)?;

        let tex_data = self.texture.get_texture_render_data()?;
        
        Some(
            RenderItem {
                entity_id: self.id,
                shader_id: self.current_shader,
                texture_id: tex_data.texture_id,
                tex_offset: tex_data.tex_offset,
                region_size: tex_data.region_size,
                max_uvs: tex_data.max_uvs,
                z_index: self.z_index,
                color_tint: Vec4::ONE,
                extended_data: None,
            }
        )
    }

    fn pass_mask(&self) -> PassMask {
        mask_bit(1)
    }
}
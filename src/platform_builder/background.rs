use glam::{Vec2, Vec4};
use crate::engine::component::transform::{Mirror, Transform};
use crate::engine::{mask_bit, Drawable, Entity, EntityID, Updatable};
use crate::engine::input::Input;
use crate::engine::rendering::{RenderItem, TextureProvider};
use crate::engine::rendering::shader::ShaderID;
use crate::engine::component::simple_sprite::{SimpleSprite, WrapMode};
use crate::engine::config::config;
use crate::engine::rendering::pass::PassMask;
use crate::engine::world::{Intention, PostFxIntention, RenderPassIntention, World};

pub struct Background {
    id: EntityID,
    current_shader: ShaderID,
    pub z_index: i32,
}

impl Background {
    pub fn new(world: &mut World, image_path: &str, width: u32, height: u32) -> Background {
        let id = world.get_next_entity_id();

        let mut transform = Transform::new();
        transform.set_size(Vec2::new(width as f32, height as f32));

        let mut sprite = SimpleSprite::new(
            world,
            image_path,
            Vec2::ZERO,
            Vec2::new(64.0, 64.0)
        ).expect("Failed to create background sprite");
        let pixel_size = transform.pixel_dimensions();
        let region_size = sprite.pixel_region_size();
        sprite.set_wrapping_mode(
            WrapMode::Tile(
                Vec2::new(
                    pixel_size.x / region_size.x,
                    pixel_size.y / region_size.y,
                )
            )
        );

        world.add_component(id, transform);
        world.add_component(id, sprite);

        let current_shader = world
            .shader_id("default")
            .expect("Failed to get shader id.");

        Background {
            id,
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
        let sprite = world.get_component::<SimpleSprite>(self.id)?;

        let tex_data = sprite.get_texture_render_data()?;
        
        Some(
            RenderItem {
                entity_id: self.id,
                shader_id: self.current_shader,
                texture_id: tex_data.texture_id,
                tex_offset: tex_data.tex_offset,
                region_size: tex_data.region_size,
                max_uvs: tex_data.max_uvs,
                z_index: self.z_index,
                extended_data: None,
                local_transform: None
            }
        )
    }

    fn pass_mask(&self) -> PassMask {
        mask_bit(1)
    }
}
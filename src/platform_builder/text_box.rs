use std::fmt::format;
use glam::{Vec2, Vec4};
use crate::engine::component::transform::Transform;
use crate::engine::{mask_bit, Drawable, Entity, EntityID, Updatable};
use crate::engine::component::text_block::TextBlock;
use crate::engine::config::config;
use crate::engine::input::Input;
use crate::engine::rendering::pass::PassMask;
use crate::engine::rendering::{LocalTransform, RenderItem, TextureProvider};
use crate::engine::rendering::shader::ShaderID;
use crate::engine::rendering::text::{FontID, FontLineMetrics, Glyph, GlyphMetrics};
use crate::engine::world::World;

pub struct TextBox {
    id: EntityID,
    current_shader: ShaderID,
    pub z_index: i32,
    pen_start: Vec2,
    displayed_characters: i32,
    text_increment_time: f32,
    timer: f32,
}

impl TextBox {
    pub fn new(world: &mut World, text: &str, font_id: FontID, size_px: f32) -> TextBox {
        let id = world.get_next_entity_id();
        let mut transform = Transform::new();
        transform.set_size(Vec2::new(4.0, 2.0));
        
        let text_block = TextBlock::new(world, text, font_id, size_px)
            .map_err(|e| {
                format!("Could not create text block: {:?}", e);
            })
            .unwrap()
            .with_wrapping(None, Some(100.0));
        
        world.add_component(id, transform);
        world.add_component(id, text_block);

        let shader_id = world.shader_id("text").expect("Could not get text shader");

        Self {
            id,
            current_shader: shader_id,
            z_index: 0,
            pen_start: Vec2::ZERO,
            displayed_characters: 0,
            text_increment_time: 0.05,
            timer: 0.0,
        }
    }
}

impl Entity for TextBox {
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

impl Updatable for TextBox {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        if self.displayed_characters == -1 {
            return;
        }

        let text_block = world.get_component::<TextBlock>(self.id).unwrap();
        
        self.timer += delta_time;

        if self.timer >= self.text_increment_time {
            self.timer = 0.0;
            self.displayed_characters += 1;
        }

        if input.is_action_pressed("right").active {
            self.displayed_characters += 1;
        }
        if input.is_action_pressed("left").active {
            self.displayed_characters -= 1;
        }

        if self.displayed_characters < 0 {
            self.displayed_characters = text_block.text.len() as i32;
        }
        else if self.displayed_characters > text_block.text.len() as i32 {
            self.displayed_characters = 0;
        }
    }
}

impl Drawable for TextBox {
    fn entity_id(&self) -> EntityID {
        self.id
    }

    fn shader_id(&self) -> ShaderID {
        self.current_shader
    }

    fn z_index(&self) -> i32 {
        self.z_index
    }

    fn collect_render_items(&self, world: &World, items: &mut Vec<RenderItem>) {
        let text_block = world.get_component::<TextBlock>(self.id).unwrap();
        
        let mut local_transforms = text_block.get_local_transforms(self.pen_start, 0);
        let mut chars = 0;

        for glyph in text_block.glyphs.iter() {
            if self.displayed_characters >= 0 && chars >= self.displayed_characters {
                break;
            }

            chars += 1;

            let t = local_transforms.remove(0);
            if t.is_none() {
                continue;
            }

            let Some(tex) = glyph.get_texture_render_data() else {
                continue;
            };


            let tint = Vec4::new(0.0, 0.0, chars as f32 / text_block.text.len() as f32, 1.0);
            let extended_data = Some(bytemuck::bytes_of(&tint).to_vec());
            
            items.push(RenderItem {
                entity_id: self.id,
                shader_id: self.current_shader,
                texture_id: tex.texture_id,
                tex_offset: tex.tex_offset,
                region_size: tex.region_size,
                max_uvs: tex.max_uvs,
                z_index: self.z_index,
                extended_data,
                local_transform: t,
            });
        }
    }


    fn pass_mask(&self) -> PassMask {
        mask_bit(2)
    }
}
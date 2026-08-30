use std::any::Any;
use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::engine::component::transform::Transform;
use crate::engine::{mask_bit, mask_bits, Drawable, Entity, EntityID};
use crate::engine::collision::{Collides, CollisionSystem};
use crate::engine::collision::shape::Shape;
use crate::engine::component::collider::{Collider};
use crate::engine::component::Multi;
use crate::engine::rendering::{RenderItem, TextureProvider};
use crate::engine::rendering::shader::ShaderID;
use crate::engine::component::simple_sprite::{SimpleSprite, WrapMode};
use crate::engine::rendering::pass::PassMask;
use crate::engine::world::World;
use crate::platform_builder::events::SomeEvent;

#[derive(Serialize, Deserialize)]
pub struct WallInit {
    pub image_path: String,
    pub starting_position: (f32, f32),
    pub width: u32,
    pub height: u32,
    /// In degrees
    pub rotation: Option<f32>,
}

pub struct Wall {
    id: EntityID,
    current_shader: ShaderID,
    pub z_index: i32,
}

impl Wall {
    pub fn new(id: EntityID, init: WallInit, world: &mut World) -> Wall {
        let WallInit {
            image_path,
            starting_position,
            width,
            height,
            rotation,
        } = init;
        
        let mut transform = Transform::new();

        transform.set_position(Vec2::from(starting_position));
        transform.set_size(Vec2::new(width as f32, height as f32));
        if let Some(rotation) = rotation {
            transform.set_rotation(rotation);
        }

        let sprite = SimpleSprite::new(
            world,
            image_path,
            Vec2::ZERO,
            Vec2::new(16.0, 16.0)
        ).expect("Could not create sprite");

        let pixel_size = transform.pixel_size();
        let region_size = sprite.pixel_region_size();

        let sprite = sprite.with_wrap_mode(
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
        
        world.event_bus.subscribe::<SomeEvent>(id);
        
        Wall {
            id,
            current_shader,
            z_index: 0,
        }
    }
    
    pub fn from_init(id: EntityID, init: Value, world: &mut World) -> Box<dyn Entity> {
        let params: WallInit = serde_json::from_value(init).expect("Failed to parse WallInit");
        Box::new(Wall::new(id, params, world))
    }
}

impl Entity for Wall {
    fn id(&self) -> EntityID {
        self.id
    }

    fn on_event(&mut self, event: &dyn Any, world: &mut World) {
        if event.is::<SomeEvent>() {
            println!("SomeEvent has happened!");
        }
        
        let p_id = world.get_entity_by_name("player").unwrap();
        let p_transform: &Transform = world.get_component(p_id).unwrap();
        let pos = p_transform.position();
        println!("Player position at: ({}, {})", pos.x, pos.y);
    }

    fn as_drawable(&self) -> Option<&dyn Drawable> {
        Some(self)
    }

    fn as_collides(&self) -> Option<&dyn Collides> {
        Some(self)
    }

    fn as_collides_mut(&mut self) -> Option<&mut dyn Collides> {
        Some(self)
    }
}

impl Drawable for Wall {
    fn entity_id(&self) -> EntityID {
        self.id
    }

    fn get_render_item(&self, world: &World) -> Option<RenderItem> {
        let sprite = world.get_component::<SimpleSprite>(self.id)?;

        let tex_data = sprite.get_texture_render_data()?;

        Some (
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

impl Collides for Wall {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem, world: &mut World) {
        let transform = world.get_component::<Transform>(self.id).unwrap();
        let size = transform.size();
        let mut shapes = Vec::new();
        shapes.push(Shape::Rectangle {
            half_extents: size / 2.0,
            rotation_offset: transform.rotation().to_radians(),
            offset: Vec2::ZERO,
        });

        let main = Collider::new(self.id, 0, shapes, world.gl())
            .with_layers(mask_bits(&[1, 2]), 0)
            .with_collision_space(mask_bit(0))
            .with_debug_color(Vec4::new(1.0, 0.0, 0.0, 1.0))
            .make_static();

        let mut colliders = Multi::new();

        colliders.insert(main.id, main);

        world.add_component(self.id, colliders);

        collision_system.add_static_colliders(self.id, world);
    }
}
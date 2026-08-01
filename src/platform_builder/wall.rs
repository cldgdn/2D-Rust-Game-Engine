use std::any::Any;
use glam::{Vec2, Vec4};
use crate::engine::component::transform::Transform;
use crate::engine::{Drawable, Entity, EntityID};
use crate::engine::collision::{Collides, CollisionSystem};
use crate::engine::collision::shape::Shape;
use crate::engine::component::collider::{Collider, ColliderType};
use crate::engine::component::Multi;
use crate::engine::rendering::{RenderItem, TextureProvider, UVData};
use crate::engine::rendering::shader::ShaderID;
use crate::engine::component::textured_quad::TexturedQuad;
use crate::engine::rendering::pass::{pass_bit, PassMask};
use crate::engine::world::World;
use crate::platform_builder::events::SomeEvent;

pub struct Wall {
    id: EntityID,
    texture: TexturedQuad,
    current_shader: ShaderID,
    width: u32,
    height: u32,
    pub z_index: i32,
}

impl Wall {
    pub fn new(image_path: &str, width: u32, height: u32, world: &mut World) -> Wall {
        let id = world.get_next_entity_id();
        let mut transform = Transform::new();

        transform.set_size(Vec2::new(width as f32, height as f32));

        world.add_component(id, transform);

        let gl = world.gl().clone();
        let tex = world.texture_manager_mut().load(
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
                16.0,
                16.0,
            ),
            region_size_uv: Vec2::new(
                16.0 / tex.width() as f32,
                16.0 / tex.height() as f32,
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
        
        world.event_bus.subscribe::<SomeEvent>(id);
        
        Wall {
            id,
            texture: quad,
            width,
            height,
            current_shader,
            z_index: 0,
        }
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
    
    fn shader_id(&self) -> ShaderID {
        self.current_shader
    }

    fn z_index(&self) -> i32 {
        self.z_index
    }

    fn get_render_item(&self, world: &World) -> Option<RenderItem> {
        let _transform = world.get_component::<Transform>(self.id)?;

        let tex_data = self.texture.get_texture_render_data()?;

        Some (
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
        pass_bit(2)
    }
}

impl Collides for Wall {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem, world: &mut World) {
        let transform = world.get_component::<Transform>(self.id).unwrap();
        let mut shapes = Vec::new();
        shapes.push(Shape::Rectangle {
            half_extents: Vec2::new(self.width as f32 / 2.0, self.height as f32 / 2.0),
            rotation_offset: transform.rotation().to_radians(),
            offset: Vec2::ZERO,
        });

        let main = Collider::new(
            self.id,
            0,
            2+8,
            0,
            ColliderType::Solid,
            true,
            true,
            shapes,
            Vec2::ZERO,
            0.0,
            Some(Vec4::new(1.0, 0.0, 0.0, 1.0)),
            world.gl()
        );

        let mut colliders = Multi::new();

        colliders.insert(main.id, main);

        world.add_component(self.id, colliders);

        collision_system.add_static_colliders(self.id, world);
    }
}
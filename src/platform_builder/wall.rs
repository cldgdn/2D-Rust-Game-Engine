use std::any::Any;
use glam::{Mat4, Vec2, Vec4};
use crate::engine::component::transform::Transform;
use crate::engine::{Drawable, Entity, EntityID};
use crate::engine::collision::{Collides, Collision, CollisionEvent, CollisionSystem};
use crate::engine::collision::collider::{Collider, ColliderType};
use crate::engine::config::config;
use crate::engine::rendering::RenderItem;
use crate::engine::rendering::shader::{Shader, ShaderID};
use crate::engine::visual::texture::TileMode;
use crate::engine::visual::textured_quad::TexturedQuad;
use crate::engine::visual::{TextureProvider, UVData};
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
}

impl Collides for Wall {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem, world: &mut World) {
        let main = Collider::new(
            self.id,
            "wall",
            2+8,
            0,
            ColliderType::Solid,
            true,
            true,
            self.width as f32,
            self.height as f32,
            Vec2::ZERO,
            Some(Vec4::new(1.0, 0.0, 0.0, 1.0)),
            world.gl(),
        );

        collision_system.add_collider(self.id, main, world);
    }
}
use std::any::Any;
use glam::{Mat4, Vec2, Vec4};
use crate::engine::component::transform::Transform;
use crate::engine::{Drawable, Entity, EntityID};
use crate::engine::collision::{Collides, Collision, CollisionEvent, CollisionSystem};
use crate::engine::collision::collider::{Collider, ColliderType};
use crate::engine::shader::{Shader, ShaderID};
use crate::engine::visual::texture::TileMode;
use crate::engine::visual::textured_quad::TexturedQuad;
use crate::engine::visual::UVData;
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
        let transform = Transform::new();
        
        world.add_transform(id, transform);

        let gl = world.gl().clone();
        let tex = world.texture_manager()
            .load(
                TileMode::Tile,
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
            gl,
            tex
        ).unwrap();

        let current_shader = world
            .shader_id("camera")
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

    fn draw_geometry(&self, shader: &Shader, world: &World) {
        self.texture.draw(shader);
    }
    
    fn shader(&self) -> ShaderID {
        self.current_shader
    }

    fn z_index(&self) -> i32 {
        self.z_index
    }
}

impl Collides for Wall {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem, world: &mut World) {
        let main = Collider::new(
            self.id,
            "wall",
            1,
            0,
            ColliderType::Solid,
            true,
            true,
            self.width,
            self.height,
            Vec2::ZERO,
            Some(Vec4::new(1.0, 0.0, 0.0, 1.0)),
            world.gl(),
        );

        collision_system.add_collider(self.id, main, world);
    }
}
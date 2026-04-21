use glam::{Mat4, Vec2, Vec4};
use crate::engine::component::transform::Transform;
use crate::engine::{Drawable, Entity, EntityID};
use crate::engine::collision::{Collides, Collision, CollisionEvent, CollisionSystem};
use crate::engine::collision::collider::{Collider, ColliderType};
use crate::engine::shader::{Shader, ShaderID};
use crate::engine::texture::TileMode;
use crate::engine::textured_quad::{TexturedQuad, UVData};
use crate::engine::world::World;

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
    fn draw(&self, shader: &Shader, world: &World) {
        let transform = if let Some(t) = world.get_transform(self.id) {
            t
        } else {
            return;
        };

        let position = transform.position();
        let scale = transform.scale();
        let model = Mat4::from_translation(position.clone());
        let model = model.mul_diagonal_scale(Vec4::new(scale.x, scale.y, 1.0, 1.0));

        shader.set_mat4("uModel", &model);

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
        let main = Collider {
            entity: self.id,
            id: "wall".to_string(),
            layers_on: 1,
            layers_collided: 0,
            collider_type: ColliderType::Solid,
            is_static: true,
            is_active: true,
            width: self.width,
            height: self.height,
            offset: Vec2::ZERO,
        };

        collision_system.add_collider(self.id, main, world);
    }

    fn react_to_collision(&mut self, collision: CollisionEvent, world: &mut World) {
        ()
    }
}
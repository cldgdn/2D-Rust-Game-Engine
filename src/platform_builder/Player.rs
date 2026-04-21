use glam::{Mat4, Vec2, Vec3, Vec4};
use crate::engine::{Drawable, Entity, EntityID};
use crate::engine::collision::{Collides, Collision, CollisionEvent, CollisionSystem};
use crate::engine::collision::collider::{Collider, ColliderType};
use crate::engine::component::transform::Transform;
use crate::engine::Updatable;
use crate::engine::input::Input;
use crate::engine::shader::{Shader, ShaderID};
use crate::engine::texture::TileMode;
use crate::engine::textured_quad::{TexturedQuad, UVData};
use crate::engine::world::World;

pub struct Player {
    id: EntityID,
    texture: TexturedQuad,
    current_shader: ShaderID,
    pub z_index: i32,
    pub is_active: bool,
    pub velocity: Vec2,
}

impl Player {
    pub fn new(world: &mut World) -> Player {
        let id = world.get_next_entity_id();
        let transform = Transform::new();

        world.add_transform(id, transform);

        let gl = world.gl().clone();
        let tex = world.texture_manager()
            .load(
                TileMode::Stretch,
                gl.clone(),
                "resources/textures/truck.png"
            ).unwrap();

        let tex_offset_pixels = Vec2::new(16.0, 32.0);

        let uv_data = UVData {
            tex_offset: Vec2::new(
                0.0, 0.0
            ),
            region_size: Vec2::new(
                32.0,
                64.0,
            ),
            region_size_uv: Vec2::new(
                32.0 / tex.width() as f32,
                64.0 / tex.height() as f32,
            ),
        };

        let quad = TexturedQuad::new(
            uv_data,
            32, 64,
            gl,
            tex
        ).unwrap();

        let current_shader = world
            .shader_id("camera")
            .expect("Failed to get shader id");

        Player {
            id,
            texture: quad,
            current_shader,
            is_active: true,
            z_index: 0,
            velocity: Vec2::new(0.0, 0.0),
        }
    }
}

impl Entity for Player {
    fn id(&self) -> EntityID {
        self.id
    }

    fn as_updatable_mut(&mut self) -> Option<&mut dyn Updatable> {
        Some(self)
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

impl Updatable for Player {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        if !self.is_active {
            return;
        }

        let transform = world.get_transform(self.id).unwrap();

        let mut direction = Vec2::new(0.0, 0.0);

        if input.is_action_down("walk_right") {
            direction.x += 1.0;
        }

        if input.is_action_down("walk_left") {
            direction.x -= 1.0;
        }

        if input.is_action_down("walk_up") {
            direction.y += 1.0;
        }

        if input.is_action_down("walk_down") {
            direction.y -= 1.0;
        }

        if input.is_action_pressed("interact") {
            let pos = transform.position_2d();
            println!("{pos}");
        }

        self.velocity = direction * 30.0;
    }

    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        if !self.is_active {
            return;
        }

        let transform = world.get_mut_transform(self.id).unwrap();
        transform.translate(self.velocity * delta_time);
    }
}

impl Drawable for Player {
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

impl Collides for Player {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem, world: &mut World) {
        let main = Collider {
            entity: self.id,
            id: "main".to_string(),
            layers_on: 1,
            layers_collided: 1,
            collider_type: ColliderType::Trigger,
            is_static: false,
            is_active: true,
            width: 32,
            height: 64,
            offset: Vec2::ZERO,
        };

        let solid = Collider {
            entity: self.id,
            id: "solid".to_string(),
            layers_on: 1,
            layers_collided: 1,
            collider_type: ColliderType::Solid,
            is_static: false,
            is_active: true,
            width: 32,
            height: 64,
            offset: Vec2::ZERO,
        };

        collision_system.add_collider(self.id, main, world);
        collision_system.add_collider(self.id, solid, world);
    }

    fn react_to_collision(&mut self, collision: CollisionEvent, world: &mut World) {
        ()
    }
}
use glam::{Mat4, Vec3, Vec4};
use crate::engine::component::transform::Transform;
use crate::engine::{Drawable, Entity, EntityID, Updatable};
use crate::engine::input::Input;
use crate::engine::shader::{Shader, ShaderID};
use crate::engine::texture::TileMode;
use crate::engine::textured_quad::TexturedQuad;
use crate::engine::world::World;

pub struct Background {
    id: EntityID,
    texture: TexturedQuad,
    current_shader: ShaderID,
}

impl Background {
    pub fn new(world: &mut World, image_path: &str, width: u32, height: u32) -> Background {
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

        let quad = TexturedQuad::new(
            width, height,
            gl,
            tex
        ).unwrap();

        let current_shader = world
            .shader_id("camera")
            .expect("Failed to get shader id.");

        Background {
            id,
            texture: quad,
            current_shader,
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
        let transform = world
            .get_mut_transform(self.id)
            .expect("Could not find transform :(");

        let mut depth = transform.position().z;

        if input.is_action_pressed("up") {
            depth += 0.2;
        }

        if input.is_action_pressed("down") {
            depth -= 0.2;
        }

        let depth = depth.clamp(0.0, 1.0);
        transform.set_depth(depth);
    }
}

impl Drawable for Background {
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
}
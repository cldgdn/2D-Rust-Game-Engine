use std::any::Any;
use glam::{Mat4, Vec4};
use rendering::shader::Shader;
use world::World;
use crate::engine::collision::Collides;
use crate::engine::component::transform::Transform;
use crate::engine::input::Input;
use rendering::shader::ShaderID;

pub mod config;
pub mod game;
pub mod input;
pub mod world;
pub mod component;
pub mod collision;
pub mod entity;
pub mod camera;
pub mod visual;
pub mod event;
pub mod rendering;

pub type EntityID = u32;

pub enum Either<A, B> {
    Left(A),
    Right(B),
}

pub trait Entity {
    fn id(&self) -> EntityID;

    fn on_event(&mut self, event: &dyn Any, world: &mut World) {
        ()
    }
    
    fn as_updatable(&self) -> Option<&dyn Updatable> {
        None
    }
    
    fn as_updatable_mut(&mut self) -> Option<&mut dyn Updatable> {None}

    fn as_drawable(&self) -> Option<&dyn Drawable> {
        None
    }
    
    fn as_drawable_mut(&mut self) -> Option<&mut dyn Drawable> {None}

    fn as_collides(&self) -> Option<&dyn Collides> {
        None
    }
    
    fn as_collides_mut(&mut self) -> Option<&mut dyn Collides> {None}
}

pub trait Drawable {
    fn entity_id(&self) -> EntityID;

    fn draw(&self, shader: &Shader, world: &World) {
        let transform = if let Some(t) = world.get_transform(self.entity_id()) {
            t
        } else {
            return;
        };

        let model = compute_model_matrix(transform);
        let depth = transform.depth();

        shader.set_float("uDepth", depth);
        shader.set_mat4("uModel", &model);

        self.prepare_draw(shader, world);

        self.draw_geometry(shader, world);
    }

    fn prepare_draw(&self, shader: &Shader, world: &World) {
        ()
    }

    fn draw_geometry(&self, shader: &Shader, world: &World);
    
    fn shader(&self) -> ShaderID;
    
    fn z_index(&self) -> i32;
}

pub trait Updatable {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        ()
    }
    
    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        ()
    }
}

fn compute_model_matrix(transform: &Transform) -> Mat4 {
    let position = transform.position().clone().extend(0.0);
    let scale = transform.scale();
    let model = Mat4::from_translation(position);
    let model = model.mul_diagonal_scale(Vec4::new(scale.x, scale.y, 1.0, 1.0));
    model
}
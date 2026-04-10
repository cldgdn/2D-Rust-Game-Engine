use shader::Shader;
use world::World;
use crate::engine::collision::Collides;
use crate::engine::input::Input;
use crate::engine::shader::ShaderID;

pub mod shader;
pub mod config;
pub mod game;
pub mod texture;
pub mod textured_quad;
pub mod input;
pub mod world;
pub mod component;
pub mod collision;
pub mod entity;
pub mod camera;

pub type EntityID = u32;

pub trait Entity {
    fn id(&self) -> EntityID;
    
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
    fn draw(&self, shader: &Shader, world: &World);
    
    fn shader(&self) -> ShaderID;
    
    fn z_index(&self) -> i32;
}

pub trait Updatable {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32);
}
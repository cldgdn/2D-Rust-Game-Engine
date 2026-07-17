use std::any::Any;
use glam::{Mat4, Vec4};
use rendering::shader::Shader;
use world::World;
use crate::engine::collision::Collides;
use crate::engine::component::transform::Transform;
use crate::engine::input::Input;
use rendering::shader::ShaderID;
use crate::engine::rendering::RenderItem;

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
    
    fn shader_id(&self) -> ShaderID;
    
    fn z_index(&self) -> i32;

    fn get_render_item(&self, world: &World) -> Option<RenderItem>;
}

pub trait Updatable {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        ()
    }
    
    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        ()
    }
}
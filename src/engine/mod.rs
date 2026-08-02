use std::any::Any;
use world::World;
use crate::engine::collision::Collides;
use crate::engine::input::Input;
use rendering::shader::ShaderID;
use crate::engine::rendering::pass::{PassMask, DEFAULT_PASS_MASK};
use crate::engine::rendering::RenderItem;

pub mod config;
pub mod game;
pub mod input;
pub mod world;
pub mod component;
pub mod collision;
pub mod entity;
pub mod camera;
pub mod event;
pub mod rendering;

pub type EntityID = u32;

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

    fn collect_render_items(&self, world: &World, items: &mut Vec<RenderItem>) {
        if let Some(item) = self.get_render_item(world) {
            items.push(item);
        }
    }

    fn get_render_item(&self, world: &World) -> Option<RenderItem> {
        None
    }

    fn pass_mask(&self) -> PassMask {
        DEFAULT_PASS_MASK
    }
}

pub trait Updatable {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        ()
    }
    
    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        ()
    }
}

pub fn mask_bit(index: u32) -> u64 {
    assert!(index < 64, "Mask index must be less than 64. Attempted index: {}", index);
    1u64 << index
}
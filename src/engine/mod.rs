use std::any::Any;
use world::World;
use crate::engine::collision::Collides;
use crate::engine::input::Input;
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
pub mod sound;

pub type BitMask = u64;
pub type EntityID = u64;

/// Behavioral trait that defines a game Entity.
pub trait Entity {
    /// The Entity's own ID
    fn id(&self) -> EntityID;
    /// Hook for reacting to events the Entity is subscribed to
    fn on_event(&mut self, event: &dyn Any, world: &mut World) {
        ()
    }
    /// Exposes the Entity as implementing the Updatable trait
    /// It is unused by the engine itself, but is left here for any potential uses a user may need.
    fn as_updatable(&self) -> Option<&dyn Updatable> {
        None
    }
    /// Mutably exposes the Entity as implementing the Updatable trait
    fn as_updatable_mut(&mut self) -> Option<&mut dyn Updatable> {None}
    /// Exposes the Entity as implementing the Drawable trait
    fn as_drawable(&self) -> Option<&dyn Drawable> {
        None
    }
    /// Mutably exposes the Entity as implementing the Drawable trait
    /// It is unused by the engine itself, but is left here for any potential uses a user may need
    fn as_drawable_mut(&mut self) -> Option<&mut dyn Drawable> {None}
    /// Exposes the Entity as implementing the Collides trait
    fn as_collides(&self) -> Option<&dyn Collides> {
        None
    }
    /// Mutably exposes the entity as implementing the Collides trait
    fn as_collides_mut(&mut self) -> Option<&mut dyn Collides> {None}
}

/// Trait which defines rendering behavior for the Entity.
pub trait Drawable {
    /// The Drawable's own ID
    fn entity_id(&self) -> EntityID;
    /// Collects all the RenderItem-s the Drawable needs to issue into the items Vec.
    /// By default, it only collects what is returned by Drawable::get_render_item if it is Some.
    fn collect_render_items(&self, world: &World, items: &mut Vec<RenderItem>) {
        if let Some(item) = self.get_render_item(world) {
            items.push(item);
        }
    }
    /// Used whenever a Drawable will only ever need to issue one RenderItem.
    /// If None is returned, nothing is rendered.
    fn get_render_item(&self, world: &World) -> Option<RenderItem> {
        None
    }
    /// The bitmask which identifies which RenderPass-es this Drawable is issuing RenderItem-s for.
    /// By default, it returns engine::rendering::pass::DEFAULT_PASS_MASK
    fn pass_mask(&self) -> PassMask {
        DEFAULT_PASS_MASK
    }
}

/// Trait which defines update behavior for the Entity
pub trait Updatable {
    /// Called once per frame, best used for input detection, Entity state updates etc.
    /// Default does nothing.
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        ()
    }
    /// Called on a fixed timestep, best used for physics (moving, rotating, issuing raycasts, etc.)
    /// Default does nothing
    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        ()
    }
    /// Returns the priority value of this updatable, used to determine update order.
    ///
    /// Lowest priority goes first in order, EntityID is used as tiebreaker for entities with
    /// the same priority value, making the entity that spawned first go first in the order.
    ///
    /// Defaults to 0 if not overridden.
    fn priority(&self) -> i32 {
        0
    }
}

/// Returns a u64 bitmask where only the bit at index is set.
/// Returns 0 (an empty mask) if index is >= 64 (past the bit count)
pub fn mask_bit(index: u32) -> u64 {
    if index >= 64 {
        0
    } else {
        1u64 << index
    }
}

/// Returns a u64 bitmask where only the bit at each index in the array are set.
/// If any index is >= 64, it will be ignored in the final result, as it is past the bit count.
pub fn mask_bits(indexes: &[u32]) -> u64 {
    let mut mask = 0;
    for index in indexes.iter() {
        mask += mask_bit(*index);
    }
    mask
}
use std::collections::HashMap;
use crate::engine::EntityID;
use crate::engine::input::Input;
use crate::engine::world::World;

pub mod transform;
pub mod collider;
pub mod animation;
pub mod simple_sprite;
pub mod text_block;
pub mod event_channel;
pub mod update_gates;
pub mod timer;
pub mod links;
pub mod tween;

/// Automated Trait Behavior
///
/// A component implementing this trait will automatically be ticked on each frame.
pub trait Tickable: 'static {
    fn tick(&mut self, delta_time: f32);
    /// Whether this component should stay in its active list after this tick. Only consulted when
    /// the component type is registered with uses_active = true.
    ///
    /// This is only meaningful to check right after an actual tick() call in the same pass. If UGC
    /// gating skipped this instance, it stays in the active list regardless (it's not finished,
    /// just paused).
    fn is_active(&self) -> bool { true }
}

/// ATB
///
/// Same as Tickable, ticked on each physics frame
pub trait TickableFixed: 'static {
    fn tick(&mut self, delta_time: f32);
    fn is_active(&self) -> bool { true }
}

/// ATB
/// 
/// Called after the update dispatch loop, gives complete access to update information and state 
/// in order to perform logic.
pub trait Updating: 'static {
    fn update(&mut self, owner: EntityID, input: &Input, world: &mut World, delta_time: f32);
}

/// ATB
/// 
/// Called after the fixed update dispatch loop, gives complete access to fixed update information
/// and state in order to perform logic.
pub trait UpdatingFixed: 'static {
    fn update(&mut self, owner: EntityID, world: &mut World, delta_time: f32);
}

/// Indexes a component in a Multi component
pub type SlotID = u32;

/// Component used to store multiple components of the same type to the same Entity.
/// Automatically implements automated trait behaviors and propagates behavior to its contents.
///
/// THIS IS REQUIRED FOR COLLIDER STORAGE, EVEN IF ONLY ONE COLLIDER IS OWNED BY THE ENTITY
pub type Multi<T> = HashMap<SlotID, T>;

impl <T: Tickable> Tickable for Multi<T> {
    fn tick(&mut self, delta_time: f32) {
        for item in self.values_mut() {
            item.tick(delta_time);
        }
    }
}

impl <T: Tickable> TickableFixed for Multi<T> {
    fn tick(&mut self, delta_time: f32) {
        for item in self.values_mut() {
            item.tick(delta_time);
        }
    }
}

impl <T: Updating> Updating for Multi<T> {
    fn update(&mut self, owner: EntityID, input: &Input, world: &mut World, delta_time: f32) {
        for item in self.values_mut() {
            item.update(owner, input, world, delta_time);
        }
    }
}

impl <T: UpdatingFixed> UpdatingFixed for Multi<T> {
    fn update(&mut self, owner: EntityID, world: &mut World, delta_time: f32) {
        for item in self.values_mut() {
            item.update(owner, world, delta_time);
        }
    }
}
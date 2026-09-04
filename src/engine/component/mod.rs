use std::collections::HashMap;

pub mod transform;
pub mod collider;
pub mod animation;
pub mod simple_sprite;
pub mod text_block;
pub mod event_channel;
pub mod update_gates;
pub mod timer;
pub mod links;

/// Automated Behavior Trait
///
/// A component implementing this trait will automatically be ticked on each frame.
pub trait Tickable: 'static {
    fn tick(&mut self, delta_time: f32);
}

/// Indexes a component in a Multi component
pub type SlotID = u32;

/// Component used to store multiple components of the same type to the same Entity.
/// Automatically implements automated behavior traits and propagates behavior to its contents.
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
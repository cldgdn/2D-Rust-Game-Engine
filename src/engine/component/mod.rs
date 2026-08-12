use std::collections::HashMap;

pub mod transform;
pub mod collider;
pub mod animation;
pub mod simple_sprite;
pub mod text_block;
pub mod event_channel;

/// Indexes an object in a Multi component
pub type SlotID = u32;

/// Component used to store multiple components of the same type to the same Entity.
pub type Multi<T> = HashMap<SlotID, T>;
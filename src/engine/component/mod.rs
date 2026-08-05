use std::collections::HashMap;

pub mod transform;
pub mod collider;
pub mod animation;
pub mod simple_sprite;
pub mod text_block;

pub type SlotID = u32;

pub type Multi<T> = HashMap<SlotID, T>;
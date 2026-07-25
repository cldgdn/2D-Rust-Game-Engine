use std::collections::HashSet;
use std::time::Duration;
use crate::engine::component::collider::Collider;
use crate::engine::component::{Multi, SlotID};
use crate::engine::EntityID;
use crate::engine::world::World;
use crate::platform_builder::stats::Health;

pub struct BasicAttack {
    pub collider: SlotID,
    pub damage: f32,
    duration_max: f32,
    duration: f32,
    damaged_entities: HashSet<EntityID>,
}

impl BasicAttack {
    pub fn new(collider: SlotID, damage: f32, cooldown: f32) -> Self {
        Self {
            collider, damage,
            duration_max: cooldown,
            duration: 0.0,
            damaged_entities: HashSet::new(),
        }
    }

    pub fn start(&mut self) {
        self.damaged_entities.clear();
        self.duration = 0.0;
    }

    pub fn is_active(&self) -> bool {
        self.duration <= self.duration_max
    }

    pub fn progress(&mut self, delta_time: f32) -> bool {
        self.duration += delta_time;
        self.is_active()
    }

    pub fn was_damaged(&mut self, target: EntityID) -> bool {
        !self.damaged_entities.insert(target)
    }
}
use std::any::{Any, TypeId};
use std::collections::HashMap;
use crate::engine::EntityID;

pub trait Event: Any + Send + Sync {}

impl<T: Any + Send + Sync> Event for T {}

pub struct EventBus {
    subscribers: HashMap<TypeId, Vec<EntityID>>,
    queue: Vec<Box<dyn Any>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
            queue: Vec::new(),
        }
    }
    
    pub fn subscribe<E: Event>(&mut self, entity: EntityID) {
        self.subscribers
            .entry(TypeId::of::<E>())
            .or_default()
            .push(entity);
    }
    
    pub fn unsubscribe_entity(&mut self, entity: EntityID) {
        for entry in self.subscribers.values_mut() {
            entry.retain(|id| *id != entity);
        }
    }
    
    pub fn emit<E: Event>(&mut self, event: E) {
        self.queue.push(Box::new(event));
    }
    
    pub fn drain(&mut self) -> Vec<Box<dyn Any>> {
        std::mem::take(&mut self.queue)
    }
    
    pub fn listeners_for(&self, type_id: TypeId) -> Option<&Vec<EntityID>> {
        self.subscribers.get(&type_id)
    }
}
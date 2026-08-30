use std::collections::HashMap;
use serde_json::Value;
use crate::engine::{Entity, EntityID};
use crate::engine::world::World;

pub mod scene;

pub type EntityFactory = fn(id: EntityID, init: Value, world: &mut World) -> Box<dyn Entity>;

pub struct EntityRegistry {
    factories: HashMap<String, EntityFactory>,
}

impl EntityRegistry {
    pub fn new() -> Self {
        EntityRegistry {
            factories: HashMap::new(),
        }
    }
    
    pub fn register_factory(&mut self, name: &str, factory: EntityFactory) {
        self.factories.insert(name.to_string(), factory);
    }
    
    pub fn create(&self, kind: &str, id: EntityID, init: Value, world: &mut World) -> Box<dyn Entity> {
        let factory = self
            .factories
            .get(kind)
            .unwrap_or_else(|| panic!("Unknown entity kind: {}", kind));
        factory(id, init, world)
    }
}
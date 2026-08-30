use std::collections::HashMap;
use crate::engine::EntityID;

#[derive(Default)]
pub struct Links {
    pub map: HashMap<String, EntityID>
}

impl Links {
    pub fn new() -> Links {
        Self { map: HashMap::new() }
    }
    
    pub fn get(&self, key: &str) -> Option<EntityID> {
        self.map.get(key).copied()
    }
}
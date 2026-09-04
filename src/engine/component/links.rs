use std::collections::HashMap;
use crate::engine::EntityID;

/// A component used to access a different Entity's own EntityID, by associating it with 
/// a local name, resolved at scene load time.
#[derive(Default)]
pub struct Links {
    pub map: HashMap<String, EntityID>
}

impl Links {
    pub fn new() -> Links {
        Self { map: HashMap::new() }
    }
    
    /// Returns the EntityID associated with the given name, if there is one.
    pub fn get(&self, key: &str) -> Option<EntityID> {
        self.map.get(key).copied()
    }
}
use std::any::Any;
use crate::engine::EntityID;
use crate::engine::event::Event;

/// Entity-owned event bus, used by Entities to receive unicast-style Events: instead of subscribing
/// to an event globally using World::event_buffer, then forcing events to have target EntityIDs when
/// destined to a specific Entity and forcing all subscribers to sift through, this allows an Entity
/// to emit any Event directly to one specific entity knowing only its id, by pulling its 
/// EventChannel component and emitting to it.
pub struct EventChannel {
    pub entity: EntityID,
    queue: Vec<Box<dyn Any>>,
}

impl EventChannel {
    pub fn new(entity: EntityID) -> Self {
        Self {
            entity,
            queue: Vec::new(),
        }
    }
    /// Emits an event to the EventChannel
    pub fn emit<E: Event>(&mut self, event: E) {
        self.queue.push(Box::new(event));
    }
    /// Returns the list of all events received since the last time this was called
    pub fn drain(&mut self) -> Vec<Box<dyn Any>> {
        std::mem::take(&mut self.queue)
    }
}
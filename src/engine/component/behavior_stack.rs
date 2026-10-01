use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::engine::{Entity, EntityID};
use crate::engine::world::World;

/// What a Behavior tells its owning BehaviorStack to do after an update call
pub enum BehaviorSignal {
    /// No change. This behavior stays on top and keeps running next call.
    Continue,

    /// Pop this behavior off the stack entirely. Fires on_exit on it,
    /// then on_enter on whatever is revealed underneath, if anything.
    Pop,

    /// Push a new Behavior on top of this one. This behavior is NOT popped, it fires on_exit
    /// and remains in the stack underneath next, ready to be revealed again once next eventually
    /// pops or replaces itself away.
    Push(Box<dyn Behavior>),

    /// Pop this behavior and immediately push the passed behavior in its place. Unlike Push, this
    /// behavior is discarded entirely rather than kept underneath, to be used for state transitions
    /// where the passing state won't be resumed after.
    Replace(Box<dyn Behavior>),
}

/// Instance-specific update logic that can be swapped in and out of an entity's BehaviorStack at
/// runtime.
pub trait Behavior {
    /// Fires when this behavior becomes the top of the stack - NOT for initialization, but
    /// preparing the behavior before the first update call after becoming top
    fn on_enter(&mut self, world: &mut World, owner: EntityID) {}

    /// Fires when this behavior stops being the top of the stack (popped off or covered by a push)
    /// - NOT for drop logic, but setting the behavior to stop on a resumable state
    fn on_exit(&mut self, world: &mut World, owner: EntityID) {}

    /// Called while this behavior is on top of the stack and the owning entity has chosen to
    /// call BehaviorStack::update this tick.
    fn update(&mut self, world: &mut World, owner: EntityID, delta_time: f32) -> BehaviorSignal;
}

pub struct BehaviorStack {
    stack: Vec<Box<dyn Behavior>>,
}

impl BehaviorStack {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Push a behavior into the stack
    pub fn push(&mut self, world: &mut World, owner: EntityID, mut behavior: Box<dyn Behavior>) {
        if let Some(top) = self.stack.last_mut() {
            top.on_exit(world, owner);
        }

        behavior.on_enter(world, owner);
        self.stack.push(behavior);
    }

    /// Pop the current top behavior, if any.
    pub fn pop(&mut self, world: &mut World, owner: EntityID) -> Option<Box<dyn Behavior>> {
        if let Some(mut top) = self.stack.pop() {
            top.on_exit(world, owner);
            if let Some(revealed) = self.stack.last_mut() {
                revealed.on_enter(world, owner);
            }
            Some(top)
        } else {
            None
        }
    }

    /// Run the current top Behavior's update once and apply whatever BehaviorSignal it returns
    pub fn update(&mut self, world: &mut World, owner: EntityID, delta_time: f32) {
        if let Some(mut top) = self.stack.pop() {
            match top.update(world, owner, delta_time) {
                BehaviorSignal::Continue => {
                    self.stack.push(top);
                },
                BehaviorSignal::Pop => {
                    top.on_exit(world, owner);
                    if let Some(revealed) = self.stack.last_mut() {
                        revealed.on_enter(world, owner);
                    }
                },
                BehaviorSignal::Push(mut next) => {
                    top.on_exit(world, owner);
                    self.stack.push(top);
                    next.on_enter(world, owner);
                    self.stack.push(next);
                },
                BehaviorSignal::Replace(mut next) => {
                    top.on_exit(world, owner);
                    next.on_enter(world, owner);
                    self.stack.push(next);
                }
            }
        }
    }

    /// Push a behavior into the stack WITHOUT firing any hooks. Intended for assembling the
    /// initial stack bottom-up during scene construction.
    pub fn push_uninitialized(&mut self, behavior: Box<dyn Behavior>) {
        self.stack.push(behavior);
    }
    /// Fires on_enter on the current top of the stack. Intended for initializing the stack at
    /// creation time.
    pub fn activate_initial(&mut self, world: &mut World, owner: EntityID) {
        if let Some(top) = self.stack.last_mut() {
            top.on_enter(world, owner);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorDescriptor {
    pub kind: String,
    #[serde(default)]
    pub init: Option<Value>,
}

pub const DEFAULT_KIND_MARKER: &str = "DEFAULT";

pub type BehaviorFactory = fn(owner: EntityID, init: Value, world: &mut World) -> Box<dyn Behavior>;
pub type DefaultBehaviorsFactory = fn(owner: EntityID, world: &mut World) -> Vec<BehaviorDescriptor>;

pub struct BehaviorRegistry {
    factories: HashMap<String, BehaviorFactory>,
    /// Keyed by entity kind, not behavior kind, used to build the default stacks for entitites that
    /// want them.
    defaults: HashMap<String, DefaultBehaviorsFactory>,
}

impl BehaviorRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            defaults: HashMap::new(),
        }
    }

    /// Registers a factory function that builds a behavior from a BehaviorDescriptor
    pub fn register_factory(&mut self, behavior_kind: &str, factory: BehaviorFactory) {
        self.factories.insert(behavior_kind.to_string(), factory);
    }
    
    /// Registers a factory function that builds a list of default BehaviorDescriptors for a 
    /// specific Entity kind.
    pub fn register_default(&mut self, entity_kind: &str, default: DefaultBehaviorsFactory) {
        self.defaults.insert(entity_kind.to_string(), default);
    }

    fn build_one(&self, world: &mut World, owner: EntityID, d: &BehaviorDescriptor) -> Box<dyn Behavior> {
        let factory = self
            .factories
            .get(&d.kind)
            .unwrap_or_else(|| panic!("No BehaviorFactory registered for kind {}", d.kind));
        factory(owner, d.init.clone().unwrap_or(Value::Null), world)
    }
    
    /// Builds a BehaviorStack from an ordered descriptor list, attaches it to owner and 
    /// activates the top entry.
    /// 
    /// entity_kind is used to resolve any default entries in descriptors against that entity
    /// kind's registered default list.
    /// 
    /// A default list cannot be recursive (cannot include further default descriptors), will panic.
    pub fn build_stack(
        &self, 
        world: &mut World, 
        owner: EntityID, 
        entity_kind: &str, 
        descriptors: &Vec<BehaviorDescriptor>)
    {
        let mut stack = BehaviorStack::new();

        for d in descriptors {
            if d.kind == DEFAULT_KIND_MARKER {
                let defaults_factory = self.defaults.get(entity_kind).unwrap_or_else(|| {
                    panic!("No default behaviors registered for Entity kind {}", entity_kind);
                });
                let default_descriptors = defaults_factory(owner, world);
                
                for dd in default_descriptors {
                    assert_ne!(
                        dd.kind, DEFAULT_KIND_MARKER, 
                        "Default behavior list for Entity kind {} contains a nested {} entry", 
                        entity_kind, DEFAULT_KIND_MARKER
                    );
                    let behavior = self.build_one(world, owner, &dd);
                    stack.push_uninitialized(behavior);
                }
            } else {
                let behavior = self.build_one(world, owner, &d);
                stack.push_uninitialized(behavior);
            }
        }

        stack.activate_initial(world, owner);
        world.add_component(owner, stack);
    }
}
use glfw::WindowEvent::Key;
use crate::engine::component::event_channel::EventChannel;
use crate::engine::entity::scene::Scene;
use crate::engine::{Entity, EntityID, Updatable};
use crate::engine::event::BeginSceneLoadEvent;
use crate::engine::input::Input;
use crate::engine::world::{Intention, World};
use crate::engine::world::Intention::Destroy;

pub struct SceneManager {
    pub id: EntityID,
    pub scene_id: EntityID,
}

impl SceneManager {
    pub fn new(id: EntityID, world: &mut World) -> Self{
        let scene_id = world.get_next_entity_id();
        let scene = Scene::new(scene_id, "resources\\scenes\\json\\test.json".parse().unwrap(), world);

        world.push_intention(Intention::Spawn(Box::new(scene)));
        let channel = world.get_component_mut::<EventChannel>(scene_id).unwrap();
        channel.emit(BeginSceneLoadEvent);

        Self {
            id,
            scene_id,
        }
    }
}

impl Entity for SceneManager {
    fn id(&self) -> EntityID {
        self.id
    }

    fn as_updatable_mut(&mut self) -> Option<&mut dyn Updatable> {
        Some(self)
    }
}

impl Updatable for SceneManager {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        let action = input.is_action_pressed("kill_scene").active;
        if action {
            world.push_intention(Destroy(self.scene_id));
        }
    }
}
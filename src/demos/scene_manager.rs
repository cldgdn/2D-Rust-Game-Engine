use glfw::WindowEvent::Key;
use crate::engine::component::event_channel::EventChannel;
use crate::engine::entity::scene::Scene;
use crate::engine::{Entity, EntityID, Updatable};
use crate::engine::event::BeginSceneLoadEvent;
use crate::engine::input::Input;
use crate::engine::world::{Intention, World};
use crate::engine::world::Intention::{Destroy, SetInputProfile};

pub struct SceneManager {
    pub id: EntityID,
    pub scene_top_down: EntityID,
    pub scene_platform: EntityID,

    pub on_platform: bool,
}

impl SceneManager {
    pub fn new(id: EntityID, world: &mut World) -> Self{
        let scene = world.get_next_entity_id();
        let s = Scene::new(scene, "resources/scenes/json/platform.json".parse().unwrap(), world);
        world.push_intention(Intention::Spawn(Box::new(s)));
        let channel = world.get_component_mut::<EventChannel>(scene).unwrap();
        channel.emit(BeginSceneLoadEvent);

        world.push_intention(SetInputProfile("platform".to_string()));

        Self {
            id,
            scene_top_down: 0,
            scene_platform: scene,
            on_platform: true,
        }
    }
}

impl Entity for SceneManager {
    fn id(&self) -> EntityID {
        self.id
    }

    fn as_updatable(&self) -> Option<&dyn Updatable> {
        Some(self)
    }

    fn as_updatable_mut(&mut self) -> Option<&mut dyn Updatable> {
        Some(self)
    }
}

impl Updatable for SceneManager {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        let action = input.is_action_pressed("toggle_scene").active;
        if action {
            if self.on_platform {
                world.push_intention(Destroy(self.scene_platform));

                let scene_top_down = world.get_next_entity_id();
                let scene = Scene::new(scene_top_down, "resources/scenes/json/top_down.json".parse().unwrap(), world);
                world.push_intention(Intention::Spawn(Box::new(scene)));
                self.scene_top_down = scene_top_down;

                let top_down_channel = world.get_component_mut::<EventChannel>(self.scene_top_down).unwrap();
                top_down_channel.emit(BeginSceneLoadEvent);

                world.push_intention(SetInputProfile("default".to_string()));
            }
            else {
                world.push_intention(Destroy(self.scene_top_down));

                let scene_platform = world.get_next_entity_id();
                let scene = Scene::new(scene_platform, "resources/scenes/json/platform.json".parse().unwrap(), world);
                world.push_intention(Intention::Spawn(Box::new(scene)));
                self.scene_platform = scene_platform;

                let platform_channel  = world.get_component_mut::<EventChannel>(self.scene_platform).unwrap();
                platform_channel.emit(BeginSceneLoadEvent);

                world.push_intention(SetInputProfile("platform".to_string()));
            }

            self.on_platform = !self.on_platform;
        }
    }
}
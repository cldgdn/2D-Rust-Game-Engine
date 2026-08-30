use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::Receiver;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::engine::{Entity, EntityID, Updatable};
use crate::engine::component::event_channel::EventChannel;
use crate::engine::event::BeginSceneLoadEvent;
use crate::engine::input::Input;
use crate::engine::world::{Intention, World};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntityDescriptor {
    pub slot: String,
    pub kind: String,
    pub init: Value,
    #[serde(default)]
    pub links: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneCameraDescriptor {
    pub name: String,
    pub target_slot: Option<String>,
    pub starting_position: Option<(f32, f32)>,
    pub boundary: Option<(f32, f32, f32, f32)>,
    pub deadzone: Option<(f32, f32)>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SceneDescriptor {
    pub cameras: Vec<SceneCameraDescriptor>,
    pub entities: Vec<SceneEntityDescriptor>,
}

pub struct Scene {
    id: EntityID,
    scene_path: PathBuf,
    pending_load: Option<Receiver<LoadedScenePayload>>,
}

impl Scene {
    pub fn new(id: EntityID, scene_path: PathBuf, world: &mut World) -> Self {
        let event_channel = EventChannel::new(id);
        world.add_component(id, event_channel);
        
        Self {
            id,
            scene_path,
            pending_load: None,
        }
    }

    pub fn begin_load(&mut self) {
        let (tx, rx) = mpsc::channel();
        let path = self.scene_path.clone();
        std::thread::spawn(move || {
            let payload = load_scene_job(path);
            let _ = tx.send(payload);
        });
        self.pending_load = Some(rx);
    }
}

impl Entity for Scene {
    fn id(&self) -> EntityID {
        self.id
    }

    fn on_event(&mut self, event: &dyn Any, world: &mut World) {
        ()
    }

    fn as_updatable_mut(&mut self) -> Option<&mut dyn Updatable> {
        Some(self)
    }
}

impl Updatable for Scene {
    fn update(&mut self, _input: &Input, world: &mut World, _delta_time: f32) {
        let channel= world.get_component_mut::<EventChannel>(self.id).unwrap();
        
        for event in channel.drain() {
            if event.is::<BeginSceneLoadEvent>() {
                self.begin_load();
            }
        }
        
        if let Some(rx) = &self.pending_load {
            match rx.try_recv() {
                Ok(payload) => {
                    world.push_intention(Intention::CommitScene(payload, Some(self.id)));
                    self.pending_load = None;
                },
                Err(TryRecvError::Empty) => {} //Still loading, do nothing,
                Err(TryRecvError::Disconnected) => {
                    eprintln!(
                        "Scene {}: thread loading scene file at {} died without sending",
                        self.id, self.scene_path.display()
                    );
                    self.pending_load = None;
                }
            }
        }
    }
}

pub struct LoadedScenePayload {
    pub descriptor: SceneDescriptor,
}

fn load_scene_job(path: PathBuf) -> LoadedScenePayload {
    let bytes = std::fs::read(path).expect("failed to read scene file");
    let descriptor: SceneDescriptor = serde_json::from_slice(&bytes)
        .expect("invalid scene descriptor");
    
    LoadedScenePayload { descriptor }
}
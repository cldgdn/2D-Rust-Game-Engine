use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use glam::Vec2;
use crate::engine::camera::{Camera, CameraID};
use crate::engine::collision::CollisionSystem;
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::{Entity, EntityID};
use crate::engine::collision::raycast::RaycastRequest;
use crate::engine::event::EventBus;
use crate::engine::rendering::shader::{Shader, ShaderBuilder, ShaderID};
use crate::engine::rendering::text::TextManager;
use crate::engine::rendering::texture::TextureManager;

pub enum Intention {
    Spawn(Box<dyn Entity>),
    Destroy(EntityID),
    RenderPass(RenderPassIntention),
    PostFx(PostFxIntention),
}

pub enum RenderPassIntention {
    SetEnabled(String, bool),
    ToggleEnabled(String),
    PostFx(String, PostFxIntention)
}

pub enum PostFxIntention {
    SetEnabled(String, bool),
    ToggleEnabled(String),
}

pub enum FixedIntention {
    RaycastRequest(RaycastRequest)
}

trait ComponentStorage: Any {
    fn get_any(&self, id: EntityID) -> Option<&dyn Any>;
    fn get_any_mut(&mut self, id: EntityID) -> Option<&mut dyn Any>;
    fn remove_any(&mut self, id: EntityID);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> ComponentStorage for HashMap<EntityID, T> {
    fn get_any(&self, id: EntityID) -> Option<&dyn Any> {
        self.get(&id).map(|v| v as &dyn Any)
    }

    fn get_any_mut(&mut self, id: EntityID) -> Option<&mut dyn Any> {
        self.get_mut(&id).map(|v| v as &mut dyn Any)
    }

    fn remove_any(&mut self, id: EntityID) {
        self.remove(&id);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct World {
    gl: Rc<glow::Context>,

    next_entity_id: EntityID,

    components: HashMap<TypeId, Box<dyn ComponentStorage>>,

    texture_manager: TextureManager,
    text_manager: TextManager,

    debug_shader: Shader,
    shaders: HashMap<ShaderID, Shader>,
    shader_names: HashMap<String, ShaderID>,
    next_shader_id: ShaderID,

    cameras: HashMap<CameraID, Camera>,
    next_camera_id: CameraID,
    
    pub event_bus: EventBus,
    pub intention_queue: Vec<Intention>,
    pub fixed_intention_queue: Vec<FixedIntention>,
}

impl World {
    pub fn new(gl: Rc<glow::Context>) -> World {
        let mut cameras = HashMap::new();
        let (x, y) = config().logic_size;
        let mut def_cam = Camera::new(0, (x, y));
        def_cam.position = Vec2::new(
            x as f32 / config().unit_pixel_size as f32, 
            y as f32 / config().unit_pixel_size as f32
        );

        cameras.insert(0, def_cam);
        
        let debug_shader = Shader::new(
            gl.clone(),
            "shaders/debug.vert",
            "shaders/debug.frag",
        ).expect("Failed to create debug shader");
        
        World {
            gl,
            next_entity_id: 0,
            components: HashMap::new(),
            texture_manager: TextureManager::new(),
            text_manager: TextManager::new(),
            debug_shader,
            shaders: HashMap::new(),
            shader_names: HashMap::new(),
            next_shader_id: 1,
            cameras,
            next_camera_id: 1,
            event_bus: EventBus::new(),
            intention_queue: Vec::new(),
            fixed_intention_queue: Vec::new(),
        }
    }
    
    pub fn register_component_type<T: 'static>(&mut self) {
        self.components
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(HashMap::<EntityID, T>::new()));
    }
    
    pub fn add_component<T: 'static>(&mut self, id: EntityID, value: T) {
        self.register_component_type::<T>();
        let storage = self.components.get_mut(&TypeId::of::<T>()).unwrap();
        
        storage.as_any_mut()
            .downcast_mut::<HashMap<EntityID, T>>()
            .unwrap()
            .insert(id, value);
    }
    
    pub fn get_component<T: 'static>(&self, id: EntityID) -> Option<&T> {
        self.components.get(&TypeId::of::<T>())?
            .as_any()
            .downcast_ref::<HashMap<EntityID, T>>()
            .unwrap()
            .get(&id)
    }
    
    pub fn get_component_mut<T: 'static>(&mut self, id: EntityID) -> Option<&mut T> {
        self.components.get_mut(&TypeId::of::<T>())?
            .as_any_mut()
            .downcast_mut::<HashMap<EntityID, T>>()
            .unwrap()
            .get_mut(&id)
    }
    
    pub fn get_many_components(&self, requests: &[(EntityID, TypeId)]) -> Vec<Option<&dyn Any>> {
        requests.iter()
            .map(|(id, type_id)| {
                self.components.get(type_id)?.get_any(*id)
            })
            .collect()
    }
    
    pub fn get_many_components_mut(&mut self, requests: &[(EntityID, TypeId)]) -> Vec<Option<&mut dyn Any>> {
        let mut seen = HashSet::with_capacity(requests.len());
        for r in requests {
            assert!(seen.insert(*r), "Attempting to request same component multiple times in the same request.");
        }
        
        let components_ptr: *mut HashMap<TypeId, Box<dyn ComponentStorage>> = &mut self.components;
        
        requests.iter()
            .map(|(id, type_id)| unsafe {
                (*components_ptr)
                    .get_mut(type_id)
                    .and_then(|storage| storage.get_any_mut(*id))
            })
            .collect()
    }

    pub fn get_all_components<T: 'static>(&self) -> impl Iterator<Item = (EntityID, &T)> {
        self.components
            .get(&TypeId::of::<T>())
            .into_iter()
            .flat_map(|storage| {
                storage
                    .as_any()
                    .downcast_ref::<HashMap<EntityID, T>>()
                    .unwrap()
                    .iter()
                    .map(|(id, component)| (*id, component))
            })
    }

    pub fn get_all_component_mut<T: 'static>(&mut self) -> impl Iterator<Item = (EntityID, &mut T)> {
        self.components
            .get_mut(&TypeId::of::<T>())
            .into_iter()
            .flat_map(|storage| {
                storage
                    .as_any_mut()
                    .downcast_mut::<HashMap<EntityID, T>>()
                    .unwrap()
                    .iter_mut()
                    .map(|(id, component)| (*id, component))
            })
    }

    pub fn remove_components(&mut self, id: EntityID) {
        for storage in self.components.values_mut() {
            storage.remove_any(id);
        }
    }
    
    pub fn get_next_entity_id(&mut self) -> EntityID {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    pub fn get_next_camera_id(&mut self) -> CameraID {
        let id = self.next_camera_id;
        self.next_camera_id += 1;
        id
    }

    pub fn add_shader(&mut self, gl: Rc<glow::Context>, vertex_source: &str, fragment_source: &str, name: &str) -> Result<ShaderID, String> {
        let compile_res = Shader::new(gl, vertex_source, fragment_source);

        match compile_res {
            Ok(shader) => {
                let id = self.next_shader_id;
                self.shaders.insert(id, shader);
                self.shader_names.insert(name.to_string(), self.next_shader_id);
                self.next_shader_id += 1;
                Ok(id)
            }
            Err(e) => Err(e)
        }
    }

    pub fn register_shader(&mut self, shader: Shader, name: &str) -> ShaderID {
        let id = self.next_shader_id;
        self.shaders.insert(id, shader);
        self.shader_names.insert(name.to_string(), self.next_shader_id);
        self.next_shader_id += 1;

        id
    }

    pub fn shader_id(&self, name: &str) -> Option<ShaderID> {
        self.shader_names.get(name).cloned()
    }
    
    pub fn shaders(&self) -> &HashMap<ShaderID, Shader> {
        &self.shaders
    }
    
    pub fn shader(&self, name: &str) -> Option<&Shader> {
        self.shaders.get(self.shader_names.get(name).unwrap())
    }
    
    pub fn shader_by_id(&self, id: ShaderID) -> Option<&Shader> {
        self.shaders.get(&id)
    }

    pub fn shader_name(&self, id: ShaderID) -> Option<&str> {
        for (name,sid) in self.shader_names.iter() {
            if id == *sid {
                return Some(name);
            }
        }

        None
    }

    pub fn texture_manager_mut(&mut self) -> &mut TextureManager {
        &mut self.texture_manager
    }

    pub fn texture_manager(&self) -> &TextureManager {&self.texture_manager}

    pub fn text_manager_mut(&mut self) -> &mut TextManager {&mut self.text_manager}
    pub fn text_manager(&self) -> &TextManager {&self.text_manager}

    pub fn gl(&self) -> Rc<glow::Context> {
        self.gl.clone()
    }

    pub fn add_camera(&mut self, camera: Camera) {
        self.cameras.insert(camera.id, camera);
    }

    pub fn camera(&self, id: CameraID) -> Option<&Camera> {
        self.cameras.get(&id)
    }

    pub fn camera_mut(&mut self, id: CameraID) -> Option<&mut Camera> {
        self.cameras.get_mut(&id)
    }

    pub fn update_cameras(&mut self) {
        for camera in self.cameras.values_mut() {
            let Some(target) = camera.target else {
                continue;
            };

            let t_pos = self.components
                .get(&TypeId::of::<Transform>())
                .unwrap()
                .as_any()
                .downcast_ref::<HashMap<EntityID, Transform>>()
                .unwrap()
                .get(&target)
                .unwrap()
                .position();

            camera.follow(t_pos);
            camera.clamp_to_boundary();
        }
    }
    
    pub fn debug_shader(&self) -> &Shader {
        &self.debug_shader
    }
    
    pub fn dispatch_events(&mut self, entities: &mut HashMap<EntityID, Box<dyn Entity>>) {
        let events = self.event_bus.drain();
        
        for event in events {
            let type_id = (*event).type_id();
            
            let listeners = match self.event_bus.listeners_for(type_id) {
                Some(v) => v.clone(),
                None => continue,
            };
            
            for entity in listeners {
                if let Some(entity) = entities.get_mut(&entity) {
                    entity.on_event(event.as_ref(), self);
                }
            }
        }
    }
    
    pub fn add_intention(&mut self, intention: Intention) {
        self.intention_queue.push(intention);
    }
}
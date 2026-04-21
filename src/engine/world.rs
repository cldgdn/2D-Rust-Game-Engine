use std::collections::HashMap;
use std::rc::Rc;
use glam::Vec2;
use crate::engine::camera::Camera;
use crate::engine::collision::CollisionSystem;
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::EntityID;
use crate::engine::shader::{Shader, ShaderID};
use crate::engine::texture::TextureManager;

pub struct World {
    gl: Rc<glow::Context>,

    next_entity_id: EntityID,

    transforms: HashMap<EntityID, Transform>,

    texture_manager: TextureManager,

    shaders: HashMap<ShaderID, Shader>,
    shader_names: HashMap<String, ShaderID>,
    next_shader_id: ShaderID,

    cameras: HashMap<String, Camera>,
    current_camera: String,
}

impl World {
    pub fn new(gl: Rc<glow::Context>) -> World {
        let mut cameras = HashMap::new();
        let (x, y) = config().logic_size;
        let def_cam = Camera::new(
            Vec2::new(x as f32, y as f32),
            1.0,
            0.0,
        );

        cameras.insert("default".to_string(), def_cam);

        World {
            gl,
            next_entity_id: 0,
            transforms: HashMap::new(),
            texture_manager: TextureManager::new(),
            shaders: HashMap::new(),
            shader_names: HashMap::new(),
            next_shader_id: 0,
            cameras,
            current_camera: "default".to_string(),
        }
    }
    
    pub fn add_transform(&mut self, entity: EntityID, transform: Transform) {
        self.transforms.insert(entity, transform);
    }
    
    pub fn get_transform(&self, id: EntityID) -> Option<&Transform> {
        self.transforms.get(&id)
    }
    
    pub fn get_mut_transform(&mut self, id: EntityID) -> Option<&mut Transform> {
        self.transforms.get_mut(&id)
    }
    
    pub fn get_next_entity_id(&mut self) -> EntityID {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
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

    pub fn texture_manager(&mut self) -> &mut TextureManager {
        &mut self.texture_manager
    }

    pub fn gl(&self) -> Rc<glow::Context> {
        self.gl.clone()
    }

    pub fn add_camera(&mut self, name: &str, camera: Camera) {
        self.cameras.insert(name.to_string(), camera);
    }

    pub fn set_camera(&mut self, name: &str) -> Result<(), ()> {
        if let Some(_) = self.cameras.get_mut(name) {
            self.current_camera = name.to_string();
            return Ok(());
        }

        Err(())
    }

    pub fn camera(&self) -> &Camera {
        self.cameras.get(&self.current_camera).unwrap()
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        self.cameras.get_mut(&self.current_camera).unwrap()
    }
}
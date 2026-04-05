use std::collections::HashMap;
use std::rc::Rc;
use crate::engine::component::transform::Transform;
use crate::engine::EntityID;
use crate::engine::shader::{Shader, ShaderID};

pub struct World {
    transforms: HashMap<EntityID, Transform>,
    next_transform_id: EntityID,

    shaders: HashMap<ShaderID, Shader>,
    shader_names: HashMap<String, ShaderID>,
    next_shader_id: ShaderID,
}

impl World {
    pub fn new() -> World {
        World {
            transforms: HashMap::new(),
            next_transform_id: 0,
            shaders: HashMap::new(),
            shader_names: HashMap::new(),
            next_shader_id: 0,
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
    
    pub fn get_next_transform_id(&mut self) -> EntityID {
        let id = self.next_transform_id;
        self.next_transform_id += 1;
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
}
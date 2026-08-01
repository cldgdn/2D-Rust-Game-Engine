use crate::engine::rendering::batched_renderer::UniformProvider;
use crate::engine::rendering::shader::{Shader, ShaderID};

pub struct PostProcessLayer {
    pub name: String,
    pub shader_id: ShaderID,
    pub enabled: bool,
    /// Determines execution order within whichever queue this layer lives in.
    /// Lower priority runs first.
    pub priority: i32,
    /// Optional hook for effect-specific uniforms (blur radius, thresholds, etc.)
    pub uniform_provider: Option<Box<dyn UniformProvider>>,
}

impl PostProcessLayer {
    pub fn new (name: &str, shader_id: ShaderID) -> Self {
        Self {
            name: name.to_owned(),
            shader_id,
            enabled: true,
            priority: 0,
            uniform_provider: None,
        }
    }
    
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn with_uniform_provider(mut self, uniform_provider: Box<dyn UniformProvider>) -> Self {
        self.uniform_provider = Some(uniform_provider);
        self
    }
    
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
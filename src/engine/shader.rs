use glow::HasContext;
use glam::{Mat4, Vec4};
use std::fs;
use std::path::Path;
use std::rc::Rc;

pub(crate) type ShaderID = u32;

pub struct Shader {
    gl: Rc<glow::Context>,
    program: glow::Program,
}

impl Shader {
    pub fn new(
        gl: Rc<glow::Context>,
        vertex_path: impl AsRef<Path>,
        fragment_path: impl AsRef<Path>,
    ) -> Result<Self, String> {
        let vertex_source = fs::read_to_string(&vertex_path)
            .map_err(|e| {
                format!("Failed to read vertex shader {:?}: {}", vertex_path.as_ref(), e)
            })?;

        let fragment_source = fs::read_to_string(&fragment_path)
            .map_err(|e| {
                format!("Failed to read fragment shader {:?}: {}", fragment_path.as_ref(), e)
            })?;

        let program = Self::create_program(&gl, &vertex_source, &fragment_source)?;

        Ok(Self {gl, program})
    }

    fn create_program(
        gl: &glow::Context,
        vertex_source: &str,
        fragment_source: &str
    ) -> Result<glow::Program, String> {
        unsafe {
            let program = gl
                .create_program()
                .map_err(|e| format!("Failed to create GL program: {}", e))?;

            let vertex_shader = gl
                .create_shader(glow::VERTEX_SHADER)
                .map_err(|e| format!("Failed to create vertex shader: {}", e))?;
            gl.shader_source(vertex_shader, vertex_source);
            gl.compile_shader(vertex_shader);

            if !gl.get_shader_compile_status(vertex_shader) {
                let log = gl.get_shader_info_log(vertex_shader);
                gl.delete_shader(vertex_shader);
                gl.delete_program(program);
                return Err(format!("Vertex shader compilation failed: \n{}", log));
            }

            let fragment_shader = gl
                .create_shader(glow::FRAGMENT_SHADER)
                .map_err(|e| format!("Failed to create fragment shader: {}", e))?;
            gl.shader_source(fragment_shader, fragment_source);
            gl.compile_shader(fragment_shader);

            if !gl.get_shader_compile_status(fragment_shader) {
                let log = gl.get_shader_info_log(fragment_shader);
                gl.delete_shader(vertex_shader);
                gl.delete_shader(fragment_shader);
                gl.delete_program(program);
                return Err(format!("Fragment shader compilation failed: \n{}", log));
            }

            gl.attach_shader(program, vertex_shader);
            gl.attach_shader(program, fragment_shader);
            gl.link_program(program);

            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.detach_shader(program, vertex_shader);
                gl.detach_shader(program, fragment_shader);
                gl.delete_shader(vertex_shader);
                gl.delete_shader(fragment_shader);
                gl.delete_program(program);
                return Err(format!("Program linking failed: \n{}", log));
            }

            gl.detach_shader(program, vertex_shader);
            gl.detach_shader(program, fragment_shader);
            gl.delete_shader(vertex_shader);
            gl.delete_shader(fragment_shader);

            //println!("{}", gl.get_program_info_log(program));

            Ok(program)
        }
    }

    pub fn use_program(&self) {
        unsafe {
            self.gl.use_program(Some(self.program));
        }
    }

    pub fn id(&self) -> glow::Program {
        self.program
    }

    fn get_uniform_location(&self, name: &str) -> Option<glow::UniformLocation> {
        unsafe {
            self.gl.get_uniform_location(self.program, name)
        }
    }

    pub fn set_bool(&self, name: &str, value: bool) {
        self.set_int(name, value as i32);
    }

    pub fn set_int(&self, name: &str, value: i32) {
        unsafe {
            if let Some(location) = self.get_uniform_location(name) {
                self.gl.uniform_1_i32(Some(&location), value);
            }
        }
    }

    pub fn set_float(&self, name: &str, value: f32) {
        unsafe {
            if let Some(location) = self.get_uniform_location(name) {
                self.gl.uniform_1_f32(Some(&location), value);
            }
        }
    }

    pub fn set_vec2(&self, name: &str, x: f32, y: f32) {
        unsafe {
            if let Some(location) = self.get_uniform_location(name) {
                self.gl.uniform_2_f32(Some(&location), x, y);
            }
        }
    }

    pub fn set_vec3(&self, name: &str, x: f32, y: f32, z: f32) {
        unsafe {
            if let Some(location) = self.get_uniform_location(name) {
                self.gl.uniform_3_f32(Some(&location), x, y, z);
            }
        }
    }

    pub fn set_vec4(&self, name: &str, vec: Vec4) {
        unsafe {
            if let Some(location) = self.get_uniform_location(name) {
                self.gl.uniform_4_f32(Some(&location), vec.x, vec.y, vec.z, vec.w);
            }
        }
    }

    pub fn set_mat4(&self, name: &str, matrix: &Mat4) {
        unsafe {
            if let Some(location) = self.get_uniform_location(name) {
                self.gl
                    .uniform_matrix_4_f32_slice(Some(&location), false, &matrix.to_cols_array());
            }
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.program);
        }
    }
}
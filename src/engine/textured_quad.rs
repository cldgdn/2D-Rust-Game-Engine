use crate::engine::texture::{Texture2D, TileMode};
use glow::{Context, HasContext};
use std::rc::Rc;
use glam::{Mat4, Vec2, Vec3, Vec4};
use crate::engine::shader::Shader;

pub struct TexturedQuad {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    width: u32,
    height: u32,
    texture: Rc<Texture2D>,
    gl: Rc<Context>,
}

impl TexturedQuad {
    pub fn new (width: u32, height: u32, gl: Rc<Context>, texture: Rc<Texture2D>) -> Result<Self, String> {

        let vertices: [f32; 16];

        {
            let mode = texture.tile_mode();
            let u_max: f32;
            let v_max: f32;
            let width = width as f32;
            let height = height as f32;

            match mode {
                TileMode::Tile => {
                    u_max = width / texture.width() as f32;
                    v_max = height / texture.height() as f32;
                }
                TileMode::Stretch => {
                    u_max = 1.0;
                    v_max = 1.0;
                }
            };

            vertices = [
                width, height, u_max, v_max,  //TR
                width, 0.0,    u_max, 0.0,    //BR
                0.0,   0.0,    0.0,   0.0,    //BL
                0.0,   height, 0.0,   v_max   //TL
            ];
        }

        let indices: [u16; 6] = [0, 1, 3, 1, 2, 3];

        unsafe {
            let vao = gl.create_vertex_array().map_err(|e| format!("Unable to create VAO: {}", e))?;
            let vbo = gl.create_buffer().map_err(|e| format!("Unable to create VBO: {}", e))?;
            let ebo = gl.create_buffer().map_err(|e| format!("Unable to create EBO: {}", e))?;

            gl.bind_vertex_array(Some(vao));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&indices),
                glow::STATIC_DRAW,
            );

            let stride = (4 * size_of::<f32>()) as i32;

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);

            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 2 * size_of::<f32>() as i32);

            gl.bind_vertex_array(None);

            Ok(Self {
                vao, vbo, ebo, width, height, texture, gl
            })
        }
    }
    
    pub fn texture(&self) -> &Texture2D { &self.texture }

    pub fn draw(&self, shader: &Shader) {
        unsafe {
            shader.set_int("uTexture", 0);
            self.texture.bind(0);
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0);
            self.gl.bind_vertex_array(None);
        }
    }
}

impl Drop for TexturedQuad {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.vbo);
            self.gl.delete_buffer(self.ebo);
            self.gl.delete_vertex_array(self.vao);
        }
    }
}
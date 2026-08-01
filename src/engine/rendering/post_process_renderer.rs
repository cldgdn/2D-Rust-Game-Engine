use std::rc::Rc;
use glow::HasContext;

pub struct PostProcessRenderer {
    gl: Rc<glow::Context>,
    quad_vao: glow::VertexArray,
    quad_vbo: glow::Buffer,
}

impl PostProcessRenderer {
    pub fn new(gl: Rc<glow::Context>) -> Self {
        #[rustfmt::skip]
        let vertices: [f32; 16] = [
            -1.0, -1.0,  0.0, 0.0, // bottom-left
             1.0, -1.0,  1.0, 0.0, // bottom-right
            -1.0,  1.0,  0.0, 1.0, // top-left
             1.0,  1.0,  1.0, 1.0, // top-right
        ];

        let (quad_vao, quad_vbo) = unsafe {
            let vao = gl
                .create_vertex_array()
                .expect("PostProcessRenderer::new - Could not create vertex array");
            let vbo = gl
                .create_buffer()
                .expect("PostProcessRenderer::new - Could not create vertex buffer");

            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );

            let stride = (4 * size_of::<f32>()) as i32;

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);

            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 2 * size_of::<f32>() as i32);

            gl.bind_vertex_array(None);

            (vao, vbo)
        };

        Self {
            gl,
            quad_vao,
            quad_vbo,
        }
    }

    /// Binds the quad's VAO and issues the draw call. The caller is responsible for
    /// having already bound the correct shader, source texture, and target framebuffer/viewport.
    pub fn draw_fullscreen_quad(&self) {
        unsafe {
            self.gl.bind_vertex_array(Some(self.quad_vao));
            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            self.gl.bind_vertex_array(None);
        }
    }
}

impl Drop for PostProcessRenderer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.quad_vbo);
            self.gl.delete_vertex_array(self.quad_vao);
        }
    }
}
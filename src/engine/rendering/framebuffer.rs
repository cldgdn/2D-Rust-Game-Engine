use std::rc::Rc;
use glow::HasContext;

/// Wraps an OpenGL frambuffer object together with its color texture
/// attachment and optional depth renderbuffer attachment.
pub struct Framebuffer {
    gl: Rc<glow::Context>,
    fbo: glow::Framebuffer,
    color_texture: glow::Texture,
    depth_renderbuffer: Option<glow::Renderbuffer>,
    width: u32,
    height: u32,
}

impl Framebuffer {
    pub fn new(gl: Rc<glow::Context>, width: u32, height: u32, with_depth: bool) -> Self {
        unsafe {
            let fbo = gl
                .create_framebuffer()
                .expect("FrameBuffer::new - Could not create framebuffer");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));

            let color_texture = gl
                .create_texture()
                .expect("FrameBuffer::new - Could not create color texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(color_texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );

            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            // Clamp rather than repeat or mirror: post-fx sampling outside the edge should
            // not wrap around to the other side of the frame.
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);

            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(color_texture),
                0,
            );

            let depth_renderbuffer = if with_depth {
                let rbo = gl
                    .create_renderbuffer()
                    .expect("FrameBuffer::new - Could not create depth renderbuffer.");
                gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rbo));
                gl.renderbuffer_storage(
                    glow::RENDERBUFFER,
                    glow::DEPTH_COMPONENT24,
                    width as i32,
                    height as i32,
                );
                gl.framebuffer_renderbuffer(
                    glow::FRAMEBUFFER,
                    glow::DEPTH_ATTACHMENT,
                    glow::RENDERBUFFER,
                    Some(rbo),
                );

                Some(rbo)
            } else {
                None
            };

            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                panic!("FrameBuffer::new - incomplete FBO, status: 0x{:x}", status);
            }

            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);

            Self {
                gl,
                fbo,
                color_texture,
                depth_renderbuffer,
                width,
                height
            }
        }
    }

    pub fn bind(&self) {
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        }
    }

    pub fn bind_default(gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }

    pub fn color_texture(&self) -> glow::Texture {
        self.color_texture
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Copies a rectangular region of this framebuffer's color buffer into `dst` at the same
    /// rectangle. Used to composite a pass' isolated local-post-fx result into the shared
    /// based framebuffer.
    ///
    /// Takes raw pixel coordinates rather than a rect type.
    pub fn blit_color_into(&self, dst: &Framebuffer, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            self.gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.fbo));
            self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(dst.fbo));
            self.gl.blit_framebuffer(
                x, y, x + width, y + height,
                x, y, x + width, y + height,
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST
            );
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }

        let with_depth = self.depth_renderbuffer.is_some();
        let gl = self.gl.clone();
        *self = Self::new(gl, width, height, with_depth);
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_texture(self.color_texture);
            if let Some(rbo) = self.depth_renderbuffer {
                self.gl.delete_renderbuffer(rbo);
            }
            self.gl.delete_framebuffer(self.fbo);
        }
    }
}
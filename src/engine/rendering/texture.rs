use glow::HasContext;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub enum TileMode {
    Tile,
    Stretch
}

pub struct Texture2D {
    id: glow::Texture,
    width: u32,
    height: u32,
    gl: Rc<glow::Context>,
}

impl Texture2D {
    pub fn id(&self) -> glow::Texture { self.id }
    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }

    pub fn bind(&self, unit: u32) {
        unsafe {
            self.gl.active_texture(glow::TEXTURE0 + unit);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
        }
    }
}

impl Drop for Texture2D {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_texture(self.id);
        }
    }
}

pub struct TextureManager {
    textures: HashMap<PathBuf, Rc<Texture2D>>,
}

impl TextureManager {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub fn load(
        &mut self,
        gl: Rc<glow::Context>,
        path: impl AsRef<Path>,
    ) -> Result<Rc<Texture2D>, String> {
        let path = path.as_ref().to_path_buf();

        if let Some(existing) = self.textures.get(&path) {
            return Ok(existing.clone());
        }

        let img = image::open(&path)
            .map_err(|e| format!("Failed to load image {:?}: {}", path, e))?;

        let img = img.flipv().to_rgba8();
        let (width, height) = img.dimensions();
        let pixels = img.into_raw();

        let texture = unsafe {
            let texture = gl
                .create_texture()
                .map_err(|e| format!("Failed to create texture for {:?}: {}", path, e))?;

            let mode = glow::REPEAT as i32;
            
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));

            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                mode,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                mode,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&pixels)),
            );

            //gl.generate_mipmap(glow::TEXTURE_2D);
            gl.bind_texture(glow::TEXTURE_2D, None);

            texture
        };

        let texture = Rc::new(Texture2D {
            id: texture,
            width, height, gl
        });
        
        self.textures.insert(path, texture.clone());
        Ok(texture)
    }
    
    pub fn get(&self, path: impl AsRef<Path>) -> Option<Rc<Texture2D>> {
        self.textures.get(path.as_ref()).cloned()
    }
}
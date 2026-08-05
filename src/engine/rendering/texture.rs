use glow::HasContext;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TextureFormat {
    Rgba8,
    R8
}

impl TextureFormat {
    fn gl_internal_format(&self) -> i32 {
        match self {
            TextureFormat::Rgba8 => glow::RGBA8 as i32,
            TextureFormat::R8 => glow::R8 as i32,
        }
    }

    fn gl_format(&self) -> u32 {
        match self {
            TextureFormat::Rgba8 => glow::RGBA,
            TextureFormat::R8 => glow::RED,
        }
    }

    fn gl_type(&self) -> u32 {
        glow::UNSIGNED_BYTE
    }

    pub fn bytes_per_pixel(self) -> usize {
        match self {
            TextureFormat::Rgba8 => 4,
            TextureFormat::R8 => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterMode {
    Nearest,
    Linear,
}

impl FilterMode {
    fn to_gl(self) -> i32 {
        match self {
            FilterMode::Nearest => glow::NEAREST as i32,
            FilterMode::Linear => glow::LINEAR as i32,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WrapMode {
    Repeat,
    ClampToEdge,
}

impl WrapMode {
    fn to_gl(self) -> i32 {
        match self {
            WrapMode::Repeat => glow::REPEAT as i32,
            WrapMode::ClampToEdge => glow::CLAMP_TO_EDGE as i32,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextureParams {
    pub format: TextureFormat,
    pub filter: FilterMode,
    pub wrap: WrapMode,
}

impl TextureParams {
    pub fn sprite_default() -> Self {
        Self {
            format: TextureFormat::Rgba8,
            filter: FilterMode::Nearest,
            wrap: WrapMode::Repeat,
        }
    }

    pub fn glyph_atlas_default() -> Self {
        Self {
            format: TextureFormat::R8,
            filter: FilterMode::Nearest,
            wrap: WrapMode::ClampToEdge,
        }
    }
}

#[derive(Debug)]
pub struct Texture2D {
    id: glow::Texture,
    width: u32,
    height: u32,
    format: TextureFormat,
    gl: Rc<glow::Context>,
}

impl Texture2D {
    pub fn id(&self) -> glow::Texture { self.id }
    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }

    pub fn format(&self) -> TextureFormat { self.format }

    pub fn bind(&self, unit: u32) {
        unsafe {
            self.gl.active_texture(glow::TEXTURE0 + unit);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
        }
    }

    pub fn new_empty(
        gl: Rc<glow::Context>,
        width: u32,
        height: u32,
        params: TextureParams,
    ) -> Result<Rc<Texture2D>, String> {
        let id = create_gl_texture(&gl, width, height, params, None)
            .map_err(|e| format!("Failed to create empty {}x{} texture: {}", width, height, e))?;

        Ok(Rc::new(Texture2D {
            id,
            width,
            height,
            format: params.format,
            gl
        }))
    }

    /// Uploads pixel data into a rectangular sub-region of this texture, like placing a
    /// freshly rasterized glyph bitmap into its slot in an atlas page.
    pub fn upload_sub_image(&self, x: u32, y: u32, width: u32, height: u32, pixels: &[u8]) {
        debug_assert_eq!(
            pixels.len(),
            (width * height) as usize * self.format.bytes_per_pixel(),
            "Pixel buffer length does not match width*height*bytes_per_pixel for this texture's format"
        );

        debug_assert!(
            x + width <= self.width && y + height <= self.height,
            "sub-image rect ({}, {}, {}, {}) does not fit inside {}x{} texture",
            x, y, width, height, self.width, self.height
        );

        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.id));

            self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                x as i32,
                y as i32,
                width as i32,
                height as i32,
                self.format.gl_format(),
                self.format.gl_type(),
                glow::PixelUnpackData::Slice(Some(pixels)),
            );
            self.gl.bind_texture(glow::TEXTURE_2D, None);
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

        let params = TextureParams::sprite_default();

        let id = create_gl_texture(&gl, width, height, params, Some(&pixels))
            .map_err(|e| format!("Failed to create texture for {:?}: {}", path, e))?;

        let texture = Rc::new(Texture2D {
            id,
            width,
            height,
            gl,
            format: params.format,
        });
        
        self.textures.insert(path, texture.clone());
        Ok(texture)
    }
    
    pub fn get(&self, path: impl AsRef<Path>) -> Option<Rc<Texture2D>> {
        self.textures.get(path.as_ref()).cloned()
    }
}

fn create_gl_texture(
    gl: &glow::Context,
    width: u32,
    height: u32,
    params: TextureParams,
    pixels: Option<&[u8]>
) -> Result<glow::Texture, String> {
    unsafe {
        let texture = gl.create_texture().map_err(|e| e.to_string())?;

        gl.bind_texture(glow::TEXTURE_2D, Some(texture));

        let wrap = params.wrap.to_gl();
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap);

        let filter = params.filter.to_gl();
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter);

        if params.format == TextureFormat::R8 {
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_SWIZZLE_R, glow::RED as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_SWIZZLE_G, glow::RED as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_SWIZZLE_B, glow::RED as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_SWIZZLE_A, glow::RED as i32);
        }

        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            params.format.gl_internal_format(),
            width as i32,
            height as i32,
            0,
            params.format.gl_format(),
            params.format.gl_type(),
            glow::PixelUnpackData::Slice(pixels),
        );

        gl.bind_texture(glow::TEXTURE_2D, None);

        Ok(texture)
    }
}
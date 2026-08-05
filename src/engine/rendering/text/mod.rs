use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use glam::Vec2;
use crate::engine::rendering::{LocalTransform, TextureProvider, TextureRenderData};
use crate::engine::rendering::text::atlas_packer::{Rect, ShelfPacker};
use crate::engine::rendering::texture::{Texture2D, TextureParams};

mod atlas_packer;

pub const DEFAULT_ATLAS_PAGE_SIZE: u32 = 1024;
pub const DEFAULT_GLYPH_PADDING: u32 = 1;

pub type FontID = u32;

struct LoadedFont {
    font: fontdue::Font,
    #[allow(dead_code)]
    path: PathBuf,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct GlyphKey {
    font: FontID,
    size_bits: u32,
    ch: char,
}

#[derive(Clone, Debug)]
pub struct GlyphVisual {
    pub texture: Rc<Texture2D>,
    pub tex_offset: Vec2,
    pub region_size: Vec2,
}

/// Layout metrics for a glyph, in pixels at the size it was rasterized.
#[derive(Clone, Copy, Debug)]
pub struct GlyphMetrics {
    /// Horizontal distance to move the pen after placing this glyph
    pub advance_width: f32,
    /// Offset from the pen (on the baseline) to the left edge of the glyph's bitmap
    pub bearing_x: f32,
    /// Offset from the pen (on the baseline) to the bottom edge of the glyph's bitmap.
    pub bearing_y: f32,

    pub width: f32,
    pub height: f32,
}

/// A single character at a specific font and pixel size.
/// visual is None for glyphs with no ink (like spaces)
/// -> RenderItems should not be emitted for these
#[derive(Clone, Debug)]
pub struct Glyph {
    pub metrics: GlyphMetrics,
    pub visual: Option<GlyphVisual>,
    pub ch: char,
}

impl Glyph {
    pub fn advance(&self, pen: Vec2) -> Vec2 {
        Vec2::new(pen.x + self.metrics.advance_width, pen.y)
    }

    pub fn quad_local_transform(&self, pen: Vec2) -> Option<LocalTransform> {
        self.visual.as_ref()?;
        let m = &self.metrics;
        let center = Vec2::new(
            pen.x + m.bearing_x + m.width * 0.5,
            pen.y + m.bearing_y + m.height * 0.5,
        );
        Some(LocalTransform {
            offset: center,
            rotation: 0.0,
            pixel_size: Vec2::new(m.width, m.height),
        })
    }
}

impl TextureProvider for Glyph {
    fn get_texture_render_data(&self) -> Option<TextureRenderData> {
        let visual = self.visual.as_ref()?;
        Some(TextureRenderData {
            texture_id: visual.texture.id(),
            tex_offset: visual.tex_offset,
            region_size: visual.region_size,
            max_uvs: Vec2::new(1.0, 1.0),
        })
    }
}

/// Vertical metrics for laying out multiple lines of text
#[derive(Clone, Copy, Debug)]
pub struct FontLineMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    /// Distance from one line's baseline to the next.
    pub new_line_size: f32,
}

struct AtlasPage {
    texture: Rc<Texture2D>,
    packer: ShelfPacker,
}

pub struct TextManager {
    font_paths: HashMap<PathBuf, FontID>,
    fonts: Vec<LoadedFont>,
    glyphs: HashMap<GlyphKey, Glyph>,
    pages: Vec<AtlasPage>,
    page_size: u32,
    glyph_padding: u32,
}

impl TextManager {
    pub fn new() -> Self {
        Self::with_atlas_config(DEFAULT_ATLAS_PAGE_SIZE, DEFAULT_GLYPH_PADDING)
    }

    pub fn with_atlas_config(page_size: u32, glyph_padding: u32) -> Self {
        Self {
            font_paths: HashMap::new(),
            fonts: Vec::new(),
            glyphs: HashMap::new(),
            pages: Vec::new(),
            page_size,
            glyph_padding,
        }
    }

    /// Lazily loads a font file from disk.
    /// Repeated calls with the same paths do not reload the same file, but just return
    /// the already loaded FontId
    pub fn load_font(&mut self, path: impl AsRef<Path>) -> Result<FontID, String> {
        let path = path.as_ref().to_path_buf();

        if let Some(&id) = self.font_paths.get(&path) {
            return Ok(id);
        }

        let bytes = std::fs::read(&path)
            .map_err(|e| format!("Failed to read font file {:?}: {}", path, e))?;

        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| format!("Failed to parse font file {:?}: {}", path, e))?;

        let id = self.fonts.len() as FontID;
        self.fonts.push(LoadedFont {
            font,
            path: path.clone(),
        });
        self.font_paths.insert(path, id);

        Ok(id)
    }

    pub fn font_line_metrics(&self, font: FontID, size_px: f32) -> Option<FontLineMetrics> {
        let loaded = self.fonts.get(font as usize)?;
        let m = loaded.font.horizontal_line_metrics(size_px)?;
        Some(FontLineMetrics {
            ascent: m.ascent,
            descent: m.descent,
            line_gap: m.line_gap,
            new_line_size: m.new_line_size,
        })
    }

    /// Returns the glyph for ch at size_px in font, rasterizing and
    /// atlas-packing it on first request. gl is only actually touched
    /// on a chache miss.
    pub fn glyph(
        &mut self,
        gl: Rc<glow::Context>,
        font: FontID,
        size_px: f32,
        ch: char,
    ) -> Result<Glyph, String> {
        let key = GlyphKey {
            font,
            size_bits: size_px.to_bits(),
            ch,
        };

        if let Some(glyph) = self.glyphs.get(&key) {
            return Ok(glyph.clone());
        }

        let glyph = self.rasterize_and_pack(gl, font, size_px, ch)?;
        self.glyphs.insert(key, glyph.clone());

        Ok(glyph)
    }

    fn rasterize_and_pack(
        &mut self,
        gl: Rc<glow::Context>,
        font: FontID,
        size_px: f32,
        ch: char,
    ) -> Result<Glyph, String> {
        let loaded = self
            .fonts
            .get(font as usize)
            .ok_or_else(|| format!("TextManager::rasterize_and_pack - Unknown FontID {:?}", font))?;

        let (metrics, bitmap) = loaded.font.rasterize(ch, size_px);

        let glyph_metrics = GlyphMetrics {
            advance_width: metrics.advance_width,
            bearing_x: metrics.xmin as f32,
            bearing_y: metrics.ymin as f32,
            width: metrics.width as f32,
            height: metrics.height as f32,
        };

        if metrics.width == 0 || metrics.height == 0 {
            return Ok(Glyph{
                metrics: glyph_metrics,
                visual: None,
                ch
            })
        };

        let (texture, rect) = self.pack_glyph(
            gl,
            metrics.width as u32,
            metrics.height as u32,
            &bitmap
        )?;

        let atlas_w = texture.width() as f32;
        let atlas_h = texture.height() as f32;

        // fontdue's bitmap row 0 is the visual top of the glyph, so the uv data
        // for the glyph's visuals need to be moved from TL to BL
        let tex_offset = Vec2::new(
            rect.x as f32 / atlas_w,
            (rect.y as f32 + rect.height as f32) / atlas_h,
        );
        let region_size = Vec2::new(rect.width as f32 / atlas_w, -(rect.height as f32) / atlas_h);

        Ok(Glyph {
            metrics: glyph_metrics,
            visual: Some(GlyphVisual {
                texture,
                tex_offset,
                region_size
            }),
            ch
        })
    }

    fn pack_glyph(
        &mut self,
        gl: Rc<glow::Context>,
        width: u32,
        height: u32,
        bitmap: &[u8],
    ) -> Result<(Rc<Texture2D>, Rect), String> {
        if let Some(page) = self.pages.last_mut() {
            if let Some(rect) = page.packer.allocate(width, height) {
                page.texture
                    .upload_sub_image(rect.x, rect.y, width, height, bitmap);
                return Ok((page.texture.clone(), rect));
            }
        }

        let texture = Texture2D::new_empty(
            gl,
            self.page_size,
            self.page_size,
            TextureParams::glyph_atlas_default()
        )?;

        let mut packer = ShelfPacker::new(self.page_size, self.page_size, self.glyph_padding);

        let rect = packer.allocate(width, height).ok_or_else(|| {
            format!(
                "Glyph bitmap {}x{} does not fit in a fresh {}x{} atlas page.\
                Either increase the atlas page size, or this character's pixel size is unreasonably large.",
                width, height, self.page_size, self.page_size
            )
        })?;

        texture.upload_sub_image(rect.x, rect.y, width, height, bitmap);

        self.pages.push(AtlasPage{
            texture: texture.clone(),
            packer,
        });

        Ok((texture, rect))
    }
}
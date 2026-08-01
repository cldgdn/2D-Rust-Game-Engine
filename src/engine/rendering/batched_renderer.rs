use std::collections::HashMap;
use std::rc::Rc;

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use glow::HasContext;

use crate::engine::rendering::RenderItem;
use crate::engine::rendering::shader::{GlslType, Shader, ShaderDescriptor, ShaderID};
use crate::engine::world::World;
use crate::engine::{Drawable, EntityID};
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::game::Game;
use crate::engine::rendering::pass::OrderingRule;
// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum number of quads that can appear in a single batch.
/// Pre-allocates the staging buffer and EBO at init time.
const MAX_QUADS_PER_BATCH: usize = 4096;

const VERTICES_PER_QUAD: usize = 4;
const INDICES_PER_QUAD:  usize = 6;

pub struct PassInputs<'a> {
    pub view_projection: &'a Mat4,
    pub camera_position: Vec2,
    pub ordering_rule: &'a dyn OrderingRule
}

// ─────────────────────────────────────────────────────────────────────────────
// FrameContext — read-only data the renderer makes available to UniformProviders
// ─────────────────────────────────────────────────────────────────────────────

/// Snapshot of per-frame global rendering state. Passed to every
/// `UniformProvider` after its shader program is bound.
pub struct FrameContext<'a> {
    pub view_projection: &'a Mat4,
    pub camera_position: Vec2,
    pub elapsed_time:    f32,
    pub delta_time:      f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// UniformProvider — per-shader hook for setting custom uniforms
// ─────────────────────────────────────────────────────────────────────────────

/// Implement this to supply custom per-batch uniforms for a specific shader.
/// The renderer calls `provide` once per batch, immediately after binding
/// the shader program, before issuing the draw call.
///
/// Standard uniforms (`uViewProjection`, `uTexture`) are set by the renderer
/// and must not be set here.
pub trait UniformProvider {
    fn provide(&self, shader: &Shader, ctx: &FrameContext);
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal batch grouping
// ─────────────────────────────────────────────────────────────────────────────

/// A transient group of `RenderItem`s that share the same shader and texture.
/// Built and consumed within a single frame; never stored across frames.
struct Batch {
    shader_id:  ShaderID,
    texture_id: glow::Texture,
    /// Indices into the per-frame `RenderItem` list.
    items: Vec<usize>,
}

// ─────────────────────────────────────────────────────────────────────────────
// VAO cache entry
// ─────────────────────────────────────────────────────────────────────────────

/// One VAO configured for a specific vertex stride/layout.
/// The VAO records the `vertex_attrib_pointer` calls for its descriptor,
/// so the renderer just binds it before a draw call — no re-configuration.
struct ShaderVao {
    vao: glow::VertexArray,
}

// ─────────────────────────────────────────────────────────────────────────────
// BatchRenderer
// ─────────────────────────────────────────────────────────────────────────────

pub struct BatchRenderer {
    gl: Rc<glow::Context>,

    /// Dynamic VBO — re-filled every frame with packed vertex data.
    vbo: glow::Buffer,

    /// Static EBO — pre-filled with indices for MAX_QUADS_PER_BATCH quads.
    ebo: glow::Buffer,

    /// CPU-side staging buffer. Filled with raw bytes, then uploaded via
    /// `buffer_sub_data`. Avoids a heap allocation per frame.
    staging: Vec<u8>,

    /// One VAO per shader (since different ShaderDescriptors may differ in
    /// stride and attribute layout). Created lazily on first use.
    vao_cache: HashMap<ShaderID, ShaderVao>,

    /// Optional per-shader uniform providers registered by the user.
    uniform_providers: HashMap<ShaderID, Box<dyn UniformProvider>>,

    /// Descriptors registered alongside compiled shaders. The renderer
    /// consults these to know each shader's vertex stride and attribute layout.
    descriptors: HashMap<ShaderID, ShaderDescriptor>,
}

impl BatchRenderer {
    pub fn new(gl: Rc<glow::Context>) -> Self {
        let max_vertex_bytes = MAX_QUADS_PER_BATCH
            * VERTICES_PER_QUAD
            // Use the standard stride as the default allocation; extended
            // shaders with larger strides will still fit as long as total
            // bytes don't exceed this. For safety, over-allocate ×2.
            * ShaderDescriptor::STANDARD_STRIDE
            * 2;

        let (vbo, ebo) = unsafe {
            // Dynamic VBO — contents change every frame.
            let vbo = gl
                .create_buffer()
                .expect("BatchRenderer: failed to create VBO");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_size(
                glow::ARRAY_BUFFER,
                max_vertex_bytes as i32,
                glow::DYNAMIC_DRAW,
            );

            // Static EBO — index pattern repeats for every quad.
            let ebo = gl
                .create_buffer()
                .expect("BatchRenderer: failed to create EBO");
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            let indices = build_quad_indices(MAX_QUADS_PER_BATCH);
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&indices),
                glow::STATIC_DRAW,
            );

            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);

            (vbo, ebo)
        };

        Self {
            gl,
            vbo,
            ebo,
            staging: Vec::with_capacity(max_vertex_bytes),
            vao_cache: HashMap::new(),
            uniform_providers: HashMap::new(),
            descriptors: HashMap::new(),
        }
    }

    // ── Registration API ──────────────────────────────────────────────────────

    /// Register a `ShaderDescriptor` for a compiled shader. Must be called
    /// before any entity requests that shader, typically at engine startup
    /// alongside shader compilation.
    pub fn register_descriptor(&mut self, shader_id: ShaderID, descriptor: ShaderDescriptor) {
        self.descriptors.insert(shader_id, descriptor);
    }

    /// Register a custom `UniformProvider` for a shader. Optional — shaders
    /// with no provider rely solely on the standard uniforms the renderer sets.
    pub fn register_uniform_provider(
        &mut self,
        shader_id: ShaderID,
        provider: Box<dyn UniformProvider>,
    ) {
        self.uniform_providers.insert(shader_id, provider);
    }

    // ── Main render entry point ───────────────────────────────────────────────

    /// Called once per frame from `Game::draw_phase`.
    /// Collects, sorts, batches, and draws all visible entities.
    pub fn render(
        &mut self,
        items: &Vec<RenderItem>,
        world: &World,
        shaders: &HashMap<ShaderID, Shader>,
        inputs: &PassInputs,
        elapsed_time: f32,
        delta_time: f32,
    ) {
        if items.is_empty() {
            return;
        }

        // ── 1. Build batches (sorted by z then grouped by shader+texture) ─────
        let batches = self.build_batches(&items, world, inputs.ordering_rule);

        // ── 2. Assemble FrameContext for UniformProviders ─────────────────────
        let ctx = FrameContext {
            view_projection: inputs.view_projection,
            camera_position: inputs.camera_position,
            elapsed_time,
            delta_time,
        };

        // ── 3. Draw each batch ────────────────────────────────────────────────
        let descriptors: HashMap<ShaderID, ShaderDescriptor> = batches
            .iter()
            .filter_map(|b| {
                self.descriptors
                    .get(&b.shader_id)
                    .map(|d| (b.shader_id, d.clone()))
            })
            .collect();

        for batch in &batches {
            let shader = match shaders.get(&batch.shader_id) {
                Some(s) => s,
                None => {
                    eprintln!("BatchRenderer::render - unknown ShaderID {:?}", batch.shader_id);
                    continue;
                }
            };

            let descriptor = match descriptors.get(&batch.shader_id) {
                Some(d) => d.clone(),
                None => {
                    eprintln!(
                        "BatchRenderer::render - no ShaderDescriptor for ShaderID {:?}",
                        batch.shader_id
                    );
                    continue;
                }
            };

            self.draw_batch(batch, &items, world, shader, &descriptor, &ctx, inputs.ordering_rule);
        }

        unsafe {
            self.gl.bind_vertex_array(None);
        }
    }

    // ── Step 1: sort + batch ──────────────────────────────────────────────────

    fn build_batches(
        &self,
        items: &[RenderItem],
        world: &World,
        ordering_rule: &dyn OrderingRule,
    ) -> Vec<Batch> {
        // Sort indices rather than the items themselves so we keep a stable
        // mapping between index and item for the drawing step.
        let mut order: Vec<usize> = (0..items.len()).collect();

        order.sort_by(|&a, &b| {
            let ia = &items[a];
            let ib = &items[b];

            let da = ordering_rule.compute_depth(ia, world);
            let db = ordering_rule.compute_depth(ib, world);

            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| ia.shader_id.cmp(&ib.shader_id))
                .then_with(|| ia.texture_id.cmp(&ib.texture_id))
        });

        // Group consecutive items with the same (shader, texture) into batches.
        let mut batches: Vec<Batch> = Vec::new();

        for idx in order {
            let item = &items[idx];
            let needs_new_batch = batches.last().map_or(true, |b: &Batch| {
                b.shader_id != item.shader_id || b.texture_id != item.texture_id
            });

            if needs_new_batch {
                batches.push(Batch {
                    shader_id:  item.shader_id,
                    texture_id: item.texture_id,
                    items:      vec![idx],
                });
            } else {
                batches.last_mut().unwrap().items.push(idx);
            }
        }

        batches
    }

    // ── Step 3: draw a single batch ───────────────────────────────────────────

    fn draw_batch(
        &mut self,
        batch: &Batch,
        items: &[RenderItem],
        world: &World,
        shader: &Shader,
        descriptor: &ShaderDescriptor,
        ctx: &FrameContext,
        ordering_rule: &dyn OrderingRule,
    ) {
        // Ensure a VAO exists for this shader's layout.
        self.ensure_vao(batch.shader_id, descriptor);

        // Fill the staging buffer with packed vertex data for all quads.
        self.staging.clear();
        let mut quad_count = 0usize;

        for &idx in &batch.items {
            let item = &items[idx];

            // Fetch the entity's transform — skip silently if missing.
            let transform = match world.get_component::<Transform>(item.entity_id) {
                Some(t) => t,
                None => continue,
            };

            let model = compute_model_matrix(
                transform.position(),
                transform.rotation(),
                &transform.total_dimensions(),
            );

            let depth = ordering_rule.compute_depth(item, world);

            let (mirror_x, mirror_y) = transform.get_mirror_visual();
            let tex_offset = Vec2::new(
                if mirror_x {item.tex_offset.x + item.region_size.x} else {item.tex_offset.x},
                if mirror_y {item.tex_offset.y + item.region_size.y} else {item.tex_offset.y},
            );
            let region_size = Vec2::new(
                if mirror_x {-item.region_size.x} else {item.region_size.x},
                if mirror_y {-item.region_size.y} else {item.region_size.y},
            );
            
            write_quad(
                &mut self.staging,
                &model,
                tex_offset,
                region_size,
                item.max_uvs,
                depth,
                item.color_tint,
                item.extended_data.as_deref(),
                descriptor,
            );

            quad_count += 1;

            if quad_count >= MAX_QUADS_PER_BATCH {
                eprintln!("BatchRenderer: MAX_QUADS_PER_BATCH exceeded, flush early");
                break;
            }
        }

        if quad_count == 0 {
            return;
        }

        unsafe {
            // Bind the VAO for this shader's layout.
            let vao = self.vao_cache[&batch.shader_id].vao;
            self.gl.bind_vertex_array(Some(vao));

            // Upload vertex data for this batch.
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            self.gl.buffer_sub_data_u8_slice(
                glow::ARRAY_BUFFER,
                0,
                &self.staging,
            );

            // Bind shader and set standard uniforms.
            shader.use_program();
            shader.set_mat4("uViewProjection", ctx.view_projection);
            shader.set_int("uTexture", 0);

            // Let the UniformProvider set any custom uniforms.
            if let Some(provider) = self.uniform_providers.get(&batch.shader_id) {
                provider.provide(shader, ctx);
            }

            // Bind the texture for this batch.
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture_unit(0, Some(batch.texture_id));

            // Draw.
            self.gl.draw_elements(
                glow::TRIANGLES,
                (quad_count * INDICES_PER_QUAD) as i32,
                glow::UNSIGNED_INT,
                0,
            );
        }
    }

    // ── VAO management ────────────────────────────────────────────────────────

    /// Creates and caches a VAO for the given shader's descriptor if one does
    /// not already exist. The VAO records all `vertex_attrib_pointer` calls so
    /// the renderer only needs to bind it, not re-configure it, each frame.
    fn ensure_vao(&mut self, shader_id: ShaderID, descriptor: &ShaderDescriptor) {
        if self.vao_cache.contains_key(&shader_id) {
            return;
        }

        let vao = unsafe {
            let vao = self
                .gl
                .create_vertex_array()
                .expect("BatchRenderer: failed to create VAO");

            self.gl.bind_vertex_array(Some(vao));
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ebo));

            let stride = descriptor.vertex_stride as i32;

            for attr in &descriptor.attributes {
                configure_attrib(&self.gl, attr.location, &attr.glsl_type, stride, attr.offset);
            }

            self.gl.bind_vertex_array(None);
            vao
        };

        self.vao_cache.insert(shader_id, ShaderVao { vao });
    }
}

impl Drop for BatchRenderer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.vbo);
            self.gl.delete_buffer(self.ebo);
            for entry in self.vao_cache.values() {
                self.gl.delete_vertex_array(entry.vao);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Vertex writing
// ─────────────────────────────────────────────────────────────────────────────

/// Writes the 4 vertices of one quad into `buf`, packed according to the
/// standard layout followed by any extended bytes.
///
/// Standard layout per vertex (100 bytes):
///   aPos        vec3  →  offset  0  (12 bytes)
///   aTexCoord   vec2  →  offset 12  ( 8 bytes)
///   aTexOffset  vec2  →  offset 20  ( 8 bytes)
///   aRegionSize vec2  →  offset 28  ( 8 bytes)
///   aModel      mat4  →  offset 36  (64 bytes)
fn write_quad(
    buf: &mut Vec<u8>,
    model: &Mat4,
    tex_offset: Vec2,
    region_size: Vec2,
    max_uvs: Vec2,
    z: f32,
    _color_tint: Vec4,
    extended: Option<&[u8]>,
    descriptor: &ShaderDescriptor,
) {
    // Unit quad corners: (local_pos, uv)
    //   TL (0,1) → uv (0,1)
    //   TR (1,1) → uv (1,1)
    //   BR (1,0) → uv (1,0)
    //   BL (0,0) → uv (0,0)
    let corners: [(f32, f32, f32, f32); 4] = [
        (-0.5,  0.5, 0.0,       max_uvs.y), // TL
        ( 0.5,  0.5, max_uvs.x, max_uvs.y), // TR
        ( 0.5, -0.5, max_uvs.x, 0.0),       // BR
        (-0.5, -0.5, 0.0,       0.0),       // BL
    ];

    let model_cols: [[f32; 4]; 4] = [
        model.col(0).into(),
        model.col(1).into(),
        model.col(2).into(),
        model.col(3).into(),
    ];

    for (lx, ly, u, v) in corners {
        // aPos (vec3): xy are local corner, z is depth
        push_f32s(buf, &[lx, ly, z]);

        // aTexCoord (vec2)
        push_f32s(buf, &[u, v]);

        // aTexOffset (vec2)
        push_f32s(buf, &[tex_offset.x, tex_offset.y]);

        // aRegionSize (vec2)
        push_f32s(buf, &[region_size.x, region_size.y]);

        // aModel (mat4 — 4 × vec4, column-major)
        for col in &model_cols {
            push_f32s(buf, col);
        }

        // Extended per-vertex data (same blob repeated for each vertex,
        // since extended data is per-object, not per-vertex).
        if let Some(ext) = extended {
            buf.extend_from_slice(ext);
        } else {
            // Pad to the full stride so the VBO layout is consistent.
            let ext_bytes = descriptor.vertex_stride - ShaderDescriptor::STANDARD_STRIDE;
            if ext_bytes > 0 {
                buf.resize(buf.len() + ext_bytes, 0u8);
            }
        }
    }
}

#[inline]
fn push_f32s(buf: &mut Vec<u8>, values: &[f32]) {
    for &v in values {
        buf.extend_from_slice(&v.to_ne_bytes());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Attribute pointer configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Calls `enable_vertex_attrib_array` + `vertex_attrib_pointer` for one
/// attribute, handling `mat4` (which needs 4 consecutive pointer calls).
unsafe fn configure_attrib(
    gl: &glow::Context,
    base_location: u32,
    glsl_type: &GlslType,
    stride: i32,
    base_offset: usize,
) {
    match glsl_type {
        GlslType::Mat4 => {
            // A mat4 occupies 4 consecutive vec4 attribute slots.
            for col in 0..4u32 {
                let loc = base_location + col;
                let offset = (base_offset + col as usize * 16) as i32;
                gl.enable_vertex_attrib_array(loc);
                gl.vertex_attrib_pointer_f32(loc, 4, glow::FLOAT, false, stride, offset);
            }
        }
        GlslType::Mat3 => {
            for col in 0..3u32 {
                let loc = base_location + col;
                let offset = (base_offset + col as usize * 12) as i32;
                gl.enable_vertex_attrib_array(loc);
                gl.vertex_attrib_pointer_f32(loc, 3, glow::FLOAT, false, stride, offset);
            }
        }
        GlslType::Mat2 => {
            for col in 0..2u32 {
                let loc = base_location + col;
                let offset = (base_offset + col as usize * 8) as i32;
                gl.enable_vertex_attrib_array(loc);
                gl.vertex_attrib_pointer_f32(loc, 2, glow::FLOAT, false, stride, offset);
            }
        }
        _ => {
            let components = glsl_type_component_count(glsl_type);
            let gl_type = glsl_base_gl_type(glsl_type);
            gl.enable_vertex_attrib_array(base_location);
            if gl_type == glow::INT || gl_type == glow::UNSIGNED_INT {
                gl.vertex_attrib_pointer_i32(
                    base_location,
                    components,
                    gl_type,
                    stride,
                    base_offset as i32,
                );
            } else {
                gl.vertex_attrib_pointer_f32(
                    base_location,
                    components,
                    glow::FLOAT,
                    false,
                    stride,
                    base_offset as i32,
                );
            }
        }
    }
}

fn glsl_type_component_count(t: &GlslType) -> i32 {
    match t {
        GlslType::Float | GlslType::Int | GlslType::UInt => 1,
        GlslType::Vec2  | GlslType::IVec2                => 2,
        GlslType::Vec3  | GlslType::IVec3                => 3,
        GlslType::Vec4  | GlslType::IVec4                => 4,
        _ => 1,
    }
}

fn glsl_base_gl_type(t: &GlslType) -> u32 {
    match t {
        GlslType::Int | GlslType::IVec2 | GlslType::IVec3 | GlslType::IVec4 => glow::INT,
        GlslType::UInt => glow::UNSIGNED_INT,
        _ => glow::FLOAT,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Index buffer generation
// ─────────────────────────────────────────────────────────────────────────────

/// Generates an index buffer for `quad_count` quads, each using the pattern
/// [0,1,3, 1,2,3] offset by 4×quad_index.
fn build_quad_indices(quad_count: usize) -> Vec<u32> {
    let mut indices = Vec::with_capacity(quad_count * INDICES_PER_QUAD);
    for q in 0..quad_count {
        let base = (q * VERTICES_PER_QUAD) as u32;
        indices.extend_from_slice(&[
            base, base + 1, base + 3,
            base + 1, base + 2, base + 3,
        ]);
    }
    indices
}

// ─────────────────────────────────────────────────────────────────────────────
// Model matrix
// ─────────────────────────────────────────────────────────────────────────────

/// Builds a model matrix from position and scale.
/// Depth is handled separately via `aPos.z` in the vertex shader,
/// so the model matrix operates purely in XY.
fn compute_model_matrix(position: &Vec2, rotation: f32, scale: &Vec2) -> Mat4 {
    let position = position * config().unit_pixel_size as f32;
    let rotation = rotation.to_radians();
    let scale = scale * config().unit_pixel_size as f32;
    let translation = Mat4::from_translation(Vec3::new(position.x, position.y, 0.0));
    let rotation_mat = Mat4::from_quat(Quat::from_rotation_z(rotation));
    let scale_mat   = Mat4::from_scale(Vec3::new(scale.x, scale.y, 1.0));
    translation * rotation_mat * scale_mat
}

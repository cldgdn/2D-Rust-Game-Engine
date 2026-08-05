use glow::HasContext;
use glam::{Mat4, Vec2, Vec3, Vec4};
use std::fs;
use std::path::Path;
use std::rc::Rc;
use crate::engine::config::config;

pub(crate) type ShaderID = u32;

/// Every GLSL type that may appear as a vertex attribute or uniform.
/// Drives both GLSL source codegen and byte-size/slot calculations.
#[derive(Debug, Clone, PartialEq)]
pub enum GlslType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Int,
    IVec2,
    IVec3,
    IVec4,
    UInt,
    Mat2,
    Mat3,
    Mat4,
    Sampler2D,
}

// ─────────────────────────────────────────────────────────────────────────────
// Descriptor types (used by the renderer at runtime)
// ─────────────────────────────────────────────────────────────────────────────

/// Describes a single vertex attribute as the renderer needs to know it:
/// where it lives in the buffer and how OpenGL should interpret it.
#[derive(Debug, Clone)]
pub struct AttributeDescriptor {
    /// `layout(location = N)` index in the shader.
    pub location: u32,
    /// Human-readable name (matches the `in` variable name in GLSL).
    pub name: String,
    /// GLSL type — determines component count and base GL type.
    pub glsl_type: GlslType,
    /// Byte offset from the start of one vertex to this attribute.
    pub offset: usize,
}

/// The complete description of a compiled shader's vertex layout and
/// extended attribute list. The renderer uses this — never the GLSL source.
#[derive(Debug, Clone)]
pub struct ShaderDescriptor {
    /// Total byte size of one vertex in the VBO.
    pub vertex_stride: usize,

    /// All vertex attributes, standard + extended, in buffer order.
    /// The renderer iterates this to call `vertex_attrib_pointer` for each.
    pub attributes: Vec<AttributeDescriptor>,

    /// Names of extended uniforms declared in this shader.
    /// Informational: the UniformProvider is responsible for setting these.
    pub extended_uniform_names: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Builder input types
// ─────────────────────────────────────────────────────────────────────────────

/// An extended per-vertex attribute declared by a custom shader.
#[derive(Debug, Clone)]
struct ExtendedAttribute {
    name: String,
    glsl_type: GlslType,
}

/// An extended uniform declared by a custom shader.
#[derive(Debug, Clone)]
struct ExtendedUniform {
    name: String,
    glsl_type: GlslType,
}

/// A varying (out from vert / in to frag) for custom per-vertex data.
#[derive(Debug, Clone)]
struct ExtendedVarying {
    name: String,
    glsl_type: GlslType,
}

/// Identifies which hook a code snippet targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShaderHook {
    /// Extra `layout(location = N) in <type> <name>;` lines in the vertex shader.
    ExtendedAttributes,
    /// Extra `uniform <type> <name>;` lines (appears in both shaders).
    ExtendedUniforms,
    /// Extra `out <type> <name>;` lines in the vertex shader.
    ExtendedVaryingsOut,
    /// Extra `in <type> <name>;` lines in the fragment shader.
    ExtendedVaryingsIn,
    /// Free-form GLSL injected inside `void main()` of the vertex shader.
    VertexMain,
    /// Free-form GLSL injected inside `void main()` of the fragment shader,
    /// after sampling but before writing `FragColor`.
    FragmentMain,
}

pub struct Shader {
    gl: Rc<glow::Context>,
    program: glow::Program,
}

// ─────────────────────────────────────────────────────────────────────────────
// ShaderBuilder
// ─────────────────────────────────────────────────────────────────────────────

/// Builds a complete shader from the engine templates by accumulating
/// extended attributes, uniforms, varyings, and free-form hook injections.
pub struct ShaderBuilder {
    gl: Rc<glow::Context>,
    extended_attributes: Vec<ExtendedAttribute>,
    extended_uniforms:   Vec<ExtendedUniform>,
    extended_varyings:   Vec<ExtendedVarying>,
    /// Free-form code snippets keyed by hook. Multiple calls to `inject`
    /// for the same hook accumulate lines in insertion order.
    hook_injections: Vec<(ShaderHook, String)>,
}

impl GlslType {
    /// The GLSL keyword for this type.
    pub fn glsl_name(&self) -> &'static str {
        match self {
            GlslType::Float     => "float",
            GlslType::Vec2      => "vec2",
            GlslType::Vec3      => "vec3",
            GlslType::Vec4      => "vec4",
            GlslType::Int       => "int",
            GlslType::IVec2     => "ivec2",
            GlslType::IVec3     => "ivec3",
            GlslType::IVec4     => "ivec4",
            GlslType::UInt      => "uint",
            GlslType::Mat2      => "mat2",
            GlslType::Mat3      => "mat3",
            GlslType::Mat4      => "mat4",
            GlslType::Sampler2D => "sampler2D",
        }
    }

    /// Size in bytes when stored in a vertex buffer.
    /// `Sampler2D` is not valid as a vertex attribute; returns 0 as a sentinel.
    pub fn byte_size(&self) -> usize {
        match self {
            GlslType::Float     => 4,
            GlslType::Vec2      => 8,
            GlslType::Vec3      => 12,
            GlslType::Vec4      => 16,
            GlslType::Int       => 4,
            GlslType::IVec2     => 8,
            GlslType::IVec3     => 12,
            GlslType::IVec4     => 16,
            GlslType::UInt      => 4,
            GlslType::Mat2      => 16,
            GlslType::Mat3      => 36,
            GlslType::Mat4      => 64,
            GlslType::Sampler2D => 0,
        }
    }

    /// Number of consecutive `layout(location = N)` slots consumed.
    /// Matrices consume one slot per column.
    pub fn location_slots(&self) -> u32 {
        match self {
            GlslType::Mat2 => 2,
            GlslType::Mat3 => 3,
            GlslType::Mat4 => 4,
            _              => 1,
        }
    }

    /// Whether this type is valid as a vertex attribute (not a sampler).
    pub fn is_attribute_type(&self) -> bool {
        !matches!(self, GlslType::Sampler2D)
    }
}

impl ShaderDescriptor {
    /// Byte size of the standard (renderer-managed) portion of each vertex.
    ///
    /// Standard layout (tightly packed floats):
    ///   location 0  aPos        vec3   →  12 bytes
    ///   location 1  aTexCoord   vec2   →   8 bytes
    ///   location 2  aTexOffset  vec2   →   8 bytes
    ///   location 3  aRegionSize vec2   →   8 bytes
    ///   location 4  aModel      mat4   →  64 bytes  (locations 4–7)
    ///                                    ──────────
    ///                                    100 bytes total
    pub const STANDARD_STRIDE: usize = 100;

    pub fn default() -> Self {
        ShaderDescriptor {
            vertex_stride: Self::STANDARD_STRIDE,
            attributes: Self::standard_attributes(),
            extended_uniform_names: vec![],
        }
    }

    /// Constructs the standard attribute descriptors with their fixed offsets.
    /// Called internally; exposed so the renderer can re-derive them if needed.
    pub fn standard_attributes() -> Vec<AttributeDescriptor> {
        vec![
            AttributeDescriptor {
                location: 0,
                name: "aPos".into(),
                glsl_type: GlslType::Vec3,
                offset: 0,
            },
            AttributeDescriptor {
                location: 1,
                name: "aTexCoord".into(),
                glsl_type: GlslType::Vec2,
                offset: 12,
            },
            AttributeDescriptor {
                location: 2,
                name: "aTexOffset".into(),
                glsl_type: GlslType::Vec2,
                offset: 20,
            },
            AttributeDescriptor {
                location: 3,
                name: "aRegionSize".into(),
                glsl_type: GlslType::Vec2,
                offset: 28,
            },
            AttributeDescriptor {
                location: 4,
                name: "aModel".into(),
                glsl_type: GlslType::Mat4,
                offset: 36,
            },
        ]
    }
}

impl ShaderHook {
    fn template_marker(&self) -> &'static str {
        match self {
            ShaderHook::ExtendedAttributes  => "{{extended_attributes}}",
            ShaderHook::ExtendedUniforms    => "{{extended_uniforms}}",
            ShaderHook::ExtendedVaryingsOut => "{{extended_varyings_out}}",
            ShaderHook::ExtendedVaryingsIn  => "{{extended_varyings_in}}",
            ShaderHook::VertexMain          => "{{vertex_main_hook}}",
            ShaderHook::FragmentMain        => "{{fragment_main_hook}}",
        }
    }
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

    pub fn set_vec2(&self, name: &str, vec: &Vec2) {
        unsafe {
            if let Some(location) = self.get_uniform_location(name) {
                self.gl.uniform_2_f32(Some(&location), vec.x, vec.y);
            }
        }
    }

    pub fn set_vec3(&self, name: &str, vec: &Vec3) {
        unsafe {
            if let Some(location) = self.get_uniform_location(name) {
                self.gl.uniform_3_f32(Some(&location), vec.x, vec.y, vec.z);
            }
        }
    }

    pub fn set_vec4(&self, name: &str, vec: &Vec4) {
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

impl ShaderBuilder {
    pub fn new(gl: Rc<glow::Context>) -> Self {
        Self {
            gl,
            extended_attributes: Vec::new(),
            extended_uniforms:   Vec::new(),
            extended_varyings:   Vec::new(),
            hook_injections:     Vec::new(),
        }
    }

    // ── Declaration methods ───────────────────────────────────────────────────

    /// Declares an extra per-vertex attribute.
    /// Location is assigned automatically after the standard block (starting at 8).
    /// `glsl_type` must not be `Sampler2D`.
    pub fn add_extended_attribute(mut self, name: &str, glsl_type: GlslType) -> Self {
        assert!(
            glsl_type.is_attribute_type(),
            "Sampler2D is not valid as a vertex attribute"
        );
        self.extended_attributes.push(ExtendedAttribute {
            name: name.to_owned(),
            glsl_type,
        });
        self
    }

    /// Declares an extra uniform available in both shaders.
    pub fn add_extended_uniform(mut self, name: &str, glsl_type: GlslType) -> Self {
        self.extended_uniforms.push(ExtendedUniform {
            name: name.to_owned(),
            glsl_type,
        });
        self
    }

    /// Declares an extra varying passed from the vertex to the fragment shader.
    /// The builder emits a matching `out` in the vertex shader and `in` in the
    /// fragment shader — you don't need to call `inject` for those declarations.
    pub fn add_extended_varying(mut self, name: &str, glsl_type: GlslType) -> Self {
        self.extended_varyings.push(ExtendedVarying {
            name: name.to_owned(),
            glsl_type,
        });
        self
    }

    /// Appends a line of GLSL code to a hook point.
    /// Multiple calls for the same hook accumulate in order.
    pub fn inject(mut self, hook: ShaderHook, code: &str) -> Self {
        self.hook_injections.push((hook, code.to_owned()));
        self
    }

    // ── Build ─────────────────────────────────────────────────────────────────

    /// Resolves all declarations and injections into complete GLSL source,
    /// compiles both shaders, and returns the `Shader` together with its
    /// `ShaderDescriptor` for the renderer.
    pub fn build(self) -> Result<(Shader, ShaderDescriptor), String> {
        let vert_source = self.resolve_vertex_template();
        let frag_source = self.resolve_fragment_template();

        let program = Shader::create_program(&self.gl, &vert_source, &frag_source)?;
        let shader = Shader {
            gl: self.gl.clone(),
            program,
        };
        let descriptor = self.build_descriptor();

        Ok((shader, descriptor))
    }

    // ── Private: GLSL resolution ──────────────────────────────────────────────

    fn resolve_vertex_template(&self) -> String {
        let mut source = fs::read_to_string(config().shader_template_vertex.to_owned())
            .expect("Could not read vertex template");

        // Extended attributes with auto-assigned locations starting at 8
        // (locations 0–3 are standard scalars/vecs; 4–7 are consumed by aModel mat4)
        let mut next_location: u32 = 8;
        let mut attr_lines = String::new();
        for attr in &self.extended_attributes {
            let slots = attr.glsl_type.location_slots();
            attr_lines.push_str(&format!(
                "layout (location = {}) in {} {};\n",
                next_location,
                attr.glsl_type.glsl_name(),
                attr.name,
            ));
            next_location += slots;
        }
        source = replace_hook(&source, ShaderHook::ExtendedAttributes, &attr_lines);

        // Extended uniforms (same in both shaders)
        let uniform_lines = self.build_uniform_lines();
        source = replace_hook(&source, ShaderHook::ExtendedUniforms, &uniform_lines);

        // Extended varyings out
        let varyings_out = self.build_varyings("out");
        source = replace_hook(&source, ShaderHook::ExtendedVaryingsOut, &varyings_out);

        // vertex_main_hook: concatenate all injections for this hook
        let vert_main = self.collect_hook_code(ShaderHook::VertexMain);
        source = replace_hook(&source, ShaderHook::VertexMain, &vert_main);

        source
    }

    fn resolve_fragment_template(&self) -> String {
        let mut source = fs::read_to_string(config().shader_template_fragment.to_owned())
            .expect("Could not read fragment template");

        // Extended varyings in
        let varyings_in = self.build_varyings("in");
        source = replace_hook(&source, ShaderHook::ExtendedVaryingsIn, &varyings_in);

        // Extended uniforms (same in both shaders)
        let uniform_lines = self.build_uniform_lines();
        source = replace_hook(&source, ShaderHook::ExtendedUniforms, &uniform_lines);

        // fragment_main_hook
        let frag_main = self.collect_hook_code(ShaderHook::FragmentMain);
        source = replace_hook(&source, ShaderHook::FragmentMain, &frag_main);

        source
    }

    fn build_uniform_lines(&self) -> String {
        self.extended_uniforms
            .iter()
            .map(|u| format!("uniform {} {};\n", u.glsl_type.glsl_name(), u.name))
            .collect()
    }

    fn build_varyings(&self, qualifier: &str) -> String {
        self.extended_varyings
            .iter()
            .map(|v| format!("{} {} {};\n", qualifier, v.glsl_type.glsl_name(), v.name))
            .collect()
    }

    fn collect_hook_code(&self, hook: ShaderHook) -> String {
        self.hook_injections
            .iter()
            .filter(|(h, _)| *h == hook)
            .map(|(_, code)| format!("    {}\n", code))
            .collect()
    }

    // ── Private: descriptor construction ─────────────────────────────────────

    fn build_descriptor(&self) -> ShaderDescriptor {
        let mut attributes = ShaderDescriptor::standard_attributes();

        // Walk extended attributes to compute their byte offsets and locations
        let mut next_location: u32 = 8;
        let mut current_offset: usize = ShaderDescriptor::STANDARD_STRIDE;

        for attr in &self.extended_attributes {
            attributes.push(AttributeDescriptor {
                location: next_location,
                name: attr.name.clone(),
                glsl_type: attr.glsl_type.clone(),
                offset: current_offset,
            });
            next_location += attr.glsl_type.location_slots();
            current_offset += attr.glsl_type.byte_size();
        }

        let extended_uniform_names = self
            .extended_uniforms
            .iter()
            .map(|u| u.name.clone())
            .collect();

        ShaderDescriptor {
            vertex_stride: current_offset,
            attributes,
            extended_uniform_names,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Replaces the marker line for `hook` in `source` with `replacement`.
/// If `replacement` is empty the marker line is simply removed.
fn replace_hook(source: &str, hook: ShaderHook, replacement: &str) -> String {
    source.replace(hook.template_marker(), replacement.trim_end_matches('\n'))
}
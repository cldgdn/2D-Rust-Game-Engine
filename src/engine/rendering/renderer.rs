use std::collections::HashMap;
use std::rc::Rc;
use glam::{Mat4, Vec2, Vec4};
use glow::HasContext;
use crate::engine::{Entity, EntityID};
use crate::engine::camera::CameraID;
use crate::engine::rendering::batched_renderer::{BatchRenderer, FrameContext, PassInputs};
use crate::engine::rendering::framebuffer::Framebuffer;
use crate::engine::rendering::pass::{PixelRect, RenderPass};
use crate::engine::rendering::post_process::PostProcessLayer;
use crate::engine::rendering::post_process_renderer::PostProcessRenderer;
use crate::engine::rendering::shader::{Shader, ShaderID};
use crate::engine::world::World;

pub struct Renderer {
    gl: Rc<glow::Context>,

    batch_renderer: BatchRenderer,
    post_process_renderer: PostProcessRenderer,

    pub passes: Vec<RenderPass>,
    global_post_fx: Vec<PostProcessLayer>,
    /// Built-in passthrough shader for the final present step. Compiled once at construction.
    present_shader: Shader,

    /// Base scene composite framebuffer. Every RenderPass with an empty local post-fx
    /// list renders directly here. Passes with local post-fx composite their result here
    /// after finishing their own C/D ping-pong.
    fbo_a: Framebuffer,
    /// Ping-pong partner for the *global* post-fx queue, applied to the fully composited
    /// frame in `fbo_a` after every pass has drawn.
    fbo_b: Framebuffer,
    /// Local post-fx source. A single reusable FBO shared sequentially by whichever pass
    /// currently needs local post-fx.
    fbo_c: Framebuffer,
    /// Ping-pong partner for `fbo_c`.
    fbo_d: Framebuffer,

    screen_width: u32,
    screen_height: u32,
}

impl Renderer {
    pub fn new(gl: Rc<glow::Context>, screen_width: u32, screen_height: u32) -> Self {
        let batch_renderer = BatchRenderer::new(gl.clone());
        let post_process_renderer = PostProcessRenderer::new(gl.clone());

        let present_shader = Shader::new(
            gl.clone(),
            "shaders/present.vert",
            "shaders/present.frag",
        ).expect("Renderer::new - Failed to create present shader");

        let fbo_a = Framebuffer::new(gl.clone(), screen_width, screen_height, true);
        let fbo_b = Framebuffer::new(gl.clone(), screen_width, screen_height, false);
        let fbo_c = Framebuffer::new(gl.clone(), screen_width, screen_height, true);
        let fbo_d = Framebuffer::new(gl.clone(), screen_width, screen_height, false);

        Self {
            gl,
            batch_renderer,
            post_process_renderer,
            passes: Vec::new(),
            global_post_fx: Vec::new(),
            present_shader,
            fbo_a,
            fbo_b,
            fbo_c,
            fbo_d,
            screen_width,
            screen_height,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.screen_width = width;
        self.screen_height = height;

        self.fbo_a.resize(width, height);
        self.fbo_b.resize(width, height);
        self.fbo_c.resize(width, height);
        self.fbo_d.resize(width, height);
    }

    pub fn add_pass(&mut self, pass: RenderPass) {
        self.passes.push(pass);
    }

    pub fn add_global_post_fx(&mut self, layer: PostProcessLayer) {
        self.global_post_fx.push(layer);
    }

    pub fn batch_renderer_mut(&mut self) -> &mut BatchRenderer {
        &mut self.batch_renderer
    }

    pub fn pass_mut(&mut self, name: &str) -> Option<&mut RenderPass> {
        self.passes.iter_mut().find(|p| p.name == name)
    }

    pub fn global_post_fx_mut(&mut self, name: &str) -> Option<&mut PostProcessLayer> {
        self.global_post_fx.iter_mut().find(|l| l.name == name)
    }

    pub fn render_frame(
        &mut self,
        entities: &HashMap<EntityID, Box<dyn Entity>>,
        world: &World,
        elapsed_time: f32,
        delta_time: f32,
    ) {
        let gl = self.gl.clone();

        let Self {
            batch_renderer,
            post_process_renderer,
            present_shader,
            passes,
            global_post_fx,
            fbo_a,
            fbo_b,
            fbo_c,
            fbo_d,
            screen_width,
            screen_height,
            ..
        } = self;

        let shaders = world.shaders();

        // Step 1. Collect RenderItems into each enabled pass.

        for pass in passes.iter_mut() {
            pass.clear_items();
        }

        for entity in entities.values() {
            let Some(drawable) = entity.as_drawable() else {
                continue;
            };
            let mask = drawable.pass_mask();

            for pass in passes.iter_mut() {
                if pass.enabled && pass.matches(mask) {
                    drawable.collect_render_items(world, &mut pass.items);
                }
            }
        }

        // Step 2. Draw passes, highest priority first.
        passes.sort_by(|a, b| b.priority.cmp(&a.priority));

        let screen_rect = PixelRect {
            x: 0,
            y: 0,
            width: *screen_width as i32,
            height: *screen_height as i32,
        };

        for pass in passes.iter() {
            if !pass.enabled || pass.items.is_empty() {
                continue;
            }

            let rect = pass.target.rect().to_pixels(*screen_width, *screen_height);
            let (view_projection, camera_position) = resolve_camera(world, pass.camera_id);

            if pass.local_post_fx.is_empty() {
                draw_pass_into(
                    &gl,
                    fbo_a,
                    rect,
                    pass,
                    false,
                    batch_renderer,
                    world,
                    shaders,
                    &view_projection,
                    camera_position,
                    elapsed_time,
                    delta_time,
                );
            } else {
                draw_pass_into(
                    &gl,
                    fbo_c,
                    rect,
                    pass,
                    true,
                    batch_renderer,
                    world,
                    shaders,
                    &view_projection,
                    camera_position,
                    elapsed_time,
                    delta_time,
                );

                let result = run_post_fx_chain(
                    &gl,
                    post_process_renderer,
                    shaders,
                    &pass.local_post_fx,
                    fbo_c,
                    fbo_d,
                    rect,
                    &view_projection,
                    camera_position,
                    elapsed_time,
                    delta_time,
                );

                result.blit_color_into(fbo_a, rect.x, rect.y, rect.width, rect.height);
            }
        }

        // Step 3. Global post-fx over the fully composited frame.
        // This step takes in the default camera, to ensure a camera is actually found.
        let (global_vp, global_cam_pos) = resolve_camera(world, 0);

        let final_frame = run_post_fx_chain(
            &gl,
            post_process_renderer,
            shaders,
            global_post_fx.as_slice(),
            fbo_a,
            fbo_b,
            screen_rect,
            &global_vp,
            global_cam_pos,
            elapsed_time,
            delta_time,
        );

        // Step 4. Present the frame.
        present(
            &gl,
            post_process_renderer,
            present_shader,
            final_frame,
            *screen_width,
            *screen_height,
        );
    }
}

fn resolve_camera(world: &World, camera_id: CameraID) -> (Mat4, Vec2) {
    let camera = world.camera(camera_id).unwrap();
    (camera.get_view_projection(), camera.position)
}

/// Per-pass drawing. Binds `target`, restricts drawing to `rect`, honors the pass's clear
/// settings, then hands off to `BatchRenderer` for the actual sprite drawing.
///
/// `force_clear` overrides the pass' own `clear_color`/`clear_depth` to always fully clear
/// (color defaulting to transparent black if the pass didn't specify one). This must be `true`
/// when `target` is `fbo_c`, which is a scratch buffer reused sequentially across unrelated
/// passes within the same frame.
fn draw_pass_into(
    gl: &glow::Context,
    target: &Framebuffer,
    rect: PixelRect,
    pass: &RenderPass,
    force_clear: bool,
    batch_renderer: &mut BatchRenderer,
    world: &World,
    shaders: &HashMap<ShaderID, Shader>,
    view_projection: &Mat4,
    camera_position: Vec2,
    elapsed_time: f32,
    delta_time: f32,
) {
    target.bind();

    unsafe {
        gl.viewport(rect.x, rect.y, rect.width, rect.height);
        gl.scissor(rect.x, rect.y, rect.width, rect.height);
        gl.enable(glow::SCISSOR_TEST);

        if force_clear {
            let color = pass.clear_color.unwrap_or(Vec4::ZERO);
            gl.clear_color(color.x, color.y, color.z, color.w);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        } else {
            let mut clear_mask = 0u32;
            if let Some(color) = pass.clear_color {
                gl.clear_color(color.x, color.y, color.z, color.w);
                clear_mask |= glow::COLOR_BUFFER_BIT;
            }
            if pass.clear_depth {
                clear_mask |= glow::DEPTH_BUFFER_BIT;
            }
            if clear_mask != 0 {
                gl.clear(clear_mask);
            }
        }

        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LEQUAL);
    }

    let inputs = PassInputs {
        view_projection,
        camera_position,
        ordering_rule: pass.ordering_rule.as_ref(),
    };

    batch_renderer.render(&pass.items, world, shaders, &inputs, elapsed_time, delta_time);

    unsafe {
        gl.disable(glow::SCISSOR_TEST);
    }
}

/// Runs an enabled, priority-sorted post-fx layer list as a ping-pong chain between
/// `source` and `ping_pong`, resctricted to `rect`. Returns whichever framebuffer ends up
/// holding the final result, so the caller doesn't need to track ping-pong parity itself.
///
/// If there are zero enabled layers, `source` is returned unchanged.
fn run_post_fx_chain<'fb>(
    gl: &glow::Context,
    post_process_renderer: &PostProcessRenderer,
    shaders: &HashMap<ShaderID, Shader>,
    layers: &[PostProcessLayer],
    source: &'fb Framebuffer,
    ping_pong: &'fb Framebuffer,
    rect: PixelRect,
    view_projection: &Mat4,
    camera_position: Vec2,
    elapsed_time: f32,
    delta_time: f32,
) -> &'fb Framebuffer {
    let mut enabled: Vec<&PostProcessLayer> = layers.iter().filter(|l| l.enabled).collect();
    enabled.sort_by_key(|l| l.priority);

    if enabled.is_empty() {
        return source;
    }

    let mut read_from = source;
    let mut write_to = ping_pong;

    unsafe {
        gl.disable(glow::DEPTH_TEST);
    }

    for layer in enabled {
        let shader = match shaders.get(&layer.shader_id) {
            Some(s) => s,
            None => {
                eprintln!("PostProcess - unknown ShaderID for layer {}", layer.name);
                continue;
            }
        };

        write_to.bind();

        //Normalizes `rect` into the source framebuffer's own texture space.
        let source_rect = Vec4::new(
            rect.x as f32 / read_from.width() as f32,
            rect.y as f32 / read_from.height() as f32,
            rect.width as f32 / read_from.width() as f32,
            rect.height as f32 / read_from.height() as f32,
        );

        unsafe {
            gl.viewport(rect.x, rect.y, rect.width, rect.height);
            gl.scissor(rect.x, rect.y, rect.width, rect.height);
            gl.enable(glow::SCISSOR_TEST);

            shader.use_program();
            shader.set_int("uScene", 0);
            shader.set_vec4("uSourceRect", &source_rect);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(read_from.color_texture()));
        }

        if let Some(provider) = &layer.uniform_provider {
            let ctx = FrameContext {
                view_projection,
                camera_position,
                elapsed_time,
                delta_time,
            };
            provider.provide(shader, &ctx);
        }

        post_process_renderer.draw_fullscreen_quad();

        std::mem::swap(&mut read_from, &mut write_to);
    }

    unsafe {
        gl.disable(glow::SCISSOR_TEST);
    }

    read_from
}

fn present(
    gl: &glow::Context,
    post_process_renderer: &PostProcessRenderer,
    present_shader: &Shader,
    source: &Framebuffer,
    screen_width: u32,
    screen_height: u32,
) {
    unsafe {
        Framebuffer::bind_default(gl);
        gl.viewport(0, 0, screen_width as i32, screen_height as i32);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::DEPTH_TEST);

        present_shader.use_program();
        present_shader.set_int("uScene", 0);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(source.color_texture()));
    }

    post_process_renderer.draw_fullscreen_quad();
}
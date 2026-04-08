use std::collections::HashMap;
use glfw::{Action, Context as _, Glfw, GlfwReceiver, PWindow, WindowHint, WindowMode};
use glow::HasContext;
use std::rc::Rc;
use std::time::Instant;
use glam::{Mat4, Vec2};
use crate::engine::config::config;
use crate::engine::{Drawable, Entity, EntityID};
use crate::engine::Updatable;
use crate::engine::shader::{ShaderID, Shader};
use crate::engine::texture::{TextureManager, TileMode};
use crate::engine::textured_quad::TexturedQuad;
use crate::engine::input::Input;
use crate::engine::world::World;

pub struct Game {
    glfw: Glfw,
    window: PWindow,
    events: GlfwReceiver<(f64, glfw::WindowEvent)>,
    gl: Rc<glow::Context>,
    world: World,
    entities: HashMap<EntityID, Box<dyn Entity>>,

}

impl Game {
    pub fn new() -> Self {
        let config = config();

        let mut glfw = glfw::init(glfw::fail_on_errors).expect("Failed to initialize GLFW");

        glfw.window_hint(WindowHint::ContextVersion(3, 3));
        glfw.window_hint(WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));

        let (mut window, events) = glfw
            .create_window(config.window_size.0, config.window_size.1, &config.game_name, WindowMode::Windowed)
            .expect("Failed to create GLFW window");

        window.make_current();
        window.set_key_polling(true);
        window.set_framebuffer_size_polling(true);

        //Create GLOW context
        let gl = Rc::new( unsafe {
            glow::Context::from_loader_function(|s| {
                window
                    .get_proc_address(s)
                    .map_or(std::ptr::null(), |p| p as *const _)
            })
        });

        unsafe {
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.blend_equation(glow::FUNC_ADD);
        }

        Game {
            glfw, window, events,
            gl: gl.clone(),
            world: World::new(gl.clone()),
            entities: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        let config = config();
        let mut current_shader: Option<ShaderID> = None;
        let mut shader: Option<&Shader> = None;

        //set initial viewport
        unsafe {
            self.gl.viewport(0, 0, config.window_size.0 as i32, config.window_size.1 as i32);
        }

        let projection = Mat4::orthographic_rh_gl(
            0.0,
            config.logic_size.0 as f32,
            0.0,
            config.logic_size.1 as f32,
            -1.0,
            1.0,
        );

        let mut input = Input::new()
            .map_err(|e| eprintln!("{e}"))
            .unwrap();

        let mut last_frame = Instant::now();
        let mut delta_time = 0.0;
        let mut phys_timer = 0.0;

        //Main loop
        while !self.window.should_close() {
            input.clear_frame();
            self.glfw.poll_events();

            for (_, event) in glfw::flush_messages(&self.events) {
                match event {
                    glfw::WindowEvent::FramebufferSize(width, height) => unsafe {
                        self.gl.viewport(0, 0, width, height);
                    }

                    glfw::WindowEvent::Key(key, _, Action::Press, _) => {
                        input.keys_down.insert(key);
                        input.keys_pressed.insert(key);
                    }

                    glfw::WindowEvent::Key(key, _, Action::Release, _) => {
                        input.keys_down.remove(&key);
                        input.keys_released.insert(key);
                    }

                    _ => {}
                }
            }

            let mut i = 5;
            while phys_timer >= config.fixed_delta_time && i >= 0 {
                self.update_phase(&input, config.fixed_delta_time);
                phys_timer -= config.fixed_delta_time;
                i -= 1;
            }


            self.draw_phase(&projection);


            let now = Instant::now();
            let duration = now.duration_since(last_frame);

            delta_time = duration.as_secs_f32();
            phys_timer += delta_time;
            last_frame = now;
        }
    }

    fn update_phase(&mut self, input: &Input, delta_time: f32) {
        for entity in self.entities.values_mut() {
            if let Some(e) = entity.as_updatable_mut() {
                e.update(&input, &mut self.world, delta_time);
            }
        }
    }

    fn draw_phase(&mut self, projection: &Mat4) {
        unsafe {
            self.gl.clear_color(0.2, 0.3, 0.3, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }

        let mut drawables = Vec::new();
        for entity in self.entities.values() {
            if let Some(e) = entity.as_drawable() {
                drawables.push(e);
            }
        }

        drawables.sort_by_key(|d| d.shader());
        let mut current_shader = None;
        let mut shader = None;

        for d in drawables {
            if current_shader != Some(d.shader()) {
                current_shader = Some(d.shader());
                shader = Some(&self.world.shaders()[&current_shader.unwrap()]);
                shader.unwrap().use_program();
                shader.unwrap().set_mat4(
                    "uViewProjection",
                    &self.world.camera().get_view_projection(),
                );
                shader.unwrap().set_vec2(
                    "uCameraPos",
                    &self.world.camera().position
                );
            }

            if let Some(shader) = shader {
                d.draw(shader, &self.world);
            }
        }

        self.window.swap_buffers();
    }
}

impl Game {
    pub fn register_entity<T: Entity + 'static>(&mut self, id: EntityID, entity: T) {
        self.entities.insert(id, Box::new(entity));
    }

    pub fn add_shader(&mut self, vertex_source: &str, fragment_source: &str, name: &str) -> Result<ShaderID, String> {
        self.world.add_shader(self.gl.clone(), vertex_source, fragment_source, name)
    }

    pub fn shader_id(&self, name: &str) -> Option<ShaderID> {
        self.world.shader_id(name)
    }

    pub fn gl(&self) -> Rc<glow::Context> {
        self.gl.clone()
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}
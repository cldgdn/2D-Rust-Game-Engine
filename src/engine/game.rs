use std::collections::HashMap;
use glfw::{Action, Context as _, GamepadButton, Glfw, GlfwReceiver, JoystickId, PWindow, WindowHint, WindowMode};
use glow::HasContext;
use std::rc::Rc;
use std::time::Instant;
use glam::{Vec2};
use crate::engine::config::config;
use crate::engine::{BitMask, Entity, EntityID};
use crate::engine::collision::CollisionSystem;
use crate::engine::collision::raycast::RaycastHit;
use crate::engine::component::animation::AnimationManager;
use crate::engine::component::timer::Timer;
use crate::engine::component::transform::{Transform, TransformDisplayMode};
use crate::engine::component::update_gates::{ActiveFlag, PauseMask};
use crate::engine::rendering::shader::{ShaderID, Shader, ShaderDescriptor};
use crate::engine::input::Input;
use crate::engine::rendering::pass::RenderPass;
use crate::engine::rendering::post_process::PostProcessLayer;
use crate::engine::rendering::renderer::Renderer;
use crate::engine::world::{FixedIntention, Intention, PostFxIntention, RenderPassIntention, World};

/// Master structure of the engine. Handles the main loop.
pub struct Game {
    glfw: Glfw,
    window: PWindow,
    events: GlfwReceiver<(f64, glfw::WindowEvent)>,
    gl: Rc<glow::Context>,
    collision_system: CollisionSystem,
    raycast_hits: Vec<(EntityID, Vec<RaycastHit>)>,
    world: World,
    entities: HashMap<EntityID, Box<dyn Entity>>,
    renderer: Renderer,
}

impl Game {
    /// Initializes all engine-critical structures and prepares for running
    pub fn new() -> Self {
        let config = config();

        let mut glfw = glfw::init(glfw::fail_on_errors).expect("Failed to initialize GLFW");

        glfw.window_hint(WindowHint::ContextVersion(3, 3));
        glfw.window_hint(WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));

        let (mut window, events) = glfw
            .create_window(config.window_size.0, config.window_size.1, &config.game_name, WindowMode::Windowed)
            .expect("Game::new - Failed to create GLFW window");

        window.make_current();
        window.set_key_polling(true);
        window.set_mouse_button_polling(true);
        window.set_cursor_pos_polling(true);
        window.set_scroll_polling(true);
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

            // Enable depth testing so Z values in aPos.z drive occlusion.
            // LEQUAL allows objects at the same depth to overwrite correctly.
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
        }

        let renderer = Renderer::new(gl.clone(), config.window_size.0, config.window_size.1);

        let mut game = Game {
            glfw, window, events,
            gl: gl.clone(),
            collision_system: CollisionSystem::new(),
            raycast_hits: Vec::new(),
            world: World::new(gl.clone()),
            entities: HashMap::new(),
            renderer,
        };

        game.register_defaults();

        game
    }

    /// Starts the main loop and runs it until stopped.
    pub fn run(&mut self) {
        let config = config();

        //set initial viewport
        unsafe {
            self.gl.viewport(0, 0, config.window_size.0 as i32, config.window_size.1 as i32);
        }

        let mut input = Input::new()
            .map_err(|e| eprintln!("{e}"))
            .unwrap();

        let mut last_frame = Instant::now();
        let mut delta_time = 0.0;
        let mut total_elapsed_time = 0.0;
        let mut phys_timer = 0.0;
        let mut timer = 0.0;
        let mut frame_count = 0;

        //Main loop
        while !self.window.should_close() {
            // INPUT POLLING -----------------------------------------------------------------------

            input.clear_frame();
            self.glfw.poll_events();

            for (_, event) in glfw::flush_messages(&self.events) {
                match event {
                    glfw::WindowEvent::FramebufferSize(width, height) => {
                        self.renderer.resize(width as u32, height as u32);
                    }

                    glfw::WindowEvent::FramebufferSize(width, height) => unsafe {
                        self.gl.viewport(0, 0, width, height);
                        input.window_size = Vec2::new(width as f32, height as f32);
                    }

                    glfw::WindowEvent::Key(key, _, Action::Press, _) => {
                        input.keys_down.insert(key);
                        input.keys_pressed.insert(key);
                    }

                    glfw::WindowEvent::Key(key, _, Action::Release, _) => {
                        input.keys_down.remove(&key);
                        input.keys_released.insert(key);
                    }

                    glfw::WindowEvent::CursorPos(x, y) => {
                        input.mouse_pos = Vec2::new(x as f32, y as f32);
                    }

                    glfw::WindowEvent::MouseButton(button, Action::Press, _) => {
                        input.mouse_down.insert(button);
                        input.mouse_pressed.insert(button);
                    }

                    glfw::WindowEvent::MouseButton(button, Action::Release, _) => {
                        input.mouse_down.remove(&button);
                        input.mouse_released.insert(button);
                    }

                    _ => {}
                }
            }

            if let Some(state) = self.glfw.get_joystick(JoystickId::Joystick1).get_gamepad_state() {
                let buttons = [
                    GamepadButton::ButtonA,
                    GamepadButton::ButtonB,
                    GamepadButton::ButtonX,
                    GamepadButton::ButtonY,
                    GamepadButton::ButtonDpadUp,
                    GamepadButton::ButtonDpadDown,
                    GamepadButton::ButtonDpadRight,
                    GamepadButton::ButtonDpadLeft,
                    GamepadButton::ButtonStart,
                    GamepadButton::ButtonBack,
                    GamepadButton::ButtonRightBumper,
                    GamepadButton::ButtonLeftBumper,
                    GamepadButton::ButtonRightThumb,
                    GamepadButton::ButtonLeftThumb,
                ];

                for button in buttons {
                    let pressed = state.get_button_state(button) == Action::Press;

                    if pressed {
                        if !input.gamepad_down.contains(&button) {
                            input.gamepad_pressed.insert(button);
                        }
                        input.gamepad_down.insert(button);
                    } else {
                        if input.gamepad_down.remove(&button) {
                            input.gamepad_released.insert(button);
                        }
                    }
                }

                // All the deadzone/edge bookkeeping lives on
                // Input itself, so the loop just hands over the raw gamepad state.
                input.update_gamepad_axes(&state);
            } else {
                // No pad connected this frame - make sure axes don't stay stuck
                // on whatever they last read before disconnect.
                input.reset_axes();
            }
            
            // UPDATE PHASES
            self.update_phase(&input, delta_time);
            
            let mut i = 5;
            while phys_timer >= config.fixed_delta_time && i >= 0 {
                self.fixed_update_phase(config.fixed_delta_time);
                phys_timer -= config.fixed_delta_time;
                i -= 1;
                
                if i <= 0 {
                    phys_timer = 0.0;
                }
            }

            // DRAW PHASE --------------------------------------------------------------------------
            self.draw_phase(delta_time, phys_timer);

            // LOOP MANAGEMENT ---------------------------------------------------------------------
            let now = Instant::now();
            let duration = now.duration_since(last_frame);

            delta_time = duration.as_secs_f32();
            last_frame = now;

            phys_timer += delta_time;
            timer += delta_time;
            frame_count += 1;
            total_elapsed_time += delta_time;

            if timer >= 1.0 {
                //println!("FPS: {}", frame_count as f32 / timer);
                timer = 0.0;
                frame_count = 0;
            }
        }
    }

    fn update_phase(&mut self, input: &Input, delta_time: f32) {
        // Done here so that entities see updated state on the same frame as the update happens
        self.world.tick_all(delta_time);

        for entity in self.entities.values_mut() {
            let id = entity.id();
            if let Some(e) = entity.as_updatable_mut() {
                if self.world.check_update_gating(id) {
                    e.update(&input, &mut self.world, delta_time);
                }
            }
        }

        self.flush_intentions();
        
        self.world.dispatch_events(&mut self.entities);
    }
    
    fn fixed_update_phase(&mut self, delta_time: f32) {
        self.world.fixed_intention_queue.clear();
        
        let transforms = self.world.get_all_components_mut::<Transform>();
        
        for (_id, t) in transforms {
            t.snapshot();
        }

        for entity in self.entities.values_mut() {
            let id = entity.id();
            if let Some(e) = entity.as_updatable_mut() {
                if self.world.check_update_gating(id) {
                    e.fixed_update(&mut self.world, delta_time);
                }
            }
        }
        
        let (collisions, _) = self
            .collision_system
            .run_collisions(&self.world);

        let collisions = self
            .collision_system
            .resolve_collisions(collisions, &mut self.world);
        
        for collision in collisions {
            let a_id = &collision.a;
            
            let entity = self.entities
                .get_mut(&a_id.entity).unwrap()
                .as_collides_mut().unwrap();
            
            entity.react_to_collision(collision, &mut self.world);
        }

        for intention in self.world.fixed_intention_queue.iter() {
            match intention {
                FixedIntention::RaycastRequest(request) => {
                    let hits = self.collision_system.get_hits(&self.world, &request);
                    self.raycast_hits.push((request.source_id, hits));
                },
            }
        }
        
        loop {
            if let Some((entity, hits)) = self.raycast_hits.pop() {
                let entity = self.entities
                    .get_mut(&entity).unwrap()
                    .as_collides_mut().unwrap();
                
                entity.react_to_raycast(hits, &mut self.world);
            }
            else {
                break;
            }
        }
        
        self.collision_system.post_raycast_clear();

        self.world.update_cameras();
    }
    
    fn draw_phase(&mut self, delta_time: f32, elapsed_time: f32) {
        unsafe {
            self.gl.clear_color(0.2, 0.3, 0.3, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }

        self.renderer.render_frame(
            &self.entities,
            &self.world,
            elapsed_time,
            delta_time,
        );

        unsafe { self.gl.clear(glow::DEPTH_BUFFER_BIT); }

        if config().debug_mode {
            self.collision_system.debug_draw(&mut self.world, &self.renderer.passes, &self.entities);
        }

        self.window.swap_buffers();
    }
    
    fn flush_intentions(&mut self) {
        loop {
            if let Some(intention) = self.world.intention_queue.pop() {
                match intention {
                    Intention::Spawn(entity) => {
                        if let Some(entity) = entity.as_collides() {
                            entity.submit_colliders(&mut self.collision_system, &mut self.world);
                        }
                        self.entities.insert(entity.id(), entity);
                    },
                    Intention::Destroy(id) => {
                        // If the entity was already destroyed (remove returns None), don't bother with cleanup
                        // This can happen if two entities issue a Destroy intention for the same
                        // entity on the same frame.
                        if let Some(_) = self.entities.remove(&id) {
                            self.collision_system.remove_static_colliders(id, &mut self.world);
                            self.world.remove_components(id);
                            self.world.remove_name(id);
                            self.world.event_bus.unsubscribe_entity(id);
                        }
                    },
                    Intention::SetPauseBits(bitmask) => {
                        self.world.pause_mask = self.world.pause_mask | bitmask;
                    },
                    Intention::UnsetPauseBits(bitmask) => {
                        self.world.pause_mask = self.world.pause_mask & !bitmask;
                    },
                    Intention::RenderPass(pass_intention) => {
                        match pass_intention {
                            RenderPassIntention::SetEnabled(name, enabled) => {
                                if let Some(pass) = self.renderer.pass_mut(name.as_str()) {
                                    pass.set_enabled(enabled);
                                }
                            },
                            RenderPassIntention::ToggleEnabled(name) => {
                                if let Some(pass) = self.renderer.pass_mut(name.as_str()) {
                                    pass.set_enabled(!pass.enabled);
                                }
                            }
                            RenderPassIntention::PostFx(name, post_fx_intention) => {
                                if let Some(pass) = self.renderer.pass_mut(name.as_str()) {
                                    match post_fx_intention {
                                        PostFxIntention::SetEnabled(name, enabled) => {
                                            if let Some(layer) = pass.local_post_fx.iter_mut().find(|fx| fx.name == name) {
                                                layer.set_enabled(enabled);
                                            }
                                        },
                                        PostFxIntention::ToggleEnabled(name) => {
                                            if let Some(layer) = pass.local_post_fx.iter_mut().find(|fx| fx.name == name) {
                                                layer.set_enabled(!layer.enabled);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Intention::PostFx(post_fx_intention) => {
                        match post_fx_intention {
                            PostFxIntention::SetEnabled(name, enabled) => {
                                if let Some(layer) = self.renderer.global_post_fx_mut(name.as_str()) {
                                    layer.set_enabled(enabled);
                                }
                            },
                            PostFxIntention::ToggleEnabled(name) => {
                                if let Some(layer) = self.renderer.global_post_fx_mut(name.as_str()) {
                                    layer.set_enabled(!layer.enabled);
                                }
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }
    }
}

impl Game {
    /// Adds a new Entity to the game, registering any colliders if present
    /// by calling Collides::submit_colliders if the Entity implements Entity::as_collides
    pub fn register_entity<T: Entity + 'static>(&mut self, id: EntityID, entity: T) {
        if let Some(e) = entity.as_collides() {
            e.submit_colliders(&mut self.collision_system, &mut self.world);
        }
        
        self.entities.insert(id, Box::new(entity));
        
    }

    /// DEPRECATED
    /// 
    /// Adds a shader to the game by compiling the two shader sources.
    pub fn add_shader(&mut self, vertex_source: &str, fragment_source: &str, name: &str) -> Result<ShaderID, String> {
        self.world.add_shader(self.gl.clone(), vertex_source, fragment_source, name)
    }
    
    /// Adds a new shader to the game, and returns the ID it is assigned.
    pub fn register_shader(&mut self, name: &str, shader: Shader, descriptor: ShaderDescriptor) -> ShaderID {
        let id = self.world.register_shader(shader, name);

        self.renderer
            .batch_renderer_mut()
            .register_descriptor(id, descriptor);

        id
    }

    /// Returns the id of a shader if a shader by the same name as what is passed has already been
    /// registered.
    /// 
    /// "default" is always available and implements the glsl pointed at by the relative config fields.
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
    
    pub fn collision_system(&self) -> &CollisionSystem { 
        &self.collision_system
    }
    
    pub fn collision_system_mut(&mut self) -> &mut CollisionSystem {
        &mut self.collision_system
    }

    pub fn add_render_pass(&mut self, pass: RenderPass) {
        self.renderer.add_pass(pass);
    }

    pub fn add_global_post_fx(&mut self, layer: PostProcessLayer) {
        self.renderer.add_global_post_fx(layer);
    }

    fn register_defaults(&mut self) {
        match self.add_shader(
            config().shader_default_vertex.as_str(),
            config().shader_default_fragment.as_str(),
            "default"
        ) {
            Ok(id) => {
                let descriptor = ShaderDescriptor::default();

                self.renderer
                    .batch_renderer_mut()
                    .register_descriptor(id, descriptor);
            },
            Err(e) => {
                panic!("Failed to compile default shader: {}", e);
            },
        };

        self.world.register_tickable_gated::<AnimationManager>();
        self.world.register_tickable_gated::<Timer>();
    }
}
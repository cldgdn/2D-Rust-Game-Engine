use glam::{Mat4, Vec2, Vec3, Vec4};
use crate::engine::{Drawable, Entity, EntityID};
use crate::engine::collision::{Collides, Collision, CollisionEvent, CollisionSystem};
use crate::engine::collision::collider::{Collider, ColliderType};
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::Updatable;
use crate::engine::input::Input;
use crate::engine::shader::{Shader, ShaderID};
use crate::engine::visual::animation::{Animation, AnimationManager, AtlasOption, FrameInfo};
use crate::engine::visual::texture::TileMode;
use crate::engine::visual::textured_quad::TexturedQuad;
use crate::engine::visual::UVData;
use crate::engine::world::World;

pub struct Player {
    id: EntityID,
    animation_manager: AnimationManager,
    current_shader: ShaderID,
    pub z_index: i32,
    pub is_active: bool,
    pub velocity: Vec2,
}

impl Player {
    pub fn new(world: &mut World) -> Player {
        let id = world.get_next_entity_id();
        let mut transform = Transform::new();

        transform.set_scale(Vec2::new(2.0, 2.0));

        world.add_transform(id, transform);

        let animation_manager = AnimationManager::new();

        let current_shader = world
            .shader_id("camera")
            .expect("Failed to get shader id");

        let mut p = Player {
            id,
            animation_manager,
            current_shader,
            is_active: true,
            z_index: 0,
            velocity: Vec2::new(0.0, 0.0),
        };

        p.setup_animations(world);

        p
    }

    fn setup_animations(&mut self, world: &mut World) {
        let gl = world.gl().clone();
        let tm = world.texture_manager();
        let atlas = tm.load(
            TileMode::Stretch,
            gl.clone(),
            "resources/textures/player.png",
        ).expect("Failed to load texture atlas at resources/textures/player.png");

        let frame_locs: Vec<FrameInfo> = Vec::from([
            FrameInfo {
                texture_offset: Vec2::new(3.0, 0.0),
                repeat: 1,
            },
            FrameInfo {
                texture_offset: Vec2::new(5.0, 0.0),
                repeat: 1,
            },
        ]);

        let walk_right = Animation::new(
            AtlasOption::Direct(atlas.clone()),
            16, 32,
            2.0,
            true,
            frame_locs,
            world,
        );

        self.animation_manager.add_animation("walk_right", walk_right);

        let frame_locs: Vec<FrameInfo> = Vec::from([
            FrameInfo {
                texture_offset: Vec2::new(2.0, 0.0),
                repeat: 1,
            },
            FrameInfo {
                texture_offset: Vec2::new(4.0, 0.0),
                repeat: 1,
            },
        ]);

        let walk_left = Animation::new(
            AtlasOption::Direct(atlas.clone()),
            16, 32,
            2.0,
            true,
            frame_locs,
            world,
        );

        self.animation_manager.add_animation("walk_left", walk_left);
        self.animation_manager.set_animation("walk_right");
    }
}

impl Entity for Player {
    fn id(&self) -> EntityID {
        self.id
    }

    fn as_updatable_mut(&mut self) -> Option<&mut dyn Updatable> {
        Some(self)
    }

    fn as_drawable(&self) -> Option<&dyn Drawable> {
        Some(self)
    }

    fn as_collides(&self) -> Option<&dyn Collides> {
        Some(self)
    }

    fn as_collides_mut(&mut self) -> Option<&mut dyn Collides> {
        Some(self)
    }
}

impl Updatable for Player {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        if !self.is_active {
            return;
        }

        if self.animation_manager.finished {
            println!("active animation finished");
        }

        let transform = world.get_transform(self.id).unwrap();

        let mut direction = Vec2::new(0.0, 0.0);

        if input.is_action_down("walk_right") {
            direction.x += 1.0;
            self.animation_manager.set_animation("walk_right");
        }

        if input.is_action_down("walk_left") {
            direction.x -= 1.0;
            self.animation_manager.set_animation("walk_left");
        }

        if input.is_action_down("walk_up") {
            direction.y += 1.0;
        }

        if input.is_action_down("walk_down") {
            direction.y -= 1.0;
        }

        if input.is_action_pressed("interact") {
            let pos = transform.position_2d();
            println!("{pos}");

            if self.animation_manager.is_playing() {
                self.animation_manager.stop()
            } else {
                self.animation_manager.resume();
            }
        }

        self.velocity = direction * 30.0;

        self.animation_manager.update(delta_time);
    }

    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        if !self.is_active {
            return;
        }

        let transform = world.get_mut_transform(self.id).unwrap();
        transform.translate(self.velocity * delta_time);
    }
}

impl Drawable for Player {
    fn draw(&self, shader: &Shader, world: &World) {
        let transform = if let Some(t) = world.get_transform(self.id) {
            t
        } else {
            return;
        };

        let position = transform.position();
        let scale = transform.scale();
        let model = Mat4::from_translation(position.clone());
        let model = model.mul_diagonal_scale(Vec4::new(scale.x, scale.y, 1.0, 1.0));

        shader.set_mat4("uModel", &model);

        self.animation_manager.draw(shader);
    }

    fn shader(&self) -> ShaderID {
        self.current_shader
    }

    fn z_index(&self) -> i32 {
        self.z_index
    }
}

impl Collides for Player {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem, world: &mut World) {
        let main = Collider::new(
            self.id,
            "main",
            1,
            1,
            ColliderType::Trigger,
            false,
            true,
            32,
            64,
            Vec2::ZERO,
            None,
            world.gl(),
        );

        let solid = Collider::new(
            self.id,
            "solid",
            1,
            1,
            ColliderType::Solid,
            false,
            true,
            32,
            64,
            Vec2::ZERO,
            Some(Vec4::new(1.0, 0.0, 0.0, 1.0)),
            world.gl(),
        );

        collision_system.add_collider(self.id, main, world);
        collision_system.add_collider(self.id, solid, world);
    }

    fn react_to_collision(&mut self, collision: CollisionEvent, world: &mut World) {
        ()
    }
}
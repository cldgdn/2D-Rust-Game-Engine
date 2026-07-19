use glam::{Mat4, Vec2, Vec3, Vec4};
use crate::engine::{Drawable, Entity, EntityID};
use crate::engine::collision::{Collides, Collision, CollisionEvent, CollisionSystem};
use crate::engine::collision::collider::{Collider, ColliderType};
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::Updatable;
use crate::engine::input::Input;
use crate::engine::rendering::RenderItem;
use crate::engine::rendering::shader::{Shader, ShaderID};
use crate::engine::visual::animation::{Animation, AnimationManager, AtlasOption, FrameInfo};
use crate::engine::visual::texture::TileMode;
use crate::engine::visual::textured_quad::TexturedQuad;
use crate::engine::visual::{TextureProvider, UVData};
use crate::engine::world::{Intention, World};
use crate::platform_builder::events::SomeEvent;
use crate::platform_builder::spawnable::Projectile;

pub struct Player {
    id: EntityID,
    current_shader: ShaderID,
    pub z_index: i32,
    pub is_active: bool,
    pub velocity: Vec2,
}

impl Player {
    pub fn new(world: &mut World) -> Player {
        let id = world.get_next_entity_id();
        let mut transform = Transform::new();

        transform.set_size(Vec2::new(1.0, 2.0));

        world.add_component(id, transform);

        let current_shader = world
            .shader_id("default")
            .expect("Failed to get shader id");

        let mut p = Player {
            id,
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
        let tm = world.texture_manager_mut();
        let atlas = tm.load(
            gl.clone(),
            "resources/textures/player.png",
        ).expect("Failed to load texture atlas at resources/textures/player.png");
        let mut animation_manager = AnimationManager::new();

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

        animation_manager.add_animation("walk_right", walk_right);

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

        animation_manager.add_animation("walk_left", walk_left);
        animation_manager.set_animation("walk_right");

        world.add_component(self.id, animation_manager);
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

        let animation_manager = world.get_component_mut::<AnimationManager>(self.id).unwrap();

        let mut direction = Vec2::new(0.0, 0.0);

        let action = input.is_action_down("walk_right");
        if action.active {
            direction.x += 1.0 * action.intensity;
            animation_manager.set_animation("walk_right");
        }

        let action = input.is_action_down("walk_left");
        if action.active {
            direction.x -= 1.0 * action.intensity;
            animation_manager.set_animation("walk_left");
        }

        let action = input.is_action_down("walk_up");
        if action.active {
            direction.y += 1.0 * action.intensity;
        }

        let action = input.is_action_down("walk_down");
        if action.active {
            direction.y -= 1.0 * action.intensity;
        }

        let action = input.is_action_pressed("interact").active;
        if action {
            if animation_manager.is_playing() {
                animation_manager.stop()
            } else {
                animation_manager.resume();
            }
        }

        self.velocity = direction * 3.0;

        animation_manager.update(delta_time);

        if action {
            world.event_bus.emit(SomeEvent);
        }

        let mut aim_direction = Vec2::ZERO;
        let action = input.is_action_down("up");
        if action.active {
            aim_direction.y += 1.0 * action.intensity;
        }

        let action = input.is_action_down("down");
        if action.active {
            aim_direction.y -= 1.0 * action.intensity;
        }

        let action = input.is_action_down("right");
        if action.active {
            aim_direction.x += 1.0 * action.intensity;
        }

        let action = input.is_action_down("left");
        if action.active {
            aim_direction.x -= 1.0 * action.intensity;
            println!("Aiming left with intensity {}", action.intensity);
        }

        let action = input.is_action_pressed("shoot").active;
        if action {
            let pos = world.get_component::<Transform>(self.id)
                .unwrap()
                .position().clone();
            let mut bullet = Projectile::new(world, pos, aim_direction, 1.0);
            bullet.z_index = 10;
            world.intention_queue.push(Intention::Spawn(Box::new(bullet)));
        }
    }

    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        if !self.is_active {
            return;
        }

        let transform = world.get_component_mut::<Transform>(self.id).unwrap();
        transform.translate(self.velocity * delta_time);
    }
}

impl Drawable for Player {
    fn entity_id(&self) -> EntityID {
        self.id
    }

    fn shader_id(&self) -> ShaderID {
        self.current_shader
    }

    fn z_index(&self) -> i32 {
        self.z_index
    }

    fn get_render_item(&self, world: &World) -> Option<RenderItem> {
        let _transform = world.get_component::<Transform>(self.id)?;
        let animation_manager = world.get_component::<AnimationManager>(self.id)?;

        let tex_data = animation_manager.get_texture_render_data()?;

        Some(
            RenderItem {
                entity_id: self.id,
                shader_id: self.current_shader,
                texture_id: tex_data.texture_id,
                tex_offset: tex_data.tex_offset,
                region_size: tex_data.region_size,
                max_uvs: tex_data.max_uvs,
                z_index: self.z_index,
                color_tint: Vec4::ONE,
                extended_data: None
            }
        )
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
            1.0,
            2.0,
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
            1.0,
            2.0,
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
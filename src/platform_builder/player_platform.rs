use std::any::TypeId;
use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::engine::component::{Multi, SlotID};
use crate::engine::component::transform::Transform;
use crate::engine::{mask_bit, Drawable, Entity, EntityID, Updatable};
use crate::engine::collision::{Collides, CollisionEvent, CollisionSystem, CollisionType};
use crate::engine::collision::math_utilities::CollisionNormalExtension;
use crate::engine::collision::shape::Shape;
use crate::engine::component::animation::{Animation, AnimationManager, AtlasOption, FrameInfo};
use crate::engine::component::collider::Collider;
use crate::engine::component::timer::Timer;
use crate::engine::input::Input;
use crate::engine::rendering::{RenderItem, TextureProvider};
use crate::engine::rendering::pass::PassMask;
use crate::engine::rendering::shader::ShaderID;
use crate::engine::world::World;

#[derive(Deserialize, Serialize, Debug)]
pub struct PlayerPlatformInit {
    pub starting_position: (f32, f32),
    pub active: bool,
}

pub struct PlayerPlatform {
    id: EntityID,
    current_shader: ShaderID,
    z_index: i32,
    velocity: Vec2,
    is_on_ground: bool,
    jump: bool,
}

impl PlayerPlatform {
    const C_SOLID: SlotID = 1;
    const JUMP_SPEED: f32 = 2.0;
    const TOP_SPEED: f32 = 3.0;
    const ACCELERATION: f32 = 1.0;

    pub fn new(id: EntityID, init: PlayerPlatformInit, world: &mut World) -> Self {
        let mut transform = Transform::new();

        transform.set_size(Vec2::new(1.0, 2.0));
        transform.set_position(Vec2::from(init.starting_position));

        transform.snapshot();

        let timer = Timer::new(1.0);

        world.add_component(id, transform);
        world.add_component(id, timer);

        let current_shader = world
            .shader_id("default")
            .expect("Failed to get default shader id");

        let mut p = PlayerPlatform {
            id,
            current_shader,
            z_index: 0,
            velocity: Vec2::ZERO,
            is_on_ground: false,
            jump: false,
        };

        p.setup_animations(world);

        p
    }

    pub fn from_init(id: EntityID, init: Value, world: &mut World) -> Box<dyn Entity> {
        let params: PlayerPlatformInit = serde_json::from_value(init).expect("Failed to parse PlayerPlatformInit");
        Box::new(PlayerPlatform::new(id, params, world))
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

impl Entity for PlayerPlatform {
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

impl Updatable for PlayerPlatform {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        let animation_manager = world.get_component_mut::<AnimationManager>(self.id).unwrap();
        let mut direction = Vec2::ZERO;

        let action = input.is_action_down("right");
        if action.active {
            direction.x += 1.0 * action.intensity;
            animation_manager.set_animation("walk_right");
        }

        let action = input.is_action_down("left");
        if action.active {
            direction.x -= 1.0 * action.intensity;
            animation_manager.set_animation("walk_left");
        }

        let action = input.is_action_pressed("jump").active;
        if action && self.is_on_ground {
            self.jump = true;
        }

        self.velocity = direction;
    }

    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        let comps  = world.get_many_components_mut(&[
            (self.id, TypeId::of::<Transform>()),
            (self.id, TypeId::of::<Timer>()),
        ]);

        let mut comps = comps.into_iter();
        let transform: &mut Transform = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let timer: &mut Timer = comps.next().unwrap().unwrap().downcast_mut().unwrap();

        transform.translate(self.velocity * delta_time);

        if self.jump {
            self.jump = false;
            timer.start();
        }

        if timer.is_running() {
            transform.translate(Vec2::new(0.0, Self::JUMP_SPEED * delta_time));
        }

        transform.translate(Vec2::new(0.0, -1.0 * delta_time));

        self.is_on_ground = false;
    }
}

impl Drawable for PlayerPlatform {
    fn entity_id(&self) -> EntityID {
        self.id
    }

    fn get_render_item(&self, world: &World) -> Option<RenderItem> {
        let animation_manager: &AnimationManager = world.get_component(self.id)?;


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
                extended_data: None,
                local_transform: None
            }
        )
    }

    fn pass_mask(&self) -> PassMask {
        mask_bit(1)
    }
}

impl Collides for PlayerPlatform {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem, world: &mut World) {
        let transform = world.get_component::<Transform>(self.id).unwrap();

        let mut shapes = Vec::new();
        shapes.push(Shape::Circle {
            radius: 0.5,
            rotation_offset: 0.0,
            offset: Vec2::new(0.0, -0.5)
        });
        shapes.push(Shape::Triangle {
            vertices: [
                Vec2::new(-0.5, 0.0),
                Vec2::new(0.5, 0.0),
                Vec2::new(0.0, 1.0)
            ],
            rotation_offset: 0.0,
            offset: Vec2::ZERO
        });
        shapes.push(Shape::Rectangle {
            half_extents: Vec2::new(
                transform.scaled_size().x / 2.0,
                transform.scaled_size().y / 8.0
            ),
            rotation_offset: 0.0,
            offset: Vec2::new(0.0, -transform.scaled_size().y / 8.0)
        });

        let solid = Collider::new(self.id, Self::C_SOLID, shapes, world.gl())
            .with_layers(mask_bit(1), mask_bit(1))
            .with_collision_space(mask_bit(0))
            .with_debug_color(Vec4::new(1.0, 0.0, 0.0, 1.0));

        let mut colliders: Multi<Collider> = Multi::new();
        colliders.insert(solid.id, solid);

        world.add_component(self.id, colliders);
    }

    fn react_to_collision(&mut self, collision: CollisionEvent, world: &mut World) {
        match collision.collision_type {
            CollisionType::Solid(_) => {
                let t: &Transform = world.get_component(self.id).unwrap();
                if collision.normal.is_floor(t.local_up()) {
                    self.is_on_ground = true;
                }
            },
            _ => ()
        }
    }
}
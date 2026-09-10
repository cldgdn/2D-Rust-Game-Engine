use crate::engine::collision::math_utilities::CollisionNormalExtension;
use crate::engine::collision::shape::Shape;
use crate::engine::collision::{Collides, CollisionEvent, CollisionSystem, CollisionType};
use crate::engine::component::animation::{Animation, AnimationManager, AtlasOption, FrameInfo};
use crate::engine::component::collider::Collider;
use crate::engine::component::transform::{Mirror, Transform};
use crate::engine::component::{Multi, SlotID};
use crate::engine::input::Input;
use crate::engine::rendering::pass::PassMask;
use crate::engine::rendering::shader::ShaderID;
use crate::engine::rendering::{RenderItem, TextureProvider};
use crate::engine::world::World;
use crate::engine::{mask_bit, Drawable, Entity, EntityID, Updatable};
use crate::platform_builder::physics::PlatformerPhysics;
use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::TypeId;
use crate::engine::component::tween::{ease, TimeMode, Tween};

#[derive(Deserialize, Serialize, Debug)]
pub struct PlayerPlatformInit {
    pub starting_position: (f32, f32),
    pub active: bool,
}

pub struct PlayerPlatform {
    id: EntityID,
    current_shader: ShaderID,
    z_index: i32,
    direction: Vec2,
    is_on_ground: bool,
    is_against_wall: bool,
    jump: bool,
    dashing: bool,
}

impl PlayerPlatform {
    const C_SOLID: SlotID = 1;
    const JUMP_SPEED: f32 = 5.0;
    const TOP_SPEED: f32 = 7.0;
    const GROUND_ACCELERATION: f32 = 5.5;
    const AIR_ACCELERATION: f32 = 4.5;
    const DASH_DISTANCE: f32 = 5.0;

    pub fn new(id: EntityID, init: PlayerPlatformInit, world: &mut World) -> Self {
        let mut transform = Transform::new();

        transform.set_size(Vec2::new(1.0, 2.0));
        transform.set_position(Vec2::from(init.starting_position));

        transform.snapshot();

        let physics = PlatformerPhysics::new()
            .with_gravity(-2.5)
            .with_terminal_velocity(6.0);

        let dash_tween: Tween<Vec2> = Tween::new(
            Vec2::ZERO,
            Vec2::ZERO,
            0.15,
            ease::sine_in,
            TimeMode::Fixed,
        );

        world.add_component(id, transform);
        world.add_component(id, physics);
        world.add_component(id, dash_tween);

        let current_shader = world
            .shader_id("default")
            .expect("Failed to get default shader id");

        let mut p = PlayerPlatform {
            id,
            current_shader,
            z_index: 0,
            direction: Vec2::ZERO,
            is_on_ground: false,
            is_against_wall: false,
            jump: false,
            dashing: false,
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
            "resources/textures/mario_small.png",
        ).expect("Failed to load texture atlas at resources/textures/player.png");

        let mut animation_manager = AnimationManager::new();

        let idle = {
            let frame_locs: Vec<FrameInfo> = Vec::from([
                FrameInfo {
                    texture_offset: Vec2::new(0.0, 0.0),
                    repeat: 1
                }
            ]);

            Animation::new(
                AtlasOption::Direct(atlas.clone()),
                16, 32,
                1.0,
                true,
                frame_locs,
                world,
            )
        };

        let walk = {
            let frame_locs: Vec<FrameInfo> = Vec::from([
                FrameInfo {
                    texture_offset: Vec2::new(3.0, 0.0),
                    repeat: 1,
                },
                FrameInfo {
                    texture_offset: Vec2::new(0.0, 0.0),
                    repeat: 1,
                },
            ]);

            Animation::new(
                AtlasOption::Direct(atlas.clone()),
                16, 32,
                10.0,
                true,
                frame_locs,
                world,
            )
        };

        let jump = {
            let frame_locs: Vec<FrameInfo> = Vec::from([
                FrameInfo {
                    texture_offset: Vec2::new(4.0, 0.0),
                    repeat: 1
                }
            ]);

            Animation::new(
                AtlasOption::Direct(atlas.clone()),
                16, 32,
                1.0,
                true,
                frame_locs,
                world,
            )
        };

        let spin = {
            let frame_locs: Vec<FrameInfo> = Vec::from([
                FrameInfo {
                    texture_offset: Vec2::new(1.0, 0.0),
                    repeat: 1
                },
                FrameInfo {
                    texture_offset: Vec2::new(2.0, 0.0),
                    repeat: 1
                },
                FrameInfo {
                    texture_offset: Vec2::new(9.0, 0.0),
                    repeat: 1
                },
                FrameInfo {
                    texture_offset: Vec2::new(0.0, 0.0),
                    repeat: 1
                }
            ]);

            Animation::new(
                AtlasOption::Direct(atlas.clone()),
                16, 32,
                10.0,
                true,
                frame_locs,
                world,
            )
        };

        let fall = {
            let frame_locs: Vec<FrameInfo> = Vec::from([
                FrameInfo {
                    texture_offset: Vec2::new(5.0, 0.0),
                    repeat: 1
                }
            ]);

            Animation::new(
                AtlasOption::Direct(atlas.clone()),
                16, 32,
                1.0,
                true,
                frame_locs,
                world,
            )
        };

        animation_manager.add_animation("idle", idle);
        animation_manager.add_animation("walk", walk);
        animation_manager.add_animation("jump", jump);
        animation_manager.add_animation("fall", fall);
        animation_manager.add_animation("spin", spin);

        animation_manager.set_animation("idle");

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
        let phys = world.remove_component::<PlatformerPhysics>(self.id).unwrap();
        let comps = world.get_many_components_mut(&[
            (self.id, TypeId::of::<Transform>()),
            (self.id, TypeId::of::<AnimationManager>()),
            (self.id, TypeId::of::<Tween<Vec2>>()),
        ]);

        let mut comps = comps.into_iter();
        let transform: &mut Transform = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let animation_manager: &mut AnimationManager = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let dash_tween: &mut Tween<Vec2> = comps.next().unwrap().unwrap().downcast_mut().unwrap();

        let mut direction = Vec2::ZERO;

        let right = input.is_action_down("right");
        let left = input.is_action_down("left");
        let jump = input.is_action_pressed("jump").active;
        let dash = input.is_action_pressed("dash").active;

        if right.active {
            direction.x += 1.0 * right.intensity;
            transform.set_mirror(Mirror::Visual(false, false));
        }
        if left.active {
            direction.x -= 1.0 * left.intensity;
            transform.set_mirror(Mirror::Visual(true, false));
        }

        if dash && !self.dashing {
            self.dashing = true;
            let start_pos = transform.position().clone();
            let direction = direction.normalize_or_zero();
            let end_pos = start_pos + (direction * Self::DASH_DISTANCE);

            dash_tween.retarget(start_pos, end_pos);
        }

        if self.is_on_ground && !self.dashing {
            if right.active {
                animation_manager.set_animation("walk");
            }
            else if left.active {
                animation_manager.set_animation("walk");
            }
            else {
                animation_manager.set_animation("idle");
            }

            if jump {
                self.jump = true;
                animation_manager.set_animation("jump");
            }
        }
        else if phys.velocity.y < 0.0 {
            animation_manager.set_animation("fall");
        }

        if self.dashing {
            world.set_tickable_fixed_active::<Tween<Vec2>>(self.id, true);
        }

        self.direction = direction;
        world.add_component(self.id, phys);
    }

    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        let dash_tween: &Tween<Vec2> = world.get_component(self.id).unwrap();

        let use_gravity = if dash_tween.is_finished() {
            self.dashing = false;
            true
        } else {
            false
        };

        let phys: &mut PlatformerPhysics = world.get_component_mut(self.id).unwrap();

        phys.use_gravity = use_gravity;

        if self.dashing {
            let comps = world.get_many_components_mut(&[
                (self.id, TypeId::of::<Transform>()),
                (self.id, TypeId::of::<Tween<Vec2>>()),
            ]);
            let mut comps = comps.into_iter();

            let transform: &mut Transform = comps.next().unwrap().unwrap().downcast_mut().unwrap();
            let dash_tween: &mut Tween<Vec2> = comps.next().unwrap().unwrap().downcast_mut().unwrap();

            transform.translate(dash_tween.get_motion());
        } else {
            if self.is_on_ground {
                phys.velocity.y = 0.0;
            }

            if self.is_against_wall {
                phys.velocity.x = 0.0;
            }

            let accel = if self.is_on_ground {
                Self::GROUND_ACCELERATION
            } else {
                Self::AIR_ACCELERATION
            };

            if self.direction.x != 0.0 {
                phys.accelerate(self.direction.x, accel, Self::TOP_SPEED, delta_time);
            } else {
                phys.decelerate(accel * 3.0, delta_time);
            }

            if self.jump {
                self.jump = false;
                phys.apply_impulse(Vec2::new(0.0, Self::JUMP_SPEED));
            }
        }

        self.is_on_ground = false;
        self.is_against_wall = false;
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
        shapes.push(Shape::Rectangle {
            half_extents: Vec2::new(
                transform.scaled_size().x / 3.0,
                transform.scaled_size().y / 4.0
            ),
            rotation_offset: 0.0,
            offset: Vec2::new(0.0, -transform.scaled_size().y / 4.0)
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
                if collision.normal.is_wall(t.local_up()) {
                    self.is_against_wall = true;
                }
            },
            _ => ()
        }
    }
}
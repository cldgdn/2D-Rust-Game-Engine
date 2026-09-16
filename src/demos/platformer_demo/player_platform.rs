use crate::demos::platformer_demo::components::physics::{PhysicsStats, PlatformerPhysics};
use crate::demos::platformer_demo::events::PlayerLandingEvent;
use crate::engine::collision::math_utilities::CollisionNormalExtension;
use crate::engine::collision::shape::Shape;
use crate::engine::collision::{Collides, CollisionEvent, CollisionSystem, CollisionType};
use crate::engine::component::animation::{Animation, AnimationManager, AtlasOption, FrameInfo};
use crate::engine::component::collider::{Collider, CollisionBehavior};
use crate::engine::component::transform::{Mirror, Transform};
use crate::engine::component::{Multi, SlotID};
use crate::engine::input::Input;
use crate::engine::rendering::pass::PassMask;
use crate::engine::rendering::shader::ShaderID;
use crate::engine::rendering::{RenderItem, TextureProvider};
use crate::engine::world::World;
use crate::engine::{mask_bit, Drawable, Entity, EntityID, Updatable};
use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::TypeId;
use crate::engine::component::timer::Timer;

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
    was_airborne: bool,
    is_against_wall: bool,
    is_against_ceiling: bool,
    jump: bool,
    is_jumping: bool,
}

impl PlayerPlatform {
    const C_SOLID: SlotID = 1;
    const JUMP_BUFFER_DURATION: f32 = 0.1;
    const JB_SLOT: SlotID = 0;
    const COYOTE_TIME: f32 = 0.1;
    const CT_SLOT: SlotID = 1;

    pub fn new(id: EntityID, init: PlayerPlatformInit, world: &mut World) -> Self {
        let mut transform = Transform::new();

        transform.set_size(Vec2::new(1.0, 2.0));
        transform.set_position(Vec2::from(init.starting_position));

        transform.snapshot();

        let physics_stats = PhysicsStats::default();

        let physics = PlatformerPhysics::new()
            .with_gravity(physics_stats.gravity())
            .with_terminal_velocity(physics_stats.terminal_velocity);

        let mut timers: Multi<Timer> = Multi::new();
        let jb_timer = Timer::new(Self::JUMP_BUFFER_DURATION);
        let ct_timer = Timer::new(Self::COYOTE_TIME);

        timers.insert(Self::JB_SLOT, jb_timer);
        timers.insert(Self::CT_SLOT, ct_timer);

        world.register_tickable::<Multi<Timer>>(true, false);

        world.add_component(id, transform);
        world.add_component(id, physics);
        world.add_component(id, physics_stats);
        world.add_component(id, timers);

        world.name_entity("player", id);

        let current_shader = world
            .shader_id("default")
            .expect("Failed to get default shader id");

        let mut p = PlayerPlatform {
            id,
            current_shader,
            z_index: 0,
            direction: Vec2::ZERO,
            is_on_ground: false,
            was_airborne: false,
            is_against_wall: false,
            is_against_ceiling: false,
            jump: false,
            is_jumping: false,
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
        let comps = world.get_many_components_mut(&[
            (self.id, TypeId::of::<Transform>()),
            (self.id, TypeId::of::<AnimationManager>()),
            (self.id, TypeId::of::<PlatformerPhysics>()),
            (self.id, TypeId::of::<PhysicsStats>()),
            (self.id, TypeId::of::<Multi<Timer>>()),
        ]);

        let mut comps = comps.into_iter();
        let transform: &mut Transform = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let animation_manager: &mut AnimationManager = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let phys: &mut PlatformerPhysics = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let phys_stats: &PhysicsStats = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let timers: &mut Multi<Timer> = comps.next().unwrap().unwrap().downcast_mut().unwrap();

        let mut direction = Vec2::ZERO;

        let right = input.is_action_down("right");
        let left = input.is_action_down("left");
        let down = input.is_action_down("down");
        let jump_pressed = input.is_action_pressed("jump").active;
        let jump_held = input.is_action_down("jump").active;

        direction.x = right.intensity - left.intensity;

        if direction.x > 0.0 {
            transform.set_mirror(Mirror::Visual(false, false));
        } else if direction.x < 0.0 {
            transform.set_mirror(Mirror::Visual(true, false));
        }

        if self.is_on_ground {
            if self.was_airborne {
                self.was_airborne = false;
            }

            let jb_timer = timers.get_mut(&Self::JB_SLOT).unwrap();
            if jump_pressed || jb_timer.is_running() {
                self.jump = true;
                jb_timer.reset();
            } else if direction.x == 0.0 {
                animation_manager.set_animation("idle");
            } else {
                animation_manager.set_animation("walk");
            }
        }
        else {
            if !self.was_airborne {
                self.was_airborne = true;
                let ct_timer = timers.get_mut(&Self::CT_SLOT).unwrap();
                ct_timer.start();
            }

            if jump_pressed {
                let jb_timer = timers.get_mut(&Self::JB_SLOT).unwrap();
                jb_timer.start();
            }

            if phys.velocity.y <= 0.0 {
                if jump_held {
                    phys.terminal_velocity = phys_stats.terminal_velocity_slow;
                }
                else if down.active {
                    phys.terminal_velocity = phys_stats.terminal_velocity_fast;
                }
                else {
                    phys.terminal_velocity = phys_stats.terminal_velocity;
                }

                animation_manager.set_animation("fall");

                let ct_timer = timers.get_mut(&Self::CT_SLOT).unwrap();
                if ct_timer.is_running() && jump_pressed {
                    self.jump = true;
                    ct_timer.reset();
                }
            } else if self.is_jumping {
                animation_manager.set_animation("jump");
            }
        }

        self.direction = direction;
    }

    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        let comps = world.get_many_components_mut(&[
            (self.id, TypeId::of::<PlatformerPhysics>()),
            (self.id, TypeId::of::<PhysicsStats>()),
        ]);

        let mut comps = comps.into_iter();
        let phys: &mut PlatformerPhysics = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let phys_stats: &PhysicsStats = comps.next().unwrap().unwrap().downcast_mut().unwrap();

        if self.is_on_ground || self.is_against_ceiling {
            phys.velocity.y = 0.0;
        }

        if self.is_against_wall {
            phys.velocity.x = 0.0;
        }

        if self.direction.x != 0.0 {
            let accel = if self.is_on_ground {
                phys_stats.ground_acceleration
            } else {
                phys_stats.air_acceleration
            };

            phys.accelerate(self.direction.x, accel, phys_stats.top_speed, delta_time);
        } else {
            let accel = if self.is_on_ground {
                phys_stats.ground_deceleration
            } else {
                phys_stats.air_deceleration
            };

            phys.decelerate(accel, delta_time);
        }

        if self.jump {
            self.jump = false;
            self.is_jumping = true;
            phys.apply_impulse(Vec2::new(0.0, phys_stats.jump_speed));
        }

        self.is_on_ground = false;
        self.is_against_wall = false;
        self.is_against_ceiling = false;
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
                tex_data,
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
                transform.size().x / 3.0,
                transform.size().y / 4.0
            ),
            rotation_offset: 0.0,
            offset: Vec2::new(0.0, -transform.size().y / 4.0)
        });

        let solid = Collider::new(self.id, Self::C_SOLID, shapes, world.gl())
            .with_layers(mask_bit(1), mask_bit(1))
            .with_collision_space(mask_bit(0))
            .with_debug_color(Vec4::new(1.0, 0.0, 0.0, 1.0))
            .with_collision_behavior(CollisionBehavior::Slide);

        let mut colliders: Multi<Collider> = Multi::new();
        colliders.insert(solid.id, solid);

        world.add_component(self.id, colliders);
    }

    fn react_to_collision(&mut self, collision: CollisionEvent, world: &mut World) {
        match collision.collision_type {
            CollisionType::Solid(_) => {
                let t: &Transform = world.get_component(self.id).unwrap();
                let local_up = t.local_up();
                if collision.normal.is_floor(local_up) {
                    self.is_on_ground = true;
                    world.event_bus.emit(PlayerLandingEvent);
                }
                else {
                    self.was_airborne = true;
                }

                if collision.normal.is_ceiling(local_up) {
                    self.is_against_ceiling = true;
                }

                if collision.normal.is_wall(local_up) {
                    self.is_against_wall = true;
                }
            },
            _ => ()
        }
    }
}
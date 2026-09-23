use crate::demos::platformer_demo::components::physics::{PhysicsStats, PlatformerPhysics};
use crate::demos::platformer_demo::events::PlayerLandingEvent;
use crate::engine::collision::math_utilities::CollisionNormalExtension;
use crate::engine::collision::shape::Shape;
use crate::engine::collision::{Collides, CollisionEvent, CollisionSystem, CollisionType};
use crate::engine::component::animation::{Animation, AnimationManager, AtlasOption, FrameInfo};
use crate::engine::component::collider::{Collider, ColliderType, CollisionBehavior};
use crate::engine::component::transform::{Mirror, Transform};
use crate::engine::component::{Multi, SlotID};
use crate::engine::input::Input;
use crate::engine::rendering::pass::PassMask;
use crate::engine::rendering::shader::ShaderID;
use crate::engine::rendering::{RenderItem, TextureProvider};
use crate::engine::world::{Intention, World};
use crate::engine::{mask_bit, Drawable, Entity, EntityID, Updatable};
use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::TypeId;
use rodio::Player;
use crate::demos::platformer_demo::components::abilities::{Abilities, AbilityReaction, AbilityResponse, DashAbility, DodgeAbility, ModifierEffect, PlayerAction};
use crate::demos::platformer_demo::components::ground_properties::GroundProperties;
use crate::demos::platformer_demo::components::state::PlayerState;
use crate::engine::component::timer::Timer;

#[derive(Deserialize, Serialize, Debug)]
pub struct PlayerPlatformInit {
    pub starting_position: (f32, f32),
    pub active: bool,
}

pub struct PlayerPlatform {
    id: EntityID,
    priority: i32,
    current_shader: ShaderID,
    z_index: i32,
    jump: bool,
    jump_modifiers: Vec<ModifierEffect>,
    queued_physics_reactions: Vec<AbilityReaction>,
}

impl PlayerPlatform {
    pub const C_SOLID: SlotID = 1;
    pub const C_H_BOX: SlotID = 2;
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

        let state = PlayerState::new(id);

        let mut abilities = Abilities::new();
        abilities.equip(0, Box::new(DashAbility::new()));
        abilities.equip(1, Box::new(DodgeAbility::new()));

        world.add_component(id, transform);
        world.add_component(id, physics);
        world.add_component(id, physics_stats);
        world.add_component(id, timers);
        world.add_component(id, state);
        world.add_component(id, abilities);

        world.name_entity("player", id);

        let current_shader = world
            .shader_id("default")
            .expect("Failed to get default shader id");

        let mut p = PlayerPlatform {
            id,
            priority: 1,
            current_shader,
            z_index: 0,
            jump: false,
            jump_modifiers: Vec::new(),
            queued_physics_reactions: Vec::new(),
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

    fn as_updatable(&self) -> Option<&dyn Updatable> {
        Some(self)
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
        if input.is_action_pressed("change_priority").active {
            if self.priority == 1 {
                self.priority = -1;
            } else {
                self.priority = 1;
            }
            world.push_intention(Intention::PriorityChange(self.priority, self.id));
        }

        let mut state: PlayerState = world.remove_component(self.id).unwrap();
        let mut abilities: Abilities = world.remove_component(self.id).unwrap();
        let mut animation_manager: AnimationManager = world.remove_component(self.id).unwrap();
        let mut timers: Multi<Timer> = world.remove_component(self.id).unwrap();

        let right = input.is_action_down("right");
        let left = input.is_action_down("left");
        let down = input.is_action_down("down");
        let jump_pressed = input.is_action_pressed("jump").active;
        let jump_held = input.is_action_down("jump").active;

        state.direction.x = right.intensity - left.intensity;

        abilities.tick(input, &mut state, world, delta_time);

        let transform: &mut Transform = world.get_component_mut(self.id).unwrap();

        if state.direction.x > 0.0 {
            transform.set_mirror(Mirror::Visual(false, false));
        } else if state.direction.x < 0.0 {
            transform.set_mirror(Mirror::Visual(true, false));
        }

        let mut wants_jump = false;

        if state.is_on_ground {
            if state.was_airborne {
                state.was_airborne = false;
            }

            let jb_timer = timers.get_mut(&Self::JB_SLOT).unwrap();
            if jump_pressed || jb_timer.is_running() {
                wants_jump = true;
                jb_timer.reset();
            } else if state.direction.x == 0.0 {
                animation_manager.set_animation("idle");
            } else {
                animation_manager.set_animation("walk");
            }
        }
        else {
            if !state.was_airborne {
                state.was_airborne = true;
                let ct_timer = timers.get_mut(&Self::CT_SLOT).unwrap();
                ct_timer.start();
            }

            if jump_pressed {
                let jb_timer = timers.get_mut(&Self::JB_SLOT).unwrap();
                jb_timer.start();
            }

            let phys_stats: &PhysicsStats = world.get_component(self.id).unwrap();
            let tv = if jump_held {
                phys_stats.terminal_velocity_slow
            }
            else if down.active {
                phys_stats.terminal_velocity_fast
            }
            else {
                phys_stats.terminal_velocity
            };

            let phys: &mut PlatformerPhysics = world.get_component_mut(self.id).unwrap();
            if phys.velocity.y <= 0.0 {
                phys.terminal_velocity = tv;

                animation_manager.set_animation("fall");

                let ct_timer = timers.get_mut(&Self::CT_SLOT).unwrap();
                if ct_timer.is_running() && jump_pressed {
                    wants_jump = true;
                    ct_timer.reset();
                }
            } else if state.is_jumping {
                animation_manager.set_animation("jump");
            }
        }

        if wants_jump {
            match abilities.resolve_action(
                PlayerAction::Jump,
                &mut state,
                world
            ) {
                AbilityResponse::Suppress => {},
                AbilityResponse::Modify(mods) => {
                    self.jump = true;
                    self.jump_modifiers = mods;
                },
                AbilityResponse::Reactions(reactions) => {
                    self.jump = true;
                    abilities.pending.extend(reactions);
                },
                AbilityResponse::PassThrough => {
                    self.jump = true;
                }
            }
        }

        for reaction in abilities.take_pending() {
            match reaction {
                AbilityReaction::SetAnimation(name) => animation_manager.set_animation(name),
                AbilityReaction::ToggleCollider(slot, state) => {
                    let colliders: &mut Multi<Collider> = world.get_component_mut(self.id).unwrap();

                    let Some(collider) = colliders.get_mut(&slot) else { break; };
                    collider.is_active = state;
                },
                physics_reaction => self.queued_physics_reactions.push(physics_reaction),
            }
        }

        world.add_component(self.id, animation_manager);
        world.add_component(self.id, timers);
        world.add_component(self.id, state);
        world.add_component(self.id, abilities);
    }

    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        let state: &PlayerState = world.get_component(self.id).unwrap();
        
        let ground_vel = if state.is_on_ground {
            if let Some(properties) = world.get_component::<GroundProperties>(state.ground_id) {
                properties.velocity
            } else {
                Vec2::ZERO
            }
        } else {
            Vec2::ZERO
        };
        
        let comps = world.get_many_components_mut(&[
            (self.id, TypeId::of::<PlatformerPhysics>()),
            (self.id, TypeId::of::<PhysicsStats>()),
            (self.id, TypeId::of::<PlayerState>()),
            (self.id, TypeId::of::<Transform>()),
        ]);

        let mut comps = comps.into_iter();
        let phys: &mut PlatformerPhysics = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let phys_stats: &PhysicsStats = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let state: &mut PlayerState = comps.next().unwrap().unwrap().downcast_mut().unwrap();
        let transform: &mut Transform = comps.next().unwrap().unwrap().downcast_mut().unwrap();

        phys.set_relative(ground_vel);
        
        if state.is_on_ground || state.is_against_ceiling {
            phys.velocity.y = 0.0;
        }

        if state.is_against_wall {
            phys.velocity.x = 0.0;
        }

        if state.direction.x != 0.0 {
            let accel = if state.is_on_ground {
                phys_stats.ground_acceleration
            } else {
                phys_stats.air_acceleration
            };

            phys.accelerate(state.direction.x, accel, phys_stats.top_speed, delta_time);
        } else {
            let accel = if state.is_on_ground {
                phys_stats.ground_deceleration
            } else {
                phys_stats.air_deceleration
            };

            phys.decelerate(accel, delta_time);
        }

        if self.jump {
            self.jump = false;
            state.is_jumping = true;
            let mut impulse = Vec2::new(0.0, phys_stats.jump_speed);

            for m in self.jump_modifiers.drain(..) {
                match m {
                    ModifierEffect::ScaleImpulse(s) => impulse *= s,
                    ModifierEffect::AddImpulse(i) => impulse += i,
                    ModifierEffect::OverrideVelocity(v) => phys.velocity = v,
                }
            }
            phys.apply_impulse(impulse);
        }

        for reaction in self.queued_physics_reactions.drain(..) {
            match reaction {
                AbilityReaction::Impulse(i) => phys.apply_impulse(i),
                AbilityReaction::Translation(t) => transform.translate(t),
                AbilityReaction::Rotation(r) => transform.rotate(r),
                AbilityReaction::OverrideVelocity(v) => phys.velocity = v,
                _ => {} // the rest were already resolved by update()
            }
        }

        state.is_on_ground = false;
        state.is_against_wall = false;
        state.is_against_ceiling = false;
    }

    fn priority(&self) -> i32 {
        1
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

        let solid = {
            let mut shapes = Vec::new();
            shapes.push(Shape::Rectangle {
                half_extents: Vec2::new(
                    transform.size().x / 3.0,
                    transform.size().y / 4.0
                ),
                rotation_offset: 0.0,
                offset: Vec2::new(0.0, -transform.size().y / 4.0)
            });

            Collider::new(self.id, Self::C_SOLID, shapes, world.gl())
                .with_layers(mask_bit(1), mask_bit(1))
                .with_collision_space(mask_bit(0))
                .with_debug_color(Vec4::new(1.0, 0.0, 0.0, 1.0))
                .with_collision_behavior(CollisionBehavior::Slide)
        };

        let hurt_box = {
            let mut shapes = Vec::new();
            shapes.push(Shape::Rectangle {
                half_extents: Vec2::new(
                    transform.size().x / 3.5,
                    transform.size().y / 4.0
                ),
                rotation_offset: 0.0,
                offset: Vec2::new(0.0, -transform.size().y / 4.0)
            });

            Collider::new(self.id, Self::C_H_BOX, shapes, world.gl())
                .with_layers(mask_bit(1), 0)
                .with_collision_space(mask_bit(0))
                .with_debug_color(Vec4::new(0.0, 0.0, 1.0, 1.0))
                .with_type(ColliderType::Trigger)
                .with_collision_behavior(CollisionBehavior::KeepGoing)
        };


        let mut colliders: Multi<Collider> = Multi::new();
        colliders.insert(solid.id, solid);
        colliders.insert(hurt_box.id, hurt_box);

        world.add_component(self.id, colliders);
    }

    fn react_to_collision(&mut self, collision: CollisionEvent, world: &mut World) {
        match collision.collision_type {
            CollisionType::Solid(_) => {
                let comps = world.get_many_components_mut(&[
                    (self.id, TypeId::of::<Transform>()),
                    (self.id, TypeId::of::<PlayerState>()),
                ]);

                let mut comps = comps.into_iter();
                let t: &Transform = comps.next().unwrap().unwrap().downcast_ref::<Transform>().unwrap();
                let state: &mut PlayerState = comps.next().unwrap().unwrap().downcast_mut().unwrap();

                let local_up = t.local_up();

                if collision.normal.is_floor(local_up) {
                    state.is_on_ground = true;
                    state.ground_id = collision.b.entity;
                }
                else {
                    state.was_airborne = true;
                }

                if collision.normal.is_ceiling(local_up) {
                    state.is_against_ceiling = true;
                }

                if collision.normal.is_wall(local_up) {
                    state.is_against_wall = true;
                }
            },
            _ => ()
        }
    }
}
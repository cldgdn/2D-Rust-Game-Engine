use std::collections::HashMap;
use glam::{Vec2, Vec4};
use crate::engine::{Drawable, Entity, EntityID};
use crate::engine::collision::{Collides, CollisionEvent, CollisionSystem};
use crate::engine::collision::shape::Shape;
use crate::engine::component::collider::{Collider, ColliderType};
use crate::engine::component::transform::Transform;
use crate::engine::Updatable;
use crate::engine::input::Input;
use crate::engine::rendering::{RenderItem, TextureProvider};
use crate::engine::rendering::shader::ShaderID;
use crate::engine::component::animation::{Animation, AnimationManager, AtlasOption, FrameInfo};
use crate::engine::component::{Multi, SlotID};
use crate::engine::world::{Intention, World};
use crate::platform_builder::events::SomeEvent;
use crate::platform_builder::spawnable::Projectile;

const MAIN_C: SlotID = 0;
const SOLID_C: SlotID = 1;

pub struct Player {
    id: EntityID,
    current_shader: ShaderID,
    pub z_index: i32,
    pub is_active: bool,
    pub velocity: Vec2,
    pub next_rotation: f32,
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
            next_rotation: 0.0,
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

        let mut tot_rotation = 0.0;
        let mut pressed = false;

        let action = input.is_action_pressed("rotate_right").active;
        if action {
            tot_rotation += 45.0;
            pressed = true;
        }

        let action = input.is_action_pressed("rotate_left").active;
        if action {
            tot_rotation -= 45.0;
            pressed = true;
        }

        if pressed {
            self.next_rotation = tot_rotation;
        }

    }

    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        if !self.is_active {
            return;
        }

        let transform = world.get_component_mut::<Transform>(self.id).unwrap();
        transform.translate(self.velocity * delta_time);
        transform.rotate(self.next_rotation);
        self.next_rotation = 0.0;
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

        let mut tex_data = animation_manager.get_texture_render_data()?;

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
        let transform = world.get_component::<Transform>(self.id).unwrap();

        let main = Collider::new(
            self.id,
            MAIN_C,
            1,
            0,
            ColliderType::Trigger,
            false,
            true,
            Vec::new(),
            Vec2::ZERO,
            0.0,
            None,
            world.gl()
        );

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
            rotation_offset: transform.rotation().to_radians(),
            offset: Vec2::ZERO
        });
        shapes.push(Shape::Rectangle {
            half_extents: Vec2::new(
                transform.total_dimensions().x / 2.0,
                transform.total_dimensions().y / 8.0
            ),
            rotation_offset: 0.0,
            offset: Vec2::new(0.0, -transform.total_dimensions().y / 8.0)
        });

        let solid = Collider::new(
            self.id,
            SOLID_C,
            2,
            2,
            ColliderType::Solid,
            false,
            true,
            shapes,
            Vec2::ZERO,
            0.0,
            Some(Vec4::new(1.0, 0.0, 0.0, 1.0)),
            world.gl()
        );
        


        let mut colliders: Multi<Collider> = Multi::new();
        colliders.insert(main.id, main);
        colliders.insert(solid.id, solid);

        world.add_component(self.id, colliders);
    }

    fn react_to_collision(&mut self, collision: CollisionEvent, world: &mut World) {
        ()
    }
}
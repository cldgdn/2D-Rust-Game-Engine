use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::demos::platformer_demo::components::ground_properties::GroundProperties;
use crate::engine::component::simple_sprite::{SimpleSprite, WrapMode};
use crate::engine::component::transform::{Mirror, Transform, TransformDisplayMode};
use crate::engine::rendering::shader::ShaderID;
use crate::engine::{mask_bit, mask_bits, Drawable, Entity, EntityID, Updatable};
use crate::engine::collision::{Collides, CollisionSystem};
use crate::engine::collision::shape::Shape;
use crate::engine::component::collider::{Collider, ResolutionBehavior};
use crate::engine::component::Multi;
use crate::engine::component::tween::{ease, EaseFunction, RepeatMode, TimeMode, Tween};
use crate::engine::rendering::pass::PassMask;
use crate::engine::rendering::{RenderItem, TextureProvider};
use crate::engine::world::World;
use crate::demos::wall::{Wall, WallInit};

#[derive(Serialize, Deserialize)]
pub struct MovingPlatformInit {
    pub starting_position: (f32, f32),
    pub ending_position: (f32, f32),
    pub tween_duration: f32,
    pub ease: EaseFunction,
    pub width: u32,
    pub height: u32,
    pub rotation: Option<f32>,
}

pub struct MovingPlatform {
    id: EntityID,
    current_shader: ShaderID,
    z_index: i32,
}

impl MovingPlatform {
    pub fn set_specific_atbs(world: &mut World) {
        world.register_tickable_fixed::<Tween<Vec2>>(true, true);
    }

    pub fn new(id: EntityID, init: MovingPlatformInit, world: &mut World) -> Self {
        let mut transform = Transform::new();

        transform.set_display_mode(TransformDisplayMode::None);

        transform.set_position(Vec2::from(init.starting_position));
        transform.set_size(Vec2::new(init.width as f32, init.height as f32));
        if let Some(rotation) = init.rotation {
            transform.set_rotation(rotation);
        }
        transform.snapshot();

        let sprite = SimpleSprite::new(
            world,
            "resources/textures/square.png",
            Vec2::ZERO,
            Vec2::new(16.0, 16.0)
        ).expect("Could not create sprite");

        let pixel_size = transform.pixel_size();
        let region_size = sprite.pixel_region_size();

        let sprite = sprite.with_wrap_mode(
            WrapMode::Tile(
                Vec2::new(
                    pixel_size.x / region_size.x,
                    pixel_size.y / region_size.y,
                )
            )
        );

        let mut tween = Tween::new(
            Vec2::from(init.starting_position),
            Vec2::from(init.ending_position),
            init.tween_duration,
            init.ease.as_fn(),
            TimeMode::Fixed,
        )
            .with_repeat(RepeatMode::PingPong)
            .with_wait(1.0);

        tween.restart();

        let ground_properties = GroundProperties::new();

        world.add_component(id, transform);
        world.add_component(id, sprite);
        world.add_component(id, tween);
        world.add_component(id, ground_properties);

        world.set_tickable_fixed_active::<Tween<Vec2>>(id, true);

        let current_shader = world
            .shader_id("default")
            .expect("No default shader");

        MovingPlatform {
            id, current_shader, z_index: 0
        }
    }

    pub fn from_init(id: EntityID, init: Value, world: &mut World) -> Box<dyn Entity> {
        let params: MovingPlatformInit = serde_json::from_value(init).expect("Failed to parse MovingPlatformInit");
        Box::new(MovingPlatform::new(id, params, world))
    }
}

impl Entity for MovingPlatform {
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

impl Updatable for MovingPlatform {
    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        let tween = world.get_component_mut::<Tween<Vec2>>(self.id).unwrap();

        let delta = tween.get_motion();

        let properties = world.get_component_mut::<GroundProperties>(self.id).unwrap();

        properties.velocity = delta;

        let transform = world.get_component_mut::<Transform>(self.id).unwrap();

        transform.translate(delta);
    }
}

impl Drawable for MovingPlatform {
    fn entity_id(&self) -> EntityID {
        self.id
    }

    fn get_render_item(&self, world: &World) -> Option<RenderItem> {
        let sprite = world.get_component::<SimpleSprite>(self.id)?;

        let tex_data = sprite.get_texture_render_data()?;

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

impl Collides for MovingPlatform {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem, world: &mut World) {
        let transform = world.get_component::<Transform>(self.id).unwrap();
        let size = transform.size();
        let mut shapes = Vec::new();
        shapes.push(Shape::Rectangle {
            half_extents: size / 2.0,
            rotation_offset: 0.0,
            offset: Vec2::ZERO,
        });

        let main = Collider::new(self.id, 0, shapes, world.gl())
            .with_layers(mask_bits(&[1, 2]), 0)
            .with_collision_space(mask_bit(0))
            .with_debug_color(Vec4::new(1.0, 0.0, 0.0, 1.0))
            .with_resolution_behavior(ResolutionBehavior::Stiff)
            .with_one_way(Some(Vec2::new(0.0, 1.0)));

        let mut colliders = Multi::new();

        colliders.insert(main.id, main);

        world.add_component(self.id, colliders);
    }
}
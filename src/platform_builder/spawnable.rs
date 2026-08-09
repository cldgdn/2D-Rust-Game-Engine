use glam::{Vec2, Vec4};
use crate::engine::component::transform::Transform;
use crate::engine::{mask_bit, BitMask, Drawable, Entity, EntityID, Updatable};
use crate::engine::collision::{Collides, CollisionEvent, CollisionSystem};
use crate::engine::collision::shape::Shape;
use crate::engine::component::collider::{Collider};
use crate::engine::component::Multi;
use crate::engine::input::Input;
use crate::engine::rendering::{RenderItem, TextureProvider};
use crate::engine::rendering::shader::ShaderID;
use crate::engine::component::simple_sprite::{SimpleSprite};
use crate::engine::rendering::pass::PassMask;
use crate::engine::world::{Intention, World};

pub struct Projectile {
    id: EntityID,
    current_shader: ShaderID,
    pub z_index: i32,
    direction: Vec2,
    velocity: f32,
    timer: f32,
    rotation_speed: f32,
}

impl Projectile {
    pub fn new(world: &mut World, start_pos: Vec2, direction: Vec2, velocity: f32) -> Projectile {
        let id = world.get_next_entity_id();
        let mut transform = Transform::new();

        transform.set_size(Vec2::new(1.0, 1.0));
        transform.set_position(start_pos);
        transform.set_scale(Vec2::new(0.5, 0.5));

        let sprite = SimpleSprite::new(
            world,
            "resources/textures/square.png",
            Vec2::ZERO,
            Vec2::new(16.0, 16.0)
        ).expect("Could not create sprite");

        let current_shader = world
            .shader_id("default")
            .expect("Failed to get shader id.");

        world.add_component(id, transform);
        world.add_component(id, sprite);

        Projectile {
            id,
            current_shader,
            z_index: 0,
            direction,
            velocity,
            timer: 0.0,
            rotation_speed: 15.0,
        }
    }
}

impl Entity for Projectile {
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

impl Updatable for Projectile {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        self.timer += delta_time;

        if self.timer > 10.0 {
            world.intention_queue.push(Intention::Destroy(self.id));
        }
    }

    fn fixed_update(&mut self, world: &mut World, delta_time: f32) {
        let transform = world.get_component_mut::<Transform>(self.id).unwrap();

        transform.translate(self.direction * self.velocity * delta_time);
        transform.rotate(self.rotation_speed * delta_time);
    }

    fn pause_mask(&self) -> BitMask {
        mask_bit(0)
    }
}

impl Drawable for Projectile {
    fn entity_id(&self) -> EntityID {
        self.id
    }

    fn get_render_item(&self, world: &World) -> Option<RenderItem> {
        let tex_data = world.get_component::<SimpleSprite>(self.id).unwrap().get_texture_render_data()?;

        Some (
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

impl Collides for Projectile {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem, world: &mut World) {
        let transform = world.get_component::<Transform>(self.id).unwrap();

        let mut shapes = Vec::new();
        shapes.push(Shape::Rectangle {
            half_extents: transform.scaled_size() / 2.0,
            rotation_offset: 0.0,
            offset: Vec2::ZERO
        });
        
        let collider = Collider::new(self.id, 0, shapes, world.gl())
            .with_layers(mask_bit(3), mask_bit(2))
            .with_collision_space(mask_bit(0))
            .with_debug_color(Vec4::new(1.0, 0.0, 0.0, 1.0));

        let mut colliders = Multi::new();

        colliders.insert(collider.id, collider);

        world.add_component(self.id, colliders);
    }

    fn react_to_collision(&mut self, collision: CollisionEvent, world: &mut World) {
        world.intention_queue.push(Intention::Destroy(self.id));
    }
}
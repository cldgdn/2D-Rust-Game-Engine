use glam::{Vec2, Vec4};
use crate::engine::component::transform::Transform;
use crate::engine::{mask_bit, Entity, EntityID, Updatable};
use crate::engine::collision::{Collides, CollisionSystem};
use crate::engine::collision::shape::Shape;
use crate::engine::component::collider::{Collider, ColliderType};
use crate::engine::component::Multi;
use crate::engine::input::Input;
use crate::engine::world::{Intention, World};
use crate::platform_builder::events::EnemyDeathEvent;
use crate::platform_builder::stats::Health;

pub struct Enemy {
    id: EntityID,
}

impl Enemy {
    pub fn new(world: &mut World) -> Self {
        let id = world.get_next_entity_id();
        let mut transform = Transform::new();
        let health = Health::new(10.0);

        transform.set_size(Vec2::new(1.0, 2.0));

        world.add_component(id, transform);
        world.add_component(id, health);

        Self {
            id
        }
    }
}

impl Entity for Enemy {
    fn id(&self) -> EntityID {
        self.id
    }

    fn as_updatable_mut(&mut self) -> Option<&mut dyn Updatable> {
        Some(self)
    }

    fn as_collides(&self) -> Option<&dyn Collides> {
        Some(self)
    }

    fn as_collides_mut(&mut self) -> Option<&mut dyn Collides> {
        Some(self)
    }
}

impl Updatable for Enemy {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        let health = world.get_component::<Health>(self.id).unwrap();
        if health.health <= 0.0 {
            world.event_bus.emit(EnemyDeathEvent{id: self.id});
            world.intention_queue.push(Intention::Destroy(self.id));
        }
    }
}

impl Collides for Enemy {
    fn submit_colliders(&self, collision_system: &mut CollisionSystem, world: &mut World) {
        let transform = world.get_component::<Transform>(self.id).unwrap();

        let mut shapes = Vec::new();
        shapes.push(Shape::Rectangle {
            half_extents: transform.size() / 2.0,
            rotation_offset: 0.0,
            offset: Vec2::ZERO
        });

        let main = Collider::new(self.id, 0, shapes, world.gl())
            .with_layers(mask_bit(4), 0)
            .with_collision_space(mask_bit(0))
            .with_debug_color(Vec4::new(0.0, 1.0, 0.0, 1.0))
            .with_type(ColliderType::Trigger);

        let mut colliders = Multi::new();
        colliders.insert(main.id, main);

        world.add_component(self.id, colliders);
    }
}
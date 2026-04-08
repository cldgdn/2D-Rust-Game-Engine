use glam::Vec2;
use crate::engine::{Entity, EntityID, Updatable};
use crate::engine::input::Input;
use crate::engine::world::World;

pub struct CameraController {
    id: EntityID
}

impl CameraController {
    pub fn new(world: &mut World) -> Self {
        Self {
            id: world.get_next_entity_id()
        }
    }
}

impl Entity for CameraController {
    fn id(&self) -> EntityID {
        self.id
    }

    fn as_updatable_mut(&mut self) -> Option<&mut dyn Updatable> {
        Some(self)
    }
}

impl Updatable for CameraController {
    fn update(&mut self, input: &Input, world: &mut World, delta_time: f32) {
        let camera = world.camera_mut();

        let mut direction = Vec2::new(0.0, 0.0);

        if input.is_action_down("walk_right") {
            direction.x += 1.0;
        }

        if input.is_action_down("walk_left") {
            direction.x -= 1.0;
        }

        if input.is_action_down("walk_up") {
            direction.y += 1.0;
        }

        if input.is_action_down("walk_down") {
            direction.y -= 1.0;
        }

        camera.position += (direction * delta_time * 10.0);
    }
}
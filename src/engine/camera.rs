use glam::{Mat4, Vec2, Vec3};
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::EntityID;
use crate::engine::world::World;

pub type CameraID = u32;

pub struct Camera {
    projection: Mat4,
    viewport_size: (u32, u32),
    pub id: CameraID,
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,
    pub target: Option<EntityID>,
    pub deadzone: Vec2,
}

impl Camera {
    pub fn new(id: CameraID, viewport_size: (u32, u32)) -> Self {
        let (width, height) = viewport_size;
        let half_w = width as i32 / 2;
        let half_h = height as i32 / 2;

        let projection = Mat4::orthographic_lh(
            -half_w as f32,
            half_w as f32,
            -half_h as f32,
            half_h as f32,
            1.0,
            -1.0,
        );
        
        Self {
            projection,
            viewport_size,
            id,
            position: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            target: None,
            deadzone: Vec2::ZERO,
        }
    }

    pub fn set_target(&mut self, target: Option<EntityID>) {
        self.target = target;
    }

    pub fn set_deadzone(&mut self, deadzone: Vec2) {
        self.deadzone = deadzone;
    }

    pub fn follow(&mut self, t_pos: &Vec2) {
        let dx = t_pos.x - self.position.x;
        let dy = t_pos.y - self.position.y;

        if dx.abs() >= self.deadzone.x {
            self.position.x += (dx.abs() - self.deadzone.x) * dx.signum();
        }
        if dy.abs() >= self.deadzone.y {
            self.position.y += (dy.abs() - self.deadzone.y) * dy.signum();
        }
    }
    
    pub fn get_projection(&self) -> Mat4 {
        self.projection
    }

    pub fn get_view(&self) -> Mat4 {
        let world_pos = self.position * config().unit_pixel_size as f32;
        let translation = Mat4::from_translation((-world_pos).extend(0.0));
        let rotation = Mat4::from_rotation_z(-self.rotation);
        let scale = 1.0 / self.zoom;
        let scale = Mat4::from_scale(Vec3::new(scale, scale, 1.0));

        translation * rotation * scale
    }

    pub fn get_view_projection(&self) -> Mat4 {
        self.get_projection() * self.get_view()
    }
}
use glam::{Mat4, Vec2, Vec3};
use crate::engine::config::config;

pub type CameraID = u32;

pub struct Camera {
    projection: Mat4,
    pub id: CameraID,
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,
}

impl Camera {
    pub fn new(id: CameraID) -> Self {
        let half_w = config().logic_size.0 as f32 * 0.5;
        let half_h = config().logic_size.1 as f32 * 0.5;

        let projection = Mat4::orthographic_lh(
            -half_w,
            half_w,
            -half_h,
            half_h,
            1.0,
            -1.0,
        );
        
        Self {
            projection,
            id,
            position: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
        }
    }
    
    pub fn get_projection(&self) -> Mat4 {
        self.projection
    }

    pub fn get_view(&self) -> Mat4 {
        let translation = Mat4::from_translation((-self.position).extend(0.0));
        let rotation = Mat4::from_rotation_z(-self.rotation);
        let scale = 1.0 / self.zoom;
        let scale = Mat4::from_scale(Vec3::new(scale, scale, 1.0));

        translation * rotation * scale
    }

    pub fn get_view_projection(&self) -> Mat4 {
        self.get_projection() * self.get_view()
    }
}
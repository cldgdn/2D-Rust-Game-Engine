use glam::{Mat4, Vec2, Vec3};
use crate::engine::config::config;

pub struct Camera {
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,
}

impl Camera {
    pub fn new(position: Vec2, zoom: f32, rotation: f32) -> Camera {
        Camera {
            position,
            zoom,
            rotation,
        }
    }

    pub fn get_projection(&self) -> Mat4 {
        let half_w = config().logic_size.0 as f32 * 0.5;
        let half_h = config().logic_size.1 as f32 * 0.5;

        Mat4::orthographic_lh(
            -half_w,
            half_w,
            -half_h,
            half_h,
            -1.0,
            1.0,
        )
    }

    pub fn get_view(&self) -> Mat4 {
        let translation = Mat4::from_translation((-self.position).extend(0.0));
        let rotation = Mat4::from_rotation_z(-self.rotation);
        let scale = 1.0 / self.zoom;
        let scale = Mat4::from_scale(Vec3::new(scale, scale, 1.0));

        scale * rotation * translation
    }

    pub fn get_view_projection(&self) -> Mat4 {
        self.get_projection() * self.get_view()
    }
}
use glam::{Mat4, Vec2, Vec3};
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::EntityID;
use crate::engine::world::World;

pub type CameraID = u32;

pub struct CameraBoundary {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl CameraBoundary {
    fn width(&self) -> f32 {
        self.right - self.left
    }

    fn height(&self) -> f32 {
        self.top - self.bottom
    }

    fn set_width(&mut self, new_width: f32) {
        let width = self.width();
        let dw = new_width - width;
        self.right += dw;
    }

    fn set_height(&mut self, new_height: f32) {
        let height = self.height();
        let dh = new_height - height;
        self.top += dh;
    }
}

pub struct Camera {
    projection: Mat4,
    viewport_size: (u32, u32),
    pub id: CameraID,
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,
    pub target: Option<EntityID>,
    pub deadzone: Vec2,
    pub boundary: Option<CameraBoundary>,
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
            boundary: None,
        }
    }

    pub fn set_target(&mut self, target: Option<EntityID>) {
        self.target = target;
    }

    pub fn set_deadzone(&mut self, deadzone: Vec2) {
        self.deadzone = deadzone;
    }

    pub fn set_boundary(&mut self, boundary: Option<CameraBoundary>) {
        if let Some(mut boundary) = boundary {
            let unit_viewport_size = self.unit_viewport_size();

            if boundary.width() < unit_viewport_size.0 {
                boundary.set_width(unit_viewport_size.0);
            }
            if boundary.height() < unit_viewport_size.1 {
                boundary.set_height(unit_viewport_size.1);
            }

            self.boundary = Some(boundary);
        } else {
            self.boundary = None;
        }
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

    pub fn clamp_to_boundary(&mut self) {
        if let Some(boundary) = &self.boundary {
            let unit_viewport_size = self.unit_viewport_size();

            let half_width = unit_viewport_size.0 / 2.0;
            let half_height = unit_viewport_size.1 / 2.0;

            let view_left = self.position.x - half_width;
            let view_right = self.position.x + half_width;
            let view_bottom = self.position.y - half_height;
            let view_top = self.position.y + half_height;

            if view_left < boundary.left {
                self.position.x += boundary.left - view_left;
            }

            if view_right > boundary.right {
                self.position.x -= view_right - boundary.right;
            }

            if view_bottom < boundary.bottom {
                self.position.y += boundary.bottom - view_bottom;
            }

            if view_top > boundary.top {
                self.position.y -= view_top - boundary.top;
            }
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

    fn unit_viewport_size(&self) -> (f32, f32) {
        (
            self.viewport_size.0 as f32 / config().unit_pixel_size as f32,
            self.viewport_size.1 as f32 / config().unit_pixel_size as f32,
        )
    }
}
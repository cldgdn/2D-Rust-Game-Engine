use glam::{Vec2, Vec3};

pub struct Transform {
    position: Vec3,
    scale: Vec2,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec2::new(1.0, 1.0),
        }
    }

    pub fn translate(&mut self, vec: Vec2) {
        self.position += vec.extend(0.0);
    }

    pub fn add_scale(&mut self, vec: Vec2) {
        self.scale += vec;
    }

    pub fn position(&self) -> &Vec3 {
        &self.position
    }

    pub fn position_2d(&self) -> Vec2 {
        Vec2::new(self.position.x, self.position.y)
    }

    pub fn scale(&self) -> &Vec2 {
        &self.scale
    }

    pub fn set_position(&mut self, new_pos: Vec2) {
        self.position = new_pos.extend(self.position.z);
    }

    pub fn set_depth(&mut self, depth: f32) {
        self.position.z = depth;
    }
    
    pub fn set_scale(&mut self, new_scale: Vec2) {
        self.scale = new_scale;
    }
}
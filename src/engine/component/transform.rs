use glam::Vec2;

pub struct Transform {
    position: Vec2,
    scale: Vec2,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec2::new(0.0, 0.0),
            scale: Vec2::new(1.0, 1.0),
        }
    }

    pub fn translate(&mut self, vec: Vec2) {
        self.position += vec;
    }

    pub fn add_scale(&mut self, vec: Vec2) {
        self.scale += vec;
    }

    pub fn position(&self) -> &Vec2 {
        &self.position
    }

    pub fn scale(&self) -> &Vec2 {
        &self.scale
    }

    pub fn set_position(&mut self, new_pos: Vec2) {
        self.position = new_pos;
    }

    pub fn set_scale(&mut self, new_scale: Vec2) {
        self.scale = new_scale;
    }
}
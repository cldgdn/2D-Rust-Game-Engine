use glam::{Vec2, Vec3};

pub enum Mirror {
    Visual(bool, bool),
    Logic(bool, bool),
    Both(bool, bool),
}

pub struct Transform {
    position: Vec2,
    rotation: f32,
    scale: Vec2,
    size: Vec2,
    mirror_visual: (bool, bool),
    mirror_logic: (bool, bool),
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec2::new(0.0, 0.0),
            rotation: 0.0,
            scale: Vec2::new(1.0, 1.0),
            size: Vec2::new(1.0, 1.0),
            mirror_visual: (false, false),
            mirror_logic: (false, false),
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

    pub fn rotate(&mut self, angle: f32) {
        self.rotation += angle;
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    pub fn scale(&self) -> &Vec2 {
        &self.scale
    }
    
    pub fn size(&self) -> &Vec2 {&self.size}
    
    pub fn total_dimensions(&self) -> Vec2 {
        Vec2::new(
            self.scale.x * self.size.x,
            self.scale.y * self.size.y,
        )
    }

    pub fn set_position(&mut self, new_pos: Vec2) {
        self.position = new_pos;
    }
    
    pub fn set_scale(&mut self, new_scale: Vec2) {
        self.scale = new_scale;
    }
    
    pub fn set_size(&mut self, new_size: Vec2) {self.size = new_size;}

    pub fn set_mirror(&mut self, mirror: Mirror) {
        match mirror {
            Mirror::Visual(x, y) => {
                self.mirror_visual = (x, y);
            }
            Mirror::Logic(x, y) => {
                self.mirror_logic = (x, y);
            }
            Mirror::Both(x, y) => {
                self.mirror_visual = (x, y);
                self.mirror_logic = (x, y);
            }
        }
    }

    pub fn toggle_mirror(&mut self, mirror: Mirror) {
        match mirror {
            Mirror::Visual(x, y) => {
                self.mirror_visual = (
                    if x {!self.mirror_visual.0} else {self.mirror_visual.0},
                    if y {!self.mirror_visual.1} else {self.mirror_visual.1},
                );
            },
            Mirror::Logic(x, y) => {
                self.mirror_logic = (
                    if x {!self.mirror_logic.0} else {self.mirror_logic.0},
                    if y {!self.mirror_logic.1} else {self.mirror_logic.1},
                );
            }
            Mirror::Both(x, y) => {
                self.mirror_visual = (
                    if x {!self.mirror_visual.0} else {self.mirror_visual.0},
                    if y {!self.mirror_visual.1} else {self.mirror_visual.1},
                );
                self.mirror_logic = (
                    if x {!self.mirror_logic.0} else {self.mirror_logic.0},
                    if y {!self.mirror_logic.1} else {self.mirror_logic.1},
                );
            }
        }
    }

    pub fn get_mirror_visual(&self) -> (bool, bool) {
        self.mirror_visual
    }

    pub fn get_mirror_logic(&self) -> (bool, bool) {
        self.mirror_logic
    }
}
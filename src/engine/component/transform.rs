use glam::{Vec2, Vec3};
use crate::engine::config::config;

/// Used to determine which axes to mirror and for which contexts.
pub enum Mirror {
    /// (x, y) -> Only mirrors the object visually, applied to all RenderItem-s issued by
    /// the owner Entity.
    Visual(bool, bool),
    /// (x, y) -> Only mirrors the object logically, no effect on rendering, but other components
    /// such as colliders may be reflected across Transform::position, rotated by Transform.rotation
    Logic(bool, bool),
    /// (x, y) -> Mirros the object both visually and logically.
    Both(bool, bool),
}

/// Handles positioning for any Entity that stores one as a component.
///
/// It is REQUIRED for any Entity that is Drawable or Collides. For use respectively in
/// rendering and collider placement for collision detection.
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

    /// Translates along the given vector in units
    pub fn translate(&mut self, vec: Vec2) {
        self.position += vec;
    }

    /// Scales in each axis by the value specified in the vector
    pub fn add_scale(&mut self, vec: Vec2) {
        self.scale += vec;
    }

    /// Returns the current position in units
    pub fn position(&self) -> &Vec2 {
        &self.position
    }

    /// Rotates the object by the specified angle in degrees.
    pub fn rotate(&mut self, angle: f32) {
        self.rotation += angle;
    }

    /// Returns the current rotation in degrees.
    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    /// Returns the current scaling factor for both axes
    pub fn scale(&self) -> &Vec2 {
        &self.scale
    }

    /// Returns the current unscaled size in units
    pub fn size(&self) -> &Vec2 {&self.size}

    /// Returns the scaled size in units
    pub fn scaled_size(&self) -> Vec2 {
        Vec2::new(
            self.scale.x * self.size.x,
            self.scale.y * self.size.y,
        )
    }

    /// Returns the scaled size in viewport pixels
    pub fn pixel_size(&self) -> Vec2 {
        self.scaled_size() * config().unit_pixel_size as f32
    }

    /// Sets the position directly, in units
    pub fn set_position(&mut self, new_pos: Vec2) {
        self.position = new_pos;
    }

    /// Sets the scale directly
    pub fn set_scale(&mut self, new_scale: Vec2) {
        self.scale = new_scale;
    }

    /// Sets the size directly, in units
    pub fn set_size(&mut self, new_size: Vec2) {self.size = new_size;}

    /// Sets the mirroring to whatever is being passed.
    ///
    /// mirror = Mirror::Logic(true, false) means the x logic mirroring will be set to true, while on y to false.
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

    /// Toggles the mirroring based on what value is passed
    ///
    /// mirror = Mirror::Logic(true, false) means the x logic mirroring wille be toggled, while y won't.
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

    /// Returns the value of visual mirroring as (x, y)
    pub fn get_mirror_visual(&self) -> (bool, bool) {
        self.mirror_visual
    }
    /// Returns the value of logic mirroring as (x, y)
    pub fn get_mirror_logic(&self) -> (bool, bool) {
        self.mirror_logic
    }
}
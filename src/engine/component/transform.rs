use glam::{Vec2};
use serde::{Deserialize, Serialize};
use crate::engine::config::config;

/// Used to determine which axes to mirror and for which contexts.
pub enum Mirror {
    /// (x, y) -> Only mirrors the object visually, applied to all RenderItem-s issued by
    /// the owner Entity.
    Visual(bool, bool),
    /// (x, y) -> Only mirrors the object logically, no effect on rendering, but other components
    /// such as colliders may be reflected across Transform::position, rotated by Transform.rotation
    Logic(bool, bool),
    /// (x, y) -> Mirrors the object both visually and logically.
    Both(bool, bool),
}

/// Used to determine the transform's behavior when giving values to the renderer
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum TransformDisplayMode {
    /// Interpolates between the last position/rotation/scale and the one just set.
    /// Produces smooth and always correct visuals, but keeps them one full physics frame behind.
    Interpolated,
    /// Extrapolates position/rotation/scale based on velocity/rotation speed/growth speed.
    /// Produces smooth and up-to-date visuals, but may cause snapbacks when the extrapolated values
    /// get corrected by collision or input when a new physics frame comes around.
    Extrapolated,
    /// Returns the raw values of position/rotation/scale.
    /// Causes motion to be restricted to physics framerate.
    /// Not as smooth if framerate > phys-framerate, but always correct and with 0 latency.
    None
}

/// Handles positioning for any Entity that stores one as a component.
///
/// It is REQUIRED for any Entity that is Drawable or Collides. For use respectively in
/// rendering and collider placement for collision detection.
pub struct Transform {
    position: Vec2,
    previous_position: Vec2,
    rotation: f32,
    previous_rotation: f32,
    scale: Vec2,
    previous_scale: Vec2,
    size: Vec2,
    mirror_visual: (bool, bool),
    mirror_logic: (bool, bool),
    display_mode: TransformDisplayMode,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            previous_position: Vec2::ZERO,
            rotation: 0.0,
            previous_rotation: 0.0,
            scale: Vec2::new(1.0, 1.0),
            previous_scale: Vec2::new(1.0, 1.0),
            size: Vec2::new(1.0, 1.0),
            mirror_visual: (false, false),
            mirror_logic: (false, false),
            display_mode: config().default_transform_display_mode
        }
    }

    /// Translates along the given vector in units
    pub fn translate(&mut self, vec: Vec2) {
        self.position += vec;
    }

    /// Increases scale in each axis by the value specified in the vector
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

    /// Sets the rotation directly, in degrees
    pub fn set_rotation(&mut self, new_rotation: f32) {
        self.rotation = new_rotation;
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

    /// ENGINE-ONLY CODE
    ///
    /// Snapshots the current position/rotation/scale as the "previous" values used for Interpolation
    /// and Extrapolation.
    ///
    /// Called engine-side once per fixed_update, before dispatching the fixed_update call to all
    /// Updatable Entities.
    pub fn snapshot(&mut self) {
        self.previous_position = self.position;
        self.previous_rotation = self.rotation;
        self.previous_scale = self.scale;
    }
    /// Returns the position to use for rendering this frame, based on `self.display_mode`.
    ///
    /// `elapsed_time`: seconds elapsed since the last fixed_update tick was committed (so since
    /// the last call to `snapshot()`).
    pub fn get_display_position(&self, elapsed_time: f32) -> Vec2 {
        match self.display_mode {
            TransformDisplayMode::Interpolated => {
                self.previous_position.lerp(self.position, Self::interpolation_alpha(elapsed_time))
            },
            TransformDisplayMode::Extrapolated => {
                let velocity = (self.position - self.previous_position) / config().fixed_delta_time;
                self.position + velocity * elapsed_time
            },
            TransformDisplayMode::None => {
                self.position
            }
        }
    }
    /// Returns the rotation in degrees to use for rendering this frame, based on `self.display_mode`
    ///
    /// `elapsed_time`: seconds elapsed since the last fixed_update tick was committed (so since
    /// the last call to `snapshot()`).
    pub fn get_display_rotation(&self, elapsed_time: f32) -> f32 {
        match self.display_mode {
            TransformDisplayMode::Interpolated => {
                let alpha = Self::interpolation_alpha(elapsed_time);
                self.previous_rotation + (self.rotation - self.previous_rotation) * alpha
            },
            TransformDisplayMode::Extrapolated => {
                let angular_velocity = (self.rotation - self.previous_rotation) / config().fixed_delta_time;
                self.rotation + angular_velocity * elapsed_time
            },
            TransformDisplayMode::None => {
                self.rotation
            }
        }
    }
    /// Returns the scale to use for rendering this frame, based on `self.display_mode`
    ///
    /// `elapsed_time`: seconds elapsed since the last fixed_update tick was committed (so since
    /// the last call to `snapshot()`).
    pub fn get_display_scale(&self, elapsed_time: f32) -> Vec2 {
        match self.display_mode {
            TransformDisplayMode::Interpolated => {
                self.previous_scale.lerp(self.scale, Self::interpolation_alpha(elapsed_time))
            },
            TransformDisplayMode::Extrapolated => {
                let velocity = (self.scale - self.previous_scale) / config().fixed_delta_time;
                self.scale + velocity * elapsed_time
            },
            TransformDisplayMode::None => {
                self.scale
            }
        }
    }
    pub fn get_display_scaled_size(&self, elapsed_time: f32) -> Vec2 {
        let scale = self.get_display_scale(elapsed_time);
        Vec2::new(
            self.size.x * scale.x,
            self.size.y * scale.y
        )
    }

    /// Converts elapsed-time-since-last-snapshot into a [0, 1] interpolation blend factor.
    /// Clamped so a late/delayed snapshot() call can't overshoot past the current values.
    fn interpolation_alpha(elapsed_time: f32) -> f32 {
        (elapsed_time / config().fixed_delta_time).clamp(0.0, 1.0)
    }
}
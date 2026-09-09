use glam::{Vec2, Vec3};

pub trait CollisionNormalExtension {
    /// Returns true if this normal points exactly in the entity's local up direction.
    fn is_floor(&self, local_up: Vec2) -> bool;
    /// Returns true if this normal is within max_angle radians of local up.
    fn is_floor_lenient(&self, local_up: Vec2, max_angle: f32) -> bool;
    /// Returns true if this normal points exactly opposite to local up.
    fn is_ceiling(&self, local_up: Vec2) -> bool;
    /// Returns true if this normal is within max_angle radians of local up.
    fn is_ceiling_lenient(&self, local_up: Vec2, max_angle: f32) -> bool;
    /// Returns true if this normal is perpendicular to local up
    fn is_wall(&self, local_up: Vec2) -> bool;
    /// Returns true if this normal is within max_angle radians of being perpendicular to local up
    fn is_wall_lenient(&self, local_up: Vec2, max_angle: f32) -> bool;
}

/// How many radians are considered as "perfect" for alignment. Avoids floating point precision.
const STRICT_EPSILON: f32 = 0.0001;

impl CollisionNormalExtension for Vec2 {
    fn is_floor(&self, local_up: Vec2) -> bool {
        self.dot(local_up) >= 1.0 - STRICT_EPSILON
    }

    fn is_floor_lenient(&self, local_up: Vec2, max_angle: f32) -> bool {
        self.dot(local_up) >= max_angle.cos()
    }

    fn is_ceiling(&self, local_up: Vec2) -> bool {
        self.dot(local_up) <= -1.0 + STRICT_EPSILON
    }

    fn is_ceiling_lenient(&self, local_up: Vec2, max_angle: f32) -> bool {
        self.dot(local_up) <= -max_angle.cos()
    }

    fn is_wall(&self, local_up: Vec2) -> bool {
        self.dot(local_up).abs() <= STRICT_EPSILON
    }

    fn is_wall_lenient(&self, local_up: Vec2, max_angle: f32) -> bool {
        self.dot(local_up).abs() <= max_angle.sin()
    }
}
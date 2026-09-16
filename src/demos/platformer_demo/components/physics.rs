use glam::Vec2;
use crate::engine::{EntityID};
use crate::engine::component::transform::Transform;
use crate::engine::component::UpdatingFixed;
use crate::engine::world::World;

/// Seeks `current` toward `target` by at most `max_delta`, without overshooting.
/// The shared building block behind `accelerate`, `decelerate`, and gravity/terminal
/// velocity clamping below - all three are "move this number toward that number, but
/// no further than this step" at heart.
fn move_toward_scalar(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

/// Velocity-based physics for a side-view platformer character: horizontal movement is
/// accelerate/decelerate/top-speed driven (parameters supplied per-call, since they're
/// context-dependent - grounded vs airborne vs sliding - not fixed physical constants),
/// vertical movement is gravity plus impulses (jumps, knockback, momentum transfer from
/// a moving platform).
///
/// Does not know about collision - if you want ground/wall detection to stop movement
/// or zero out velocity on contact, that's collision resolution's job, reading/writing
/// `velocity` directly (it's public) before or after this component's own fixed_update.
pub struct PlatformerPhysics {
    /// Current velocity, in units/second. X: horizontal. Y: vertical (sign convention
    /// follows whatever your engine treats as "up" - adjust `apply_gravity`'s sign if
    /// falling should decrease Y instead of increase it).
    pub velocity: Vec2,

    /// Acceleration applied to `velocity.y` every fixed_update, in units/s^2, while
    /// `use_gravity` is true.
    pub gravity: f32,

    /// Caps the magnitude of the falling component of `velocity.y`. `0.0` disables
    /// the cap (uncapped fall speed).
    pub terminal_velocity: f32,

    /// Whether `apply_gravity` has any effect this fixed_update. Toggle off for e.g.
    /// a grapple hook, a temporary flight power-up, or while frozen/stunned.
    pub use_gravity: bool,
}

impl PlatformerPhysics {
    pub fn new() -> Self {
        Self {
            velocity: Vec2::ZERO,
            gravity: 0.0,
            terminal_velocity: 0.0,
            use_gravity: true,
        }
    }

    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn with_terminal_velocity(mut self, terminal_velocity: f32) -> Self {
        self.terminal_velocity = terminal_velocity;
        self
    }

    /// Seeks horizontal velocity toward `direction * top_speed`, at `acceleration`
    /// units/s^2. `direction` is clamped to [-1, 1] so an out-of-range analog input
    /// (e.g. a slightly miscalibrated gamepad stick) can never exceed `top_speed`.
    ///
    /// Call this every fixed_update the character is actively trying to move, with
    /// whatever `acceleration`/`top_speed` fit the current context (grounded, airborne,
    /// on ice, etc - these are NOT stored on this component, since they vary by
    /// situation rather than being physical constants of the character itself).
    pub fn accelerate(&mut self, direction: f32, acceleration: f32, top_speed: f32, dt: f32) {
        let target = direction.clamp(-1.0, 1.0) * top_speed;
        self.velocity.x = move_toward_scalar(self.velocity.x, target, acceleration * dt);
    }

    /// Seeks horizontal velocity toward zero, at `deceleration` units/s^2. Call this
    /// when there's no active movement input (or you want to actively brake) instead
    /// of `accelerate` - unlike a naive subtraction, this can never overshoot past zero
    /// and reverse direction.
    pub fn decelerate(&mut self, deceleration: f32, dt: f32) {
        self.velocity.x = move_toward_scalar(self.velocity.x, 0.0, deceleration * dt);
    }

    /// Applies one fixed_update's worth of gravity to `velocity.y`, respecting
    /// `use_gravity` and `terminal_velocity`. Called automatically from
    /// `fixed_update` below - only call it manually if you want gravity applied
    /// somewhere other than this component's own dispatch.
    pub fn apply_gravity(&mut self, dt: f32) {
        if !self.use_gravity {
            return;
        }
        self.velocity.y += self.gravity * dt;

        if self.terminal_velocity != 0.0 {
            let max_fall_speed = self.terminal_velocity.abs();
            if self.gravity > 0.0 {
                // +Y down convention: falling = velocity.y increasing
                self.velocity.y = self.velocity.y.min(max_fall_speed);
            } else if self.gravity < 0.0 {
                // +Y up convention: falling = velocity.y decreasing
                self.velocity.y = self.velocity.y.max(-max_fall_speed);
            }
        }
    }

    /// Instantly adds `impulse` to velocity - jumps, knockback, explosions, or
    /// transferring a moving platform's velocity to a rider on dismount (see the
    /// moving-platform discussion: this is the mechanism for that, not Transform
    /// parenting).
    pub fn apply_impulse(&mut self, impulse: Vec2) {
        self.velocity += impulse;
    }
}

impl UpdatingFixed for PlatformerPhysics {
    fn update(&mut self, owner: EntityID, world: &mut World, fixed_delta_time: f32) {
        self.apply_gravity(fixed_delta_time);

        let delta = self.velocity * fixed_delta_time;

        if let Some(transform) = world.get_component_mut::<Transform>(owner) {
            transform.translate(delta);
        }
    }
}


pub struct PhysicsStats {
    pub terminal_velocity: f32,
    pub terminal_velocity_fast: f32,
    pub terminal_velocity_slow: f32,

    pub ground_acceleration: f32,
    pub ground_deceleration: f32,

    pub air_acceleration: f32,
    pub air_deceleration: f32,

    gravity: f32,

    pub top_speed: f32,

    pub jump_speed: f32,
    max_jump_height: f32,
}

impl Default for PhysicsStats {
    fn default() -> Self {
        let mut s = Self {
            terminal_velocity: 5.0,
            terminal_velocity_fast: 6.0,
            terminal_velocity_slow: 4.0,
            ground_acceleration: 4.0,
            ground_deceleration: 9.0,
            air_acceleration: 3.0,
            air_deceleration: 4.0,
            gravity: -3.0,
            top_speed: 8.0,
            jump_speed: 0.0,
            max_jump_height: 5.1,
        };

        s.recompute_jump_speed();

        s
    }
}

impl PhysicsStats {
    pub fn gravity(&self) -> f32 {
        self.gravity
    }

    pub fn set_gravity(&mut self, gravity: f32) {
        self.gravity = gravity;

        self.recompute_jump_speed();
    }

    pub fn max_jump_height(&self) -> f32 {
        self.max_jump_height
    }

    pub fn set_max_jump_height(&mut self, max_jump_height: f32) {
        self.max_jump_height = max_jump_height;

        self.recompute_jump_speed();
    }

    fn recompute_jump_speed(&mut self) {
        self.jump_speed = (2.0 * self.gravity.abs() * self.max_jump_height).sqrt();
    }
}
use std::ops::{Add, Mul, Sub};
use serde::{Deserialize, Serialize};
use crate::engine::component::{Tickable, TickableFixed};

/// Anything a Tween can operate on: needs a zero, addition, subtraction, and
/// scaling by a scalar. f32 and glam::Vec2/Vec3/Vec4 satisfy this already.
pub trait TweenValue:
Copy + Default + Add<Output = Self> + Sub<Output = Self> + Mul<f32, Output = Self>
{
}
impl<T> TweenValue for T where
    T: Copy + Default + Add<Output = Self> + Sub<Output = Self> + Mul<f32, Output = Self>
{
}

/// Which loop this tween expects to be advanced by. Purely informational:
/// the tween tags itself with the intent set at construction, and its
/// `advance_update` / `advance_fixed` entry points (called from your
/// Tickable / TickableFixed impls) each no-op unless this matches. That
/// makes it safe to register a Tween on both ATB dispatch lists
/// unconditionally, if that's ever simpler than branching at registration
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeMode {
    Update,
    Fixed,
}

/// What happens when elapsed time reaches the end of the duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    /// Stop at `end`, mark finished.
    Once,
    /// Jump back to `start` and continue. Note: this produces a discontinuous
    /// delta on the wrap frame (end -> start instantly). Fine for a repeating
    /// VFX pulse; almost certainly wrong for driving a Transform's position
    /// via get_motion(), unless start == end. Use PingPong for movement.
    Loop,
    /// Reverse direction at each end. Continuous, no jump.
    PingPong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Finished,
}

pub struct Tween<T: TweenValue> {
    start: T,
    end: T,
    duration: f32,
    elapsed: f32,
    delay: f32,
    ease: Box<dyn Fn(f32) -> f32>,
    time_mode: TimeMode,
    repeat: RepeatMode,
    state: PlaybackState,
    going_forward: bool, // used by PingPong
    last_sample: T,
    pending_motion: T,
}

impl<T: TweenValue> Tween<T> {
    pub fn new(
        start: T,
        end: T,
        duration: f32,
        ease: impl Fn(f32) -> f32 + 'static,
        time_mode: TimeMode,
    ) -> Self {
        Self {
            start,
            end,
            duration: duration.max(0.0),
            elapsed: 0.0,
            delay: 0.0,
            ease: Box::new(ease),
            time_mode,
            repeat: RepeatMode::Once,
            state: PlaybackState::Finished,
            going_forward: true,
            last_sample: start,
            pending_motion: T::default(),
        }
    }

    pub fn with_delay(mut self, delay: f32) -> Self {
        self.delay = delay.max(0.0);
        self
    }

    pub fn with_repeat(mut self, repeat: RepeatMode) -> Self {
        self.repeat = repeat;
        self
    }

    pub fn time_mode(&self) -> TimeMode {
        self.time_mode
    }

    pub fn is_finished(&self) -> bool {
        self.state == PlaybackState::Finished
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == PlaybackState::Paused {
            self.state = PlaybackState::Playing;
        }
    }

    /// Call from your Tickable::tick impl. No-ops unless time_mode == Update,
    /// so it's safe to call unconditionally.
    pub fn advance_update(&mut self, dt: f32) {
        if self.time_mode == TimeMode::Update {
            self.advance(dt);
        }
    }

    /// Call from your TickableFixed::tick_fixed impl. No-ops unless
    /// time_mode == Fixed.
    pub fn advance_fixed(&mut self, dt: f32) {
        if self.time_mode == TimeMode::Fixed {
            self.advance(dt);
        }
    }

    fn advance(&mut self, dt: f32) {
        if self.state != PlaybackState::Playing {
            return;
        }

        if self.delay > 0.0 {
            self.delay -= dt;
            if self.delay > 0.0 {
                return;
            }
            // Carry the leftover dt (past the delay) into this frame's step.
            let overshoot = -self.delay;
            self.delay = 0.0;
            self.step(overshoot);
            return;
        }

        self.step(dt);
    }

    fn step(&mut self, dt: f32) {
        if self.duration <= 0.0 {
            self.jump_to_end();
            return;
        }

        let signed_dt = if self.going_forward { dt } else { -dt };
        self.elapsed += signed_dt;

        match self.repeat {
            RepeatMode::Once => {
                if self.elapsed >= self.duration {
                    self.elapsed = self.duration;
                    self.sample_and_accumulate();
                    self.state = PlaybackState::Finished;
                    return;
                }
            }
            RepeatMode::Loop => {
                if self.elapsed >= self.duration {
                    self.elapsed -= self.duration;
                    self.sample_and_accumulate(); // flush the tail at `end`
                    self.last_sample = self.start; // seam: next delta is from start
                }
            }
            RepeatMode::PingPong => {
                if self.elapsed >= self.duration {
                    self.elapsed = self.duration - (self.elapsed - self.duration);
                    self.going_forward = false;
                } else if self.elapsed <= 0.0 {
                    self.elapsed = -self.elapsed;
                    self.going_forward = true;
                }
            }
        }

        self.sample_and_accumulate();
    }

    fn jump_to_end(&mut self) {
        self.elapsed = self.duration;
        self.sample_and_accumulate();
        self.state = PlaybackState::Finished;
    }

    fn sample_and_accumulate(&mut self) {
        let t = if self.duration > 0.0 {
            self.elapsed / self.duration
        } else {
            1.0
        };
        let eased_t = (self.ease)(t.clamp(0.0, 1.0));
        let sample = self.start + (self.end - self.start) * eased_t;
        self.pending_motion = self.pending_motion + (sample - self.last_sample);
        self.last_sample = sample;
    }

    /// Drain and return the motion accumulated since the last call.
    pub fn get_motion(&mut self) -> T {
        let motion = self.pending_motion;
        self.pending_motion = T::default();
        motion
    }

    /// Current absolute value, for consumers that want to read the tween's
    /// position directly (e.g. a UI progress readout) rather than accumulate
    /// deltas.
    pub fn current_value(&self) -> T {
        self.last_sample
    }

    /// Restart from `start`, as configured. Good for tweens that always begin
    /// from a known rest state.
    pub fn restart(&mut self) {
        self.elapsed = 0.0;
        self.going_forward = true;
        self.state = PlaybackState::Playing;
        self.last_sample = self.start;
    }

    /// Restart, but begin interpolating from wherever the tween currently is
    /// rather than snapping to `start`. Useful for re-triggering a tween that
    /// might already be in flight (e.g. re-hovering a button before its flash
    /// finished) without a visible pop.
    pub fn restart_from_current(&mut self) {
        self.start = self.last_sample;
        self.elapsed = 0.0;
        self.going_forward = true;
        self.state = PlaybackState::Playing;
    }

    pub fn retarget(&mut self, start: T, end: T) {
        self.start = start;
        self.end = end;
        self.restart();
    }
}

impl <T: TweenValue + 'static> Tickable for Tween<T> {
    fn tick(&mut self, delta_time: f32) {
        self.advance(delta_time);
    }

    fn is_active(&self) -> bool {
        self.state == PlaybackState::Playing
    }
}

impl <T: TweenValue + 'static> TickableFixed for Tween<T> {
    fn tick(&mut self, delta_time: f32) {
        self.advance_fixed(delta_time);
    }

    fn is_active(&self) -> bool {
        self.state == PlaybackState::Playing
    }
}

/// Default, ready-to-use easing functions. Pass any of these (or your own
/// closure) as the `ease` argument to `Tween::new`.
pub mod ease {
    pub fn linear(t: f32) -> f32 {
        t
    }

    pub fn quad_in(t: f32) -> f32 {
        t * t
    }
    pub fn quad_out(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t)
    }
    pub fn quad_in_out(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        }
    }

    pub fn cubic_in(t: f32) -> f32 {
        t * t * t
    }
    pub fn cubic_out(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(3)
    }
    pub fn cubic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }

    pub fn sine_in(t: f32) -> f32 {
        1.0 - (t * std::f32::consts::FRAC_PI_2).cos()
    }
    pub fn sine_out(t: f32) -> f32 {
        (t * std::f32::consts::FRAC_PI_2).sin()
    }
    pub fn sine_in_out(t: f32) -> f32 {
        -((std::f32::consts::PI * t).cos() - 1.0) / 2.0
    }

    pub fn back_out(t: f32) -> f32 {
        let c1 = 1.70158;
        let c3 = c1 + 1.0;
        1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
    }

    pub fn elastic_out(t: f32) -> f32 {
        if t == 0.0 {
            return 0.0;
        }
        if t == 1.0 {
            return 1.0;
        }
        let c4 = (2.0 * std::f32::consts::PI) / 3.0;
        2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EaseFunction {
    #[default]
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    SineIn,
    SineOut,
    SineInOut,
    BackOut,
    ElasticOut,
}

impl EaseFunction {
    /// Resolve to the concrete function pointer. Every arm has the exact
    /// same type (`fn(f32) -> f32`) since every function in `ease` is a
    /// plain fn item with no captured state, so this compiles without any
    /// boxing or dynamic dispatch.
    pub fn as_fn(self) -> fn(f32) -> f32 {
        match self {
            EaseFunction::Linear => ease::linear,
            EaseFunction::QuadIn => ease::quad_in,
            EaseFunction::QuadOut => ease::quad_out,
            EaseFunction::QuadInOut => ease::quad_in_out,
            EaseFunction::CubicIn => ease::cubic_in,
            EaseFunction::CubicOut => ease::cubic_out,
            EaseFunction::CubicInOut => ease::cubic_in_out,
            EaseFunction::SineIn => ease::sine_in,
            EaseFunction::SineOut => ease::sine_out,
            EaseFunction::SineInOut => ease::sine_in_out,
            EaseFunction::BackOut => ease::back_out,
            EaseFunction::ElasticOut => ease::elastic_out,
        }
    }
}

impl From<EaseFunction> for fn(f32) -> f32 {
    fn from(ease_fn: EaseFunction) -> Self {
        ease_fn.as_fn()
    }
}
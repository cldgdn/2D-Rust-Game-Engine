use glam::Vec2;
use serde::{Deserialize, Serialize};
use crate::engine::config::config;
use crate::engine::EntityID;

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

/// Encodes Transform parenting data.
///
/// `Transform::position`/`rotation`/`scale` ALWAYS hold resolved world-space values -
/// the renderer, collision system, and `get_display_*` never need to know a Transform
/// is parented. `pos_offset`/`rot_offset`/`scale_offset` here are the LOCAL values,
/// expressed in the parent's frame of reference; they're what you edit (via
/// `Transform::set_local_position` and friends) to move a parented object, and they're
/// what `Transform::resolve_parent` combines with the parent's current world pose to
/// refresh `position`/`rotation`/`scale` each fixed_update.
pub struct TransformParentingInfo {
    /// The Entity which owns the parent Transform, for retrieval from World
    pub id: EntityID,
    /// The position that the child Transform would be in a frame of reference
    /// centered on the parent
    pub pos_local: Vec2,
    /// The child's rotation around the parent's center, in degrees
    pub rot_local: f32,
    /// The scaling factor the child Transform has relative to its parent
    pub scale_local: Vec2,
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
    /// If Some, this Transform's position/rotation/scale are kept relative to
    /// another Entity's Transform. See `TransformParentingInfo` and `resolve_parent`.
    parent: Option<TransformParentingInfo>,
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
            display_mode: config().default_transform_display_mode,
            parent: None,
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

    /// Returns the previous position in units
    pub fn previous_position(&self) -> &Vec2 {
        &self.previous_position
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
    ///
    /// NOTE: if this Transform is parented, this sets the world-space position
    /// directly, but it will be overwritten the next time `resolve_parent` runs.
    /// Use `set_local_position` to move a parented Transform persistently.
    pub fn set_position(&mut self, new_pos: Vec2) {
        self.position = new_pos;
    }

    /// Sets the scale directly
    ///
    /// NOTE: see the caveat on `set_position` - the same applies while parented.
    pub fn set_scale(&mut self, new_scale: Vec2) {
        self.scale = new_scale;
    }

    /// Sets the rotation directly, in degrees
    ///
    /// NOTE: see the caveat on `set_position` - the same applies while parented.
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

    // ---------------------------------------------------------------
    // Parenting
    // ---------------------------------------------------------------

    /// Parents this Transform to `parent_id`, choosing the local offset so that this
    /// Transform's current world-space pose is preserved (it won't visually jump).
    ///
    /// `parent_position`/`parent_rotation`/`parent_scale` must be the parent's
    /// CURRENT world-space pose. Since this needs read access to the parent's
    /// Transform and write access to this one, fetch the parent immutably first,
    /// copy out its pose (Vec2/f32 are Copy), then fetch this Transform mutably:
    ///
    /// ```ignore
    /// let parent_pose = world.get_component::<Transform>(parent_id)
    ///     .map(|p| (*p.position(), p.rotation(), *p.scale()));
    /// if let Some((pos, rot, scale)) = parent_pose {
    ///     let child: &mut Transform = world.get_component_mut(child_id).unwrap();
    ///     child.set_parent(parent_id, pos, rot, scale);
    /// }
    /// ```
    pub fn set_parent(&mut self, parent_id: EntityID, parent_position: Vec2, parent_rotation: f32, parent_scale: Vec2) {
        let rot_offset = self.rotation - parent_rotation;
        let unrotated = rotate_vec2(self.position - parent_position, -parent_rotation);
        let pos_offset = Vec2::new(unrotated.x / parent_scale.x, unrotated.y / parent_scale.y);
        let scale_offset = Vec2::new(self.scale.x / parent_scale.x, self.scale.y / parent_scale.y);

        self.parent = Some(TransformParentingInfo {
            id: parent_id,
            pos_local: pos_offset,
            rot_local: rot_offset,
            scale_local: scale_offset,
        });
    }

    /// Parents this Transform to `parent_id` using explicit local offsets, without
    /// needing to know the parent's current pose (e.g. when spawning an Entity that's
    /// parented from the start).
    pub fn set_parent_with_local(&mut self, parent_id: EntityID, pos_offset: Vec2, rot_offset: f32, scale_offset: Vec2) {
        self.parent = Some(TransformParentingInfo { id: parent_id, pos_local: pos_offset, rot_local: rot_offset, scale_local: scale_offset });
    }

    /// Removes parenting. Position/rotation/scale keep whatever world-space values
    /// they last resolved to, so nothing jumps.
    pub fn clear_parent(&mut self) {
        self.parent = None;
    }

    /// Returns the parent Entity's ID, if any.
    pub fn parent_id(&self) -> Option<EntityID> {
        self.parent.as_ref().map(|p| p.id)
    }

    pub fn is_parented(&self) -> bool {
        self.parent.is_some()
    }

    /// Returns the local position offset relative to the parent, if parented.
    pub fn local_position(&self) -> Option<Vec2> {
        self.parent.as_ref().map(|p| p.pos_local)
    }
    /// Returns the local rotation offset relative to the parent, in degrees, if parented.
    pub fn local_rotation(&self) -> Option<f32> {
        self.parent.as_ref().map(|p| p.rot_local)
    }
    /// Returns the local scale offset relative to the parent, if parented.
    pub fn local_scale(&self) -> Option<Vec2> {
        self.parent.as_ref().map(|p| p.scale_local)
    }

    /// Sets the local position offset. No-op if this Transform isn't parented.
    pub fn set_local_position(&mut self, pos_offset: Vec2) {
        if let Some(p) = &mut self.parent { p.pos_local = pos_offset; }
    }
    /// Translates the local position offset by `vec`. No-op if this Transform isn't parented.
    pub fn translate_local(&mut self, vec: Vec2) {
        if let Some(p) = &mut self.parent { p.pos_local += vec; }
    }
    /// Sets the local rotation offset, in degrees. No-op if this Transform isn't parented.
    pub fn set_local_rotation(&mut self, rot_offset: f32) {
        if let Some(p) = &mut self.parent { p.rot_local = rot_offset; }
    }
    /// Rotates the local rotation offset by `angle` degrees. No-op if this Transform isn't parented.
    pub fn rotate_local(&mut self, angle: f32) {
        if let Some(p) = &mut self.parent { p.rot_local += angle; }
    }
    /// Sets the local scale offset. No-op if this Transform isn't parented.
    pub fn set_local_scale(&mut self, scale_offset: Vec2) {
        if let Some(p) = &mut self.parent { p.scale_local = scale_offset; }
    }

    /// Recomputes world-space `position`/`rotation`/`scale` from the parent's current
    /// world-space pose plus this Transform's local offset. No-ops if unparented.
    ///
    /// Call once per fixed_update for every parented Transform, after collision resolution,
    /// before the next snapshot() call.
    pub fn resolve_parent(&mut self, parent_position: Vec2, parent_rotation: f32, parent_scale: Vec2) {
        let Some(info) = &self.parent else { return; };

        self.rotation = parent_rotation + info.rot_local;
        self.scale = parent_scale * info.scale_local;
        self.position = parent_position + rotate_vec2(info.pos_local * parent_scale, parent_rotation);
    }

    /// Moves this Transform by `vec`, choosing the right frame automatically: if
    /// parented, offsets the LOCAL position (relative to the parent, so e.g. WASD
    /// input keeps behaving the same way whether or not the Entity is currently
    /// riding a platform); if unparented, offsets world-space position directly.
    ///
    /// This is a convenience for gameplay/input code that doesn't want to branch on
    /// parenting state at every callsite. Code that must stay in a specific frame
    /// regardless of parenting - physics forces, collision correction, knockback -
    /// should keep calling `translate`/`translate_local` explicitly instead, since
    /// this method's effect depends on runtime state and isn't a fixed number of
    /// world units.
    pub fn move_by(&mut self, vec: Vec2) {
        if self.parent.is_some() {
            self.translate_local(vec);
        } else {
            self.translate(vec);
        }
    }

    /// See `move_by` - same rationale, for rotation.
    pub fn rotate_by(&mut self, angle: f32) {
        if self.parent.is_some() {
            self.rotate_local(angle);
        } else {
            self.rotate(angle);
        }
    }

    /// Snapshots the current position/rotation/scale as the "previous" values used for Interpolation
    /// and Extrapolation. Use when spawning in the Entity's new/factory function if any of position,
    /// size or rotation are initialized and interpolation/extrapolation are active. This makes it so
    /// there is no chance of an object spawning and being shown to slide from (0, 0) to its actual
    /// position for the first few frames.
    ///
    /// Called engine-side once per fixed_update, before dispatching the fixed_update call to all
    /// Updatable Entities.
    pub fn snapshot(&mut self) {
        self.previous_position = self.position;
        self.previous_rotation = self.rotation;
        self.previous_scale = self.scale;
    }
    
    pub fn set_display_mode(&mut self, display_mode: TransformDisplayMode) {
        self.display_mode = display_mode;
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
    
    /// Returns a normalized vector pointing in the up direction of the Transform, based on rotation.
    pub fn local_up(&self) -> Vec2 {
        let radians = self.rotation.to_radians();
        Vec2::new(-radians.sin(), radians.cos())
    }
}

/// Rotates a 2D vector by `degrees`, counter-clockwise.
fn rotate_vec2(v: Vec2, degrees: f32) -> Vec2 {
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    Vec2::new(
        v.x * cos - v.y * sin,
        v.x * sin + v.y * cos,
    )
}
use glam::Vec4;
use crate::engine::camera::CameraID;
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::rendering::post_process::PostProcessLayer;
use crate::engine::rendering::RenderItem;
use crate::engine::world::World;

pub type PassMask = u64;

pub const DEFAULT_PASS_MASK: PassMask = 1;

pub fn pass_bit(index: u32) -> PassMask {
    assert!(index < 64, "Render Pass index must be less than 64. Attempted index: {}", index);
    1u64 << index
}

///A rectangle in normalized screen space: (0, 0) is the bottom-left corner of the window,
/// (1, 1) is the top-right. Normalized coordinates mean a pass`s target automatically adapts to
/// window resizing without the user needing to recompute anything.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const FULLSCREEN: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };

    pub fn to_pixels(&self, framebuffer_width: u32, framebuffer_height: u32) -> PixelRect {
        PixelRect {
            x: (self.x * framebuffer_width as f32).round() as i32,
            y: (self.y * framebuffer_height as f32).round() as i32,
            width: (self.width * framebuffer_width as f32).round() as i32,
            height: (self.height * framebuffer_height as f32).round() as i32,
        }
    }
}

///A screen space rectangle in pixels, ready to hand to `glViewport`/`glScissor`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Where a RenderPass' output goes on screen.
///
/// Only Viewport exists as an option for now.
#[derive(Debug, Clone, Copy)]
pub enum Target {
    Viewport(Rect)
}

impl Target {
    pub fn rect(&self) -> Rect {
        match self {
            Target::Viewport(rect) => *rect,
        }
    }
}

impl Default for Target {
    fn default() -> Self {
        Target::Viewport(Rect::FULLSCREEN)
    }
}

/// Determines the depth value (`aPos.z`) written for each `RenderItem` within a pass.
pub trait OrderingRule {
    fn compute_depth(&self, item: &RenderItem, world: &World) -> f32;
}

/// Default ordering: uses each item's `z_index`.
pub struct ZIndexOrdering;

impl OrderingRule for ZIndexOrdering {
    fn compute_depth(&self, item: &RenderItem, world: &World) -> f32 {
        item.z_index as f32 / config().z_index_max as f32
    }
}

/// Orders by the entity's world-space Y position instead of z_index. Standard for top-down or
/// isometric games.
pub struct TransformYOrdering {
    pub y_min: f32,
    pub y_max: f32,
}

impl OrderingRule for TransformYOrdering {
    fn compute_depth(&self, item: &RenderItem, world: &World) -> f32 {
        let y = world
            .get_component::<Transform>(item.entity_id)
            .map(|t| t.position().y)
            .unwrap_or(self.y_min);

        let clamped = y.clamp(self.y_min, self.y_max);
        let range = (self.y_max - self.y_min).max(f32::EPSILON);

        (clamped - self.y_min) / range
    }
}

pub struct RenderPass {
    pub name: String,
    pub mask: PassMask,
    pub camera_id: CameraID,
    pub priority: i32,
    pub ordering_rule: Box<dyn OrderingRule>,
    pub enabled: bool,
    pub target: Target,
    /// If `Some`, the pass's region is cleared to this color before drawing.
    /// If `None`, whatever was already in that region (from an earlier pass)
    /// is left untouched and drawn over.
    pub clear_color: Option<Vec4>,
    /// Whether to clear the depth buffer before this pass. Should be `true` for a pass that
    /// shouldn't be occluded by whatever a previous pass left in the depth buffer (like a UI pass)
    pub clear_depth: bool,
    /// Post-processing (post-fx) layers scoped to just this pass' own content, isolated from
    /// every other pass.
    pub local_post_fx: Vec<PostProcessLayer>,
    /// Retained per-frame 'RenderItem' buffer for this pass.
    pub(crate) items: Vec<RenderItem>,
}

impl RenderPass {
    pub fn new(name: &str, mask: PassMask, camera_id: CameraID) -> Self {
        Self {
            name: name.to_owned(),
            mask,
            camera_id,
            priority: 0,
            ordering_rule: Box::new(ZIndexOrdering),
            enabled: true,
            target: Target::default(),
            clear_color: None,
            clear_depth: true,
            local_post_fx: Vec::new(),
            items: Vec::new(),
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_target(mut self, target: Target) -> Self {
        self.target = target;
        self
    }

    pub fn with_ordering_rule(mut self, ordering_rule: Box<dyn OrderingRule>) -> Self {
        self.ordering_rule = ordering_rule;
        self
    }

    pub fn with_clear_color(mut self, clear_color: Vec4) -> Self {
        self.clear_color = Some(clear_color);
        self
    }

    pub fn with_clear_depth(mut self, clear_depth: bool) -> Self {
        self.clear_depth = clear_depth;
        self
    }

    pub fn add_local_post_fx(mut self, layer: PostProcessLayer) -> Self {
        self.local_post_fx.push(layer);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn matches(&self, drawable_mask: PassMask) -> bool {
        self.mask & drawable_mask != 0
    }
    
    pub(crate) fn clear_items(&mut self) {
        self.items.clear();
    }
}
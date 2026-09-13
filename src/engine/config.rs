use std::sync::OnceLock;
use serde::Deserialize;
use std::fs;
use glam::Vec2;
use crate::engine::component::transform::TransformDisplayMode;

/// Holds all engine critical information that may be customized.
/// This is not a settings struct, and is meant to contain data that is set once at startup only.
/// Configs may be edited manually or from code by editing the resources/config.json file.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// The name of the Game, sets the window title.
    pub game_name: String,
    /// Game version
    pub game_version: String,
    /// Window size at startup
    pub window_size: (u32, u32),
    /// Default camera viewport size.
    pub logic_size: (u32, u32),
    /// Dimensions (in units) of the sectors for broad-phase collision detection.
    /// 
    /// Smaller sectors lead to less narrow-phase collider-to-collider checks, but many pairs of
    /// colliders may fall in a larger number of sectors shared among them, leading to multiple
    /// repeated narrow-phase checks.
    /// 
    /// Larger sectors lead to less narrow-phase check duplication, but causes each collider to check
    /// against more colliders, rendering broad-phase less effective at saving performance.
    pub collision_sector_size: (u32, u32),
    /// The amount of time between each fixed_update call, in seconds.
    pub fixed_delta_time: f32,
    /// Conversion rate from game units to viewport pixels, as pixels/unit.
    pub unit_pixel_size: u32,
    /// When set to true, debug features are enabled, such as Collider rendering.
    pub debug_mode: bool,
    /// The path to the vertex shader template. This is here to allow the user to change the directory
    /// structure, but NO CHANGES are allowed to the shader template.
    pub shader_template_vertex: String,
    /// The path to the fragment shader template. This is here to allow the user to change the directory
    /// structure, but NO CHANGES are allowed to the shader template.
    pub shader_template_fragment: String,
    /// The path to the default vertex shader. This is here to allow the user to change the directory
    /// structure, but NO CHANGES are allowed to the default shader.
    pub shader_default_vertex: String,
    /// The path to the default fragment shader. This is here to allow the user to change the directory
    /// structure, but NO CHANGES are allowed to the default shader.
    pub shader_default_fragment: String,
    /// The highest value for the z_index. This is used to normalize z_index for rendering.
    pub z_index_max: i32,
    /// The default value that a new Transform will assign to its display mode.
    pub default_transform_display_mode: TransformDisplayMode,
    /// The safe ratio between a collider's minimum extent and the distance it moves in a frame.
    pub collision_substep_safe_fraction: f32,
    /// The maximum amount of collision substeps.
    pub collision_substep_max: u32,
    /// The maximum amount of collision cascade resolution attempts.
    pub collision_cascade_settle_max: u32,
}

impl Config {
    pub fn screen_unit_size(&self) -> Vec2 {
        Vec2::new(
            self.logic_size.0 as f32 / self.unit_pixel_size as f32,
            self.logic_size.1 as f32 / self.unit_pixel_size as f32,
        )
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn load_config(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&json)?;

    CONFIG
        .set(config)
        .map_err(|_| "Config already initialized")?;

    Ok(())
}

pub fn config() -> &'static Config {
    CONFIG.get().expect("Config is not initialized")
}
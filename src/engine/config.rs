use std::sync::OnceLock;
use serde::Deserialize;
use std::fs;
use glam::Vec2;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub game_name: String,
    pub game_version: String,
    pub window_size: (u32, u32),
    pub logic_size: (u32, u32),
    pub collision_sector_size: (u32, u32),
    pub fixed_delta_time: f32,
    pub unit_pixel_size: u32,
    pub debug_mode: bool,
    pub shader_template_vertex: String,
    pub shader_template_fragment: String,
    pub shader_default_vertex: String,
    pub shader_default_fragment: String,
    pub z_index_max: i32,
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
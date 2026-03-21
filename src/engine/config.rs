use std::sync::OnceLock;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub game_name: String,
    pub game_version: String,
    pub window_size: (u32, u32),
    pub logic_size: (u32, u32),
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
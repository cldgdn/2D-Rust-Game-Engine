pub mod keycode;

use std::collections::{HashMap, HashSet};
use glfw::{Action, GlfwReceiver, Key};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

use keycode::KeyCode;

pub struct Input {
    pub keys_down: HashSet<Key>,
    pub keys_pressed: HashSet<Key>,
    pub keys_released: HashSet<Key>,
    profiles: HashMap<String, Profile>,
    current_profile: String,
}

impl Input {
    pub fn new() -> Result<Self, String> {
        let profiles = Self::load_profiles()
            .map_err(|e| format!("Failed to load input profiles: {}", e))?;

        Ok(Self {
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            profiles,
            current_profile: "default".to_string(),
        })
    }

    pub fn clear_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
    }

    pub fn current_profile(&self) -> &str { self.current_profile.as_str() }

    pub fn set_current_profile(&mut self, profile: &str) -> Result<(), String> {
        if let Some(_) = self.profiles.get(profile) {
            self.current_profile = String::from(profile);
            Ok(())
        }
        else {
            Err("Profile does not exist".to_string())
        }
    }

    pub fn is_action_down(&self, action: &str) -> bool {
        if let Some(profile) = self.profiles.get(&self.current_profile) {
            if let Some(keys) = profile.action_map.get(action) {
                return keys.iter().any(|k| {
                    let glfw_key: Key = (*k).into();
                    self.keys_down.contains(&glfw_key)
                });
            }
        }
        false
    }

    pub fn is_action_pressed(&self, action: &str) -> bool {
        if let Some(profile) = self.profiles.get(&self.current_profile) {
            if let Some(keys) = profile.action_map.get(action) {
                return keys.iter().any(|k| {
                    let glfw_key: Key = (*k).into();
                    self.keys_pressed.contains(&glfw_key)
                });
            }
        }
        false
    }

    pub fn is_action_released(&self, action: &str) -> bool {
        if let Some(profile) = self.profiles.get(&self.current_profile) {
            if let Some(keys) = profile.action_map.get(action) {
                return keys.iter().any(|k| {
                    let glfw_key: Key = (*k).into();
                    self.keys_released.contains(&glfw_key)
                });
            }
        }
        false
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_released(&self, key: Key) -> bool {
        self.keys_released.contains(&key)
    }

    fn load_profiles() -> io::Result<HashMap<String, Profile>> {
        let mut profiles = HashMap::new();

        for entry in fs::read_dir("resources/input_profiles")? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid file name"))?
                .to_string();

            let profile = Profile::load(&path.to_str().unwrap())?;

            profiles.insert(name, profile);
        }

        if !profiles.contains_key("default") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing required profile: default.json"
            ));
        }

        Ok(profiles)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Profile {
    action_map: HashMap<String, Vec<KeyCode>>,
}

impl Profile {
    pub fn load(path: &str) -> io::Result<Self> {
        let data = fs::read_to_string(&path)?;
        let profile: Self = serde_json::from_str(&data)?;

        Ok(profile)
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        let data = serde_json::to_string_pretty(&self)?;
        fs::write(path, data)?;

        Ok(())
    }
}
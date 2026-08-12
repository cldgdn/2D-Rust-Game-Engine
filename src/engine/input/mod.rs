pub mod keycode;

use std::collections::{HashMap, HashSet};
use glfw::{GamepadAxis, GamepadButton, GamepadState, Key, MouseButton};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use glam::Vec2;
use keycode::{AxisCode};
use crate::engine::config::config;
use crate::engine::input::keycode::InputCode;


/// Result of an action query. `active` mirrors what the old `bool` return
/// used to mean; `intensity` is 1.0 for any digital input (key/mouse/pad
/// button) and the real analog value (0.0-1.0) for axis inputs. When an
/// action is bound to several inputs, `intensity` is the highest reported
/// by any of the currently-active ones.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ActionState {
    pub active: bool,
    pub intensity: f32,
}

/// Which of the three digital "phases" (down/pressed/released) an action
/// query is asking about. Used internally to pick the right set for each
/// input kind without triplicating the matching logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionPhase {
    Down,
    Pressed,
    Released,
}

pub struct Input {
    pub keys_down: HashSet<Key>,
    pub keys_pressed: HashSet<Key>,
    pub keys_released: HashSet<Key>,

    pub mouse_down: HashSet<MouseButton>,
    pub mouse_pressed: HashSet<MouseButton>,
    pub mouse_released: HashSet<MouseButton>,

    pub gamepad_down: HashSet<GamepadButton>,
    pub gamepad_pressed: HashSet<GamepadButton>,
    pub gamepad_released: HashSet<GamepadButton>,

    /// Current intensity (0.0-1.0) of every axis, updated wholesale each
    /// frame. Unlike the digital sets, this isn't cleared in `clear_frame` -
    /// it always reflects the latest hardware reading.
    pub axis_intensity: HashMap<AxisCode, f32>,
    /// Axes currently past the deadzone.
    pub axis_down: HashSet<AxisCode>,
    /// Axes that crossed *into* the deadzone-exceeding range this frame.
    pub axis_pressed: HashSet<AxisCode>,
    /// Axes that crossed *out of* the deadzone-exceeding range this frame.
    pub axis_released: HashSet<AxisCode>,

    pub mouse_pos: Vec2,
    pub window_size: Vec2,

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

            mouse_down: HashSet::new(),
            mouse_pressed: HashSet::new(),
            mouse_released: HashSet::new(),

            gamepad_down: HashSet::new(),
            gamepad_pressed: HashSet::new(),
            gamepad_released: HashSet::new(),

            axis_intensity: HashMap::new(),
            axis_down: HashSet::new(),
            axis_pressed: HashSet::new(),
            axis_released: HashSet::new(),

            mouse_pos: Vec2::ZERO,
            window_size: Vec2::new(
                config().window_size.0 as f32,
                config().window_size.1 as f32,
            ),

            profiles,
            current_profile: "default".to_string(),
        })
    }

    pub fn clear_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();

        self.mouse_pressed.clear();
        self.mouse_released.clear();

        self.gamepad_pressed.clear();
        self.gamepad_released.clear();

        self.axis_pressed.clear();
        self.axis_released.clear();
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

    pub fn is_action_down(&self, action: &str) -> ActionState {
        self.query_action(action, ActionPhase::Down)
    }

    pub fn is_action_pressed(&self, action: &str) -> ActionState {
        self.query_action(action, ActionPhase::Pressed)
    }

    pub fn is_action_released(&self, action: &str) -> ActionState {
        self.query_action(action, ActionPhase::Released)
    }

    fn query_action(&self, action: &str, phase: ActionPhase) -> ActionState {
        let mut result = ActionState::default();

        let Some(profile) = self.profiles.get(&self.current_profile) else {
            return result;
        };
        let Some(inputs) = profile.action_map.get(action) else {
            return result;
        };

        for input in inputs {
            let (active, intensity) = match input {
                InputCode::Key(k) => {
                    let glfw_key: Key = (*k).into();
                    let active = match phase {
                        ActionPhase::Down => self.keys_down.contains(&glfw_key),
                        ActionPhase::Pressed => self.keys_pressed.contains(&glfw_key),
                        ActionPhase::Released => self.keys_released.contains(&glfw_key),
                    };
                    (active, 1.0)
                }
                InputCode::MouseButton(b) => {
                    let glfw_button: MouseButton = (*b).into();
                    let active = match phase {
                        ActionPhase::Down => self.mouse_down.contains(&glfw_button),
                        ActionPhase::Pressed => self.mouse_pressed.contains(&glfw_button),
                        ActionPhase::Released => self.mouse_released.contains(&glfw_button),
                    };
                    (active, 1.0)
                }
                InputCode::GamepadButton(b) => {
                    let glfw_button: GamepadButton = (*b).into();
                    let active = match phase {
                        ActionPhase::Down => self.gamepad_down.contains(&glfw_button),
                        ActionPhase::Pressed => self.gamepad_pressed.contains(&glfw_button),
                        ActionPhase::Released => self.gamepad_released.contains(&glfw_button),
                    };
                    (active, 1.0)
                }
                InputCode::Axis(code) => {
                    let active = match phase {
                        ActionPhase::Down => self.axis_down.contains(code),
                        ActionPhase::Pressed => self.axis_pressed.contains(code),
                        ActionPhase::Released => self.axis_released.contains(code),
                    };
                    let intensity = self.axis_intensity.get(code).copied().unwrap_or(0.0);
                    (active, intensity)
                }
            };

            if active {
                result.active = true;
                if intensity > result.intensity {
                    result.intensity = intensity;
                }
            }
        }

        result
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

    pub fn is_mouse_down(&self, button: MouseButton) -> bool {self.mouse_down.contains(&button)}

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {self.mouse_pressed.contains(&button)}

    pub fn is_mouse_released(&self, button: MouseButton) -> bool {self.mouse_released.contains(&button)}
    
    /// Returns mouse position in screen-space, in units.
    /// 
    /// pos = (0,0) -> Bottom left of the window.
    /// pos = config().logic_size -> Top right of the window
    pub fn mouse_pos(&self) -> Vec2 {
        let mut normalized = Vec2::new(
            self.mouse_pos.x / self.window_size.x,
            self.mouse_pos.y / self.window_size.y,
        );

        normalized.y = 1.0 - normalized.y;

        Vec2::new(
            normalized.x * config().logic_size.0 as f32,
            normalized.y * config().logic_size.1 as f32,
        )
    }

    /// Feeds one axis' reading into the input system, updating both its
    /// continuous intensity and its down/pressed/released edges relative
    /// to `AXIS_DEADZONE`. `intensity` is clamped to 0.0-1.0.
    pub fn set_axis(&mut self, code: AxisCode, intensity: f32) {
        let intensity = intensity.clamp(0.0, 1.0);
        self.axis_intensity.insert(code, intensity);
        
        let profile = self.profiles.get(&self.current_profile).unwrap();
        let active = intensity > profile.action_deadzone;
        if active {
            if !self.axis_down.contains(&code) {
                self.axis_pressed.insert(code);
            }
            self.axis_down.insert(code);
        } else if self.axis_down.remove(&code) {
            self.axis_released.insert(code);
        }
    }

    /// Reads both sticks and both triggers out of a GLFW gamepad state and
    /// feeds them in as directional/trigger axes. Call this once per frame
    /// whenever a gamepad is connected.
    pub fn update_gamepad_axes(&mut self, state: &GamepadState) {
        let left_x = state.get_axis(GamepadAxis::AxisLeftX);
        let left_y = state.get_axis(GamepadAxis::AxisLeftY);
        let right_x = state.get_axis(GamepadAxis::AxisRightX);
        let right_y = state.get_axis(GamepadAxis::AxisRightY);

        // GLFW reports triggers in -1.0 (released) ..= 1.0 (fully pressed);
        // remap to the 0.0..=1.0 range everything else uses.
        let left_trigger = (state.get_axis(GamepadAxis::AxisLeftTrigger) + 1.0) * 0.5;
        let right_trigger = (state.get_axis(GamepadAxis::AxisRightTrigger) + 1.0) * 0.5;

        // Stick Y is conventionally -1.0 at the top, so "up" is negative.
        self.set_axis(AxisCode::LeftStickRight, left_x.max(0.0));
        self.set_axis(AxisCode::LeftStickLeft, (-left_x).max(0.0));
        self.set_axis(AxisCode::LeftStickUp, (-left_y).max(0.0));
        self.set_axis(AxisCode::LeftStickDown, left_y.max(0.0));

        self.set_axis(AxisCode::RightStickRight, right_x.max(0.0));
        self.set_axis(AxisCode::RightStickLeft, (-right_x).max(0.0));
        self.set_axis(AxisCode::RightStickUp, (-right_y).max(0.0));
        self.set_axis(AxisCode::RightStickDown, right_y.max(0.0));

        self.set_axis(AxisCode::LeftTrigger, left_trigger);
        self.set_axis(AxisCode::RightTrigger, right_trigger);
    }

    /// Zeroes every axis. Call this when no gamepad is connected so a
    /// disconnect doesn't leave stale non-zero values sitting around.
    pub fn reset_axes(&mut self) {
        for code in AxisCode::ALL {
            self.set_axis(code, 0.0);
        }
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
    action_map: HashMap<String, Vec<InputCode>>,
    action_deadzone: f32,
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
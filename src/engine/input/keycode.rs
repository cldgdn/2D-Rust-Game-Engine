use glfw::{GamepadButton, Key, MouseButton};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Copy, Debug)]
#[serde(tag = "type", content = "value")]
pub enum InputCode {
    Key(KeyCode),
    MouseButton(MouseButtonCode),
    GamepadButton(GamepadButtonCode),
    Axis(AxisCode),
}

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub enum GamepadButtonCode {
    South, // A / Cross    / B
    East,  // B / Circle   / A
    West,  // X / Square   / Y
    North, // Y / Triangle / X

    L1,
    R1,

    Select,
    Start,

    LStick,
    RStick,

    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}
#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub enum MouseButtonCode {
    Left,
    Right,
    Middle,
    Mouse4,
    Mouse5,
}

/// Analog inputs, expressed as individual directions/triggers rather than
/// raw two-axis sticks. This lets a single direction (e.g. "stick pushed
/// right") sit in an action's input list right next to keys and buttons,
/// each carrying its own 0.0-1.0 intensity.
///
/// Only the *name* of the axis is ever serialized - intensity is a runtime
/// concept (see `AxisInput`) and is never written to a profile JSON.
#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub enum AxisCode {
    LeftStickUp,
    LeftStickDown,
    LeftStickLeft,
    LeftStickRight,

    RightStickUp,
    RightStickDown,
    RightStickLeft,
    RightStickRight,

    LeftTrigger,
    RightTrigger,
}

impl AxisCode {
    pub const ALL: [AxisCode; 10] = [
        AxisCode::LeftStickUp,
        AxisCode::LeftStickDown,
        AxisCode::LeftStickLeft,
        AxisCode::LeftStickRight,
        AxisCode::RightStickUp,
        AxisCode::RightStickDown,
        AxisCode::RightStickLeft,
        AxisCode::RightStickRight,
        AxisCode::LeftTrigger,
        AxisCode::RightTrigger,
    ];
}

/// Runtime pairing of an `AxisCode` with the intensity it was read at.
/// This is *not* serialized - profiles only ever reference the `AxisCode`
/// itself (via `InputCode::Axis`); the intensity is populated each frame
/// from the actual hardware state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisInput {
    pub code: AxisCode,
    pub intensity: f32,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum KeyCode {
    A, B, C, D, E, F, G,
    H, I, J, K, L, M, N,
    O, P, Q, R, S, T, U,
    V, W, X, Y, Z,

    Space,
    Escape,
    Enter,
    Tab,
    LeftShift,
    LeftAlt,
    LeftControl,

    Left,
    Right,
    Up,
    Down,
}

impl From<GamepadButtonCode> for GamepadButton {
    fn from(button: GamepadButtonCode) -> GamepadButton {
        match button {
            GamepadButtonCode::South => GamepadButton::ButtonA,
            GamepadButtonCode::East => GamepadButton::ButtonB,
            GamepadButtonCode::West => GamepadButton::ButtonX,
            GamepadButtonCode::North => GamepadButton::ButtonY,

            GamepadButtonCode::DPadDown => GamepadButton::ButtonDpadDown,
            GamepadButtonCode::DPadLeft => GamepadButton::ButtonDpadLeft,
            GamepadButtonCode::DPadRight => GamepadButton::ButtonDpadRight,
            GamepadButtonCode::DPadUp => GamepadButton::ButtonDpadUp,

            GamepadButtonCode::R1 => GamepadButton::ButtonRightBumper,
            GamepadButtonCode::L1 => GamepadButton::ButtonLeftBumper,

            GamepadButtonCode::RStick => GamepadButton::ButtonRightThumb,
            GamepadButtonCode::LStick => GamepadButton::ButtonLeftThumb,

            GamepadButtonCode::Start => GamepadButton::ButtonStart,
            GamepadButtonCode::Select => GamepadButton::ButtonBack,
        }
    }
}

impl TryFrom<GamepadButton> for GamepadButtonCode {
    type Error = ();

    fn try_from(button: GamepadButton) -> Result<Self, Self::Error> {
        Ok(match button {
            GamepadButton::ButtonA => GamepadButtonCode::South,
            GamepadButton::ButtonB => GamepadButtonCode::East,
            GamepadButton::ButtonX => GamepadButtonCode::West,
            GamepadButton::ButtonY => GamepadButtonCode::North,

            GamepadButton::ButtonDpadDown => GamepadButtonCode::DPadDown,
            GamepadButton::ButtonDpadLeft => GamepadButtonCode::DPadLeft,
            GamepadButton::ButtonDpadRight => GamepadButtonCode::DPadRight,
            GamepadButton::ButtonDpadUp => GamepadButtonCode::DPadUp,

            GamepadButton::ButtonRightBumper => GamepadButtonCode::R1,
            GamepadButton::ButtonLeftBumper => GamepadButtonCode::L1,

            GamepadButton::ButtonLeftThumb => GamepadButtonCode::LStick,
            GamepadButton::ButtonRightThumb => GamepadButtonCode::RStick,

            GamepadButton::ButtonStart => GamepadButtonCode::Start,
            GamepadButton::ButtonBack => GamepadButtonCode::Select,

            _ => return Err(())
        })
    }
}

impl From<MouseButtonCode> for MouseButton {
    fn from(code: MouseButtonCode) -> Self {
        match code {
            MouseButtonCode::Left => MouseButton::Button1,
            MouseButtonCode::Right => MouseButton::Button2,
            MouseButtonCode::Middle => MouseButton::Button3,
            MouseButtonCode::Mouse4 => MouseButton::Button4,
            MouseButtonCode::Mouse5 => MouseButton::Button5,
        }
    }
}

impl TryFrom<MouseButton> for MouseButtonCode {
    type Error = ();

    fn try_from(code: MouseButton) -> Result<Self, Self::Error> {
        Ok(match code {
            MouseButton::Button1 => MouseButtonCode::Left,
            MouseButton::Button2 => MouseButtonCode::Right,
            MouseButton::Button3 => MouseButtonCode::Middle,
            MouseButton::Button4 => MouseButtonCode::Mouse4,
            MouseButton::Button5 => MouseButtonCode::Mouse5,

            _ => return Err(()),
        })
    }
}

impl From<KeyCode> for Key {
    fn from(k: KeyCode) -> Self {
        match k {
            KeyCode::A => Key::A,
            KeyCode::B => Key::B,
            KeyCode::C => Key::C,
            KeyCode::D => Key::D,
            KeyCode::E => Key::E,
            KeyCode::F => Key::F,
            KeyCode::G => Key::G,
            KeyCode::H => Key::H,
            KeyCode::I => Key::I,
            KeyCode::J => Key::J,
            KeyCode::K => Key::K,
            KeyCode::L => Key::L,
            KeyCode::M => Key::M,
            KeyCode::N => Key::N,
            KeyCode::O => Key::O,
            KeyCode::P => Key::P,
            KeyCode::Q => Key::Q,
            KeyCode::R => Key::R,
            KeyCode::S => Key::S,
            KeyCode::T => Key::T,
            KeyCode::U => Key::U,
            KeyCode::V => Key::V,
            KeyCode::W => Key::W,
            KeyCode::X => Key::X,
            KeyCode::Y => Key::Y,
            KeyCode::Z => Key::Z,

            KeyCode::Space => Key::Space,
            KeyCode::Escape => Key::Escape,
            KeyCode::Enter => Key::Enter,
            KeyCode::Tab => Key::Tab,
            KeyCode::LeftShift => Key::LeftShift,
            KeyCode::LeftAlt => Key::LeftAlt,
            KeyCode::LeftControl => Key::LeftControl,

            KeyCode::Left => Key::Left,
            KeyCode::Right => Key::Right,
            KeyCode::Up => Key::Up,
            KeyCode::Down => Key::Down,
        }
    }
}

impl TryFrom<Key> for KeyCode {
    type Error = ();

    fn try_from(k: Key) -> Result<Self, Self::Error> {
        Ok(match k {
            Key::A => KeyCode::A,
            Key::B => KeyCode::B,
            Key::C => KeyCode::C,
            Key::D => KeyCode::D,
            Key::E => KeyCode::E,
            Key::F => KeyCode::F,
            Key::G => KeyCode::G,
            Key::H => KeyCode::H,
            Key::I => KeyCode::I,
            Key::J => KeyCode::J,
            Key::K => KeyCode::K,
            Key::L => KeyCode::L,
            Key::M => KeyCode::M,
            Key::N => KeyCode::N,
            Key::O => KeyCode::O,
            Key::P => KeyCode::P,
            Key::Q => KeyCode::Q,
            Key::R => KeyCode::R,
            Key::S => KeyCode::S,
            Key::T => KeyCode::T,
            Key::U => KeyCode::U,
            Key::V => KeyCode::V,
            Key::W => KeyCode::W,
            Key::X => KeyCode::X,
            Key::Y => KeyCode::Y,
            Key::Z => KeyCode::Z,

            Key::Space => KeyCode::Space,
            Key::Escape => KeyCode::Escape,
            Key::Enter => KeyCode::Enter,
            Key::Tab => KeyCode::Tab,
            Key::LeftShift => KeyCode::LeftShift,
            Key::LeftAlt => KeyCode::LeftAlt,
            Key::LeftControl => KeyCode::LeftControl,

            Key::Left => KeyCode::Left,
            Key::Right => KeyCode::Right,
            Key::Up => KeyCode::Up,
            Key::Down => KeyCode::Down,

            _ => return Err(()), // ignore unsupported keys
        })
    }
}
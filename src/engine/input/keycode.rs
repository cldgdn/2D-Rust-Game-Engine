use glfw::Key;
use serde::{Deserialize, Serialize};

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
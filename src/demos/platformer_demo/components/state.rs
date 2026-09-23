use crate::engine::EntityID;
use glam::Vec2;

pub struct PlayerState {
    pub id: EntityID,
    pub is_on_ground: bool,
    pub was_airborne: bool,
    pub is_jumping: bool,
    pub is_against_wall: bool,
    pub is_against_ceiling: bool,
    pub direction: Vec2,
    pub ground_id: EntityID
}

impl PlayerState {
    pub fn new(id: EntityID) -> PlayerState {
        let mut ps = PlayerState::default();
        ps.id = id;
        ps
    }
}

impl Default for PlayerState {
    fn default() -> PlayerState {
        PlayerState {
            id: 0,
            is_on_ground: false,
            was_airborne: false,
            is_jumping: false,
            is_against_wall: false,
            is_against_ceiling: false,
            direction: Vec2::ZERO,
            ground_id: 0,
        }
    }
}
use crate::engine::EntityID;

pub struct SomeEvent;
pub struct EnemyDeathEvent {
    pub id: EntityID
}
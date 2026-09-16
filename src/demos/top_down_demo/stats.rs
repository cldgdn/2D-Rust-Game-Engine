

pub struct Health {
    pub max_health: f32,
    pub health: f32,
}

impl Health {
    pub fn new(max_health: f32) -> Health {
        Self {
            max_health,
            health: max_health,
        }
    }
}
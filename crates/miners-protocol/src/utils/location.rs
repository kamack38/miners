#[derive(Debug, Clone)]
pub struct Location {
    pub world: Option<String>,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Location {
    pub fn zero() -> Location {
        Location {
            world: None,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}
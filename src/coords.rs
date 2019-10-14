use serde::Deserialize;

#[derive(Debug, Default, Clone, Copy, PartialEq, Deserialize)]
pub struct Coords {
    x: f64,
    y: f64,
    z: f64,
}

impl Coords {
    pub fn dist_to(self, other: Coords) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2))
            .sqrt()
    }
}

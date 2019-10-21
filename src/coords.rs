use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct Coords {
    x: f64,
    y: f64,
    z: f64,
}

impl Coords {
    pub fn zero() -> Coords {
        Coords {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn dist_to(self, other: Coords) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2))
            .sqrt()
    }
}

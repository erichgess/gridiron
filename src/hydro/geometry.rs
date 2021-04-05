



/**
 * A 3D vector
 */
pub struct Vector3d(f64, f64, f64);




// ============================================================================
impl Vector3d {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vector3d(x, y, z)
    }
}




/**
 * Enum to hold a unit vector in 3D space
 */
#[derive(Clone, Copy)]
pub enum Direction { X, Y, Z }




// ============================================================================
impl Direction {
    pub fn along(&self, other: Direction) -> f64 {
        match (self, other) {
            (Direction::X, Direction::X) => 1.0,
            (Direction::Y, Direction::Y) => 1.0,
            (Direction::Z, Direction::Z) => 1.0,
            _ => 0.0,
        }
    }
}

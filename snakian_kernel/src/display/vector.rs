use core::ops::{Add, Mul, Sub};



/// A simple struct to represent a 2D vector.
/// Because there is a 99% chance that we wont need decimals, this is a integer vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vector{
    pub x: usize,
    pub y: usize,
}

impl Default for Vector{
    fn default() -> Self{
        Vector{
            x: 0,
            y: 0,
        }
    }
}

impl Vector {
    /// Creates a new Vector with the given x and y values.
    pub const fn new(x: usize, y: usize) -> Vector {
        Vector {
            x,
            y,
        }
    }
}

impl Add for Vector {
    type Output = Vector;

    fn add(self, other: Vector) -> Vector {
        Vector {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vector {
    type Output = Vector;

    fn sub(self, other: Vector) -> Vector {
        Vector {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul for Vector {
    type Output = Vector;

    fn mul(self, other: Vector) -> Vector {
        Vector {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}


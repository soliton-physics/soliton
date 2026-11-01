use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A plane vector. Laid out as two consecutive `f64`s so slices of `Vec2` view as flat arrays.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const X: Self = Self::new(1.0, 0.0);
    pub const Y: Self = Self::new(0.0, 1.0);

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn from_polar(length: f64, angle: f64) -> Self {
        Self::new(length * angle.cos(), length * angle.sin())
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// The z component of the 3D cross product.
    pub fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Rotated a quarter turn counter-clockwise.
    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Unit vector in the same direction, or zero for the zero vector.
    pub fn normalized(self) -> Self {
        let length = self.length();
        if length == 0.0 {
            Self::ZERO
        } else {
            self / length
        }
    }

    pub fn rotated(self, angle: f64) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, scale: f64) -> Self {
        Self::new(self.x * scale, self.y * scale)
    }
}

impl Mul<Vec2> for f64 {
    type Output = Vec2;
    fn mul(self, vector: Vec2) -> Vec2 {
        vector * self
    }
}

impl Div<f64> for Vec2 {
    type Output = Self;
    fn div(self, scale: f64) -> Self {
        Self::new(self.x / scale, self.y / scale)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl From<(f64, f64)> for Vec2 {
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}

impl From<[f64; 2]> for Vec2 {
    fn from([x, y]: [f64; 2]) -> Self {
        Self::new(x, y)
    }
}

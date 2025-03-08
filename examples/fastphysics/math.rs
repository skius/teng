use std::ops::{Add, AddAssign, Mul, MulAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn floor_to_i64(&self) -> (i64, i64) {
        (self.x.floor() as i64, self.y.floor() as i64)
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    pub fn normalized(&self) -> Self {
        let length = self.length();
        if length == 0.0 {
            Self::zero()
        } else {
            *self * (1.0 / length)
        }
    }

    pub fn with_length(&self, length: f64) -> Self {
        self.normalized() * length
    }
}

impl Into<(f64, f64)> for Vec2 {
    fn into(self) -> (f64, f64) {
        (self.x, self.y)
    }
}

impl From<(f64, f64)> for Vec2 {
    fn from((x, y): (f64, f64)) -> Self {
        Self { x, y }
    }
}

impl AddAssign<Vec2> for Vec2 {
    fn add_assign(&mut self, other: Vec2) {
        *self = *self + other;
    }
}

impl Add<Vec2> for Vec2 {
    type Output = Self;

    fn add(self, other: Vec2) -> Self::Output {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Mul<Vec2> for Vec2 {
    type Output = f64;

    fn mul(self, other: Vec2) -> Self::Output {
        self.x * other.x + self.y * other.y
    }
}

impl Mul<f64> for Vec2 {
    type Output = Self;

    fn mul(self, scalar: f64) -> Self::Output {
        Self::new(self.x * scalar, self.y * scalar)
    }
}

impl Mul<Vec2> for f64 {
    type Output = Vec2;

    fn mul(self, vec: Vec2) -> Self::Output {
        vec * self
    }
}

impl MulAssign<f64> for Vec2 {
    fn mul_assign(&mut self, scalar: f64) {
        *self = *self * scalar;
    }
}

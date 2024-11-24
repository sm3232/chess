
use eframe::egui;

#[derive(Clone,Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32
}

impl Point {
    pub fn valid(&self) -> bool {
        return self.x >= 0 && self.x < 8 && self.y >= 0 && self.y < 8;
    }
    pub fn to_index(&self) -> usize {
        return (self.y * 8 + self.x) as usize;
    }
    pub fn from_index(index: usize) -> Point {
        return Point {
            x: (index % 8) as i32,
            y: (index / 8) as i32
        };
    }
}

impl Default for Point {
    fn default() -> Self {
        return Self { x: 0, y: 0 };
    }
}

impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{:?}", self);
    }
}
impl std::fmt::Debug for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "({:#?}, {:#?})", self.x, self.y);
    }
}

impl std::ops::Add<Point> for Point {
    type Output = Self;
    fn add(self, rhs: Point) -> Self::Output {
        return Point { x: self.x + rhs.x, y: self.y + rhs.y };
    }
}
impl std::ops::Sub<Point> for Point {
    type Output = Self;
    fn sub(self, rhs: Point) -> Self::Output {
        return Point { x: self.x - rhs.x, y: self.y - rhs.y };
    }
}

impl std::ops::Mul<f32> for Point {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        return Point { x: ((self.x as f32) * rhs).floor() as i32, y: ((self.y as f32) * rhs).floor() as i32 };
    }
}
impl std::ops::Mul<i32> for Point {
    type Output = Self;
    fn mul(self, rhs: i32) -> Self::Output {
        return Point { x: self.x * rhs, y: self.y * rhs };
    }
}


impl std::hash::Hash for Point {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

impl std::cmp::PartialEq<Point> for Point {
    fn eq(&self, other: &Point) -> bool {
        return self.x == other.x && self.y == other.y;
    }
}
impl std::cmp::Eq for Point {}

impl Into<egui::Pos2> for Point {
    fn into(self) -> egui::Pos2 {
        return egui::Pos2 {
            x: self.x as f32,
            y: self.y as f32
        }
    }
}
impl From<egui::Pos2> for Point {
    fn from(value: egui::Pos2) -> Self {
        let v = value.floor();
        return Self { x: v.x as i32, y: v.y as i32 };
    }
}


pub fn point(xv: i32, yv: i32) -> Point {
    return Point { x: xv, y: yv };
}

pub fn algebraic_to_point(alg: &str) -> Point {
    return Point {
        x: alg.chars().nth(0).unwrap_or('a') as i32 - 97,
        y: (alg.chars().nth(1).unwrap_or('1').to_digit(10).unwrap_or(1) as i32)
    };
}

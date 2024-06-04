use std::ops::Add;

use geo::{HaversineDestination, HaversineDistance, HaversineIntermediate, Point};

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    max: Point,
    min: Point,
}

impl Bounds {
    pub fn new(x: f64, y: f64, r: f64) -> Self {
        let c = Point::new(x, y);
        let max = c.haversine_destination(45.0, r);
        let min = c.haversine_destination(45.0 + 180.0, r);
        Self { max, min }
    }

    pub fn x_y_r(self) -> (f64, f64, f64) {
        if self.max == self.min {
            let (x, y) = self.max.x_y();
            (x, y, 0.0)
        } else {
            let c = self.max.haversine_intermediate(&self.min, 0.5);
            let r = self.max.haversine_distance(&c);
            let (x, y) = c.x_y();
            (x, y, r)
        }
    }
}

impl Add<(f64, f64)> for Bounds {
    type Output = Self;

    fn add(mut self, (x, y): (f64, f64)) -> Self {
        if x > self.max.x() {
            self.max.set_x(x);
        } else if x < self.min.x() {
            self.min.set_x(x);
        }
        if y > self.max.y() {
            self.max.set_y(y);
        } else if y < self.min.y() {
            self.min.set_y(y);
        }
        self
    }
}

impl Add<Point> for Bounds {
    type Output = Self;

    fn add(self, other: Point) -> Self {
        self + other.x_y()
    }
}

impl Add for Bounds {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        if other.max.x() > self.max.x() {
            self.max.set_x(other.max.x());
        } else if other.min.x() < self.min.x() {
            self.min.set_x(other.min.x());
        }
        if other.max.y() > self.max.y() {
            self.max.set_y(other.max.y());
        } else if other.min.y() < self.min.y() {
            self.min.set_y(other.min.y());
        }
        self
    }
}

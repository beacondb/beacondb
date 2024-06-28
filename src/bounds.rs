use std::ops::Add;

use geo::Point;

// TODO: refactor, this doesn't need to be dependent on the geo library
// DbBounds should be the same as Bounds

pub struct DbBounds {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

impl From<DbBounds> for Bounds {
    fn from(value: DbBounds) -> Self {
        let min = Point::new(value.min_lon, value.min_lat);
        let max = Point::new(value.max_lon, value.max_lat);
        Self { min, max }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    min: Point,
    max: Point,
}

impl Bounds {
    pub fn new(max: Point, min: Point) -> Self {
        Self { max, min }
    }

    pub fn empty(x: Point) -> Self {
        Self { max: x, min: x }
    }

    // TODO: refactor as above, make fields public
    pub fn values(&self) -> (f64, f64, f64, f64) {
        let (min_lon, min_lat) = self.min.x_y();
        let (max_lon, max_lat) = self.max.x_y();
        (min_lon, min_lat, max_lon, max_lat)
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

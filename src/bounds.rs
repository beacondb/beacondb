//! A module to handle geospatial bounding boxes and basic operations.

use std::ops::Add;

use geo::Point;

/// A geospatial bounding box
///
/// This struct represents a geospatial [minimal bounding rectangle](https://en.wikipedia.org/wiki/Minimum_bounding_rectangle).
#[derive(Clone, Copy)]
pub struct Bounds {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

impl Bounds {
    /// Create a new `Bounds` struct around a single point.
    pub fn new(lat: f64, lon: f64) -> Self {
        Self {
            min_lat: lat,
            min_lon: lon,
            max_lat: lat,
            max_lon: lon,
        }
    }

    /// Return the bottom left and the top right point of the rectangle.
    pub fn points(&self) -> (Point, Point) {
        let min = Point::new(self.min_lon, self.min_lat);
        let max = Point::new(self.max_lon, self.max_lat);
        (min, max)
    }
}

impl Add<(f64, f64)> for Bounds {
    type Output = Self;

    /// Union of two bounds.
    fn add(mut self, (lat, lon): (f64, f64)) -> Self {
        if lat < self.min_lat {
            self.min_lat = lat;
        } else if lat > self.max_lat {
            self.max_lat = lat;
        }

        if lon < self.min_lon {
            self.min_lon = lon;
        } else if lon > self.max_lon {
            self.max_lon = lon;
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_check() {
        let b = Bounds::new(0.0, 0.0);

        let b = b + (0.1, 0.1);
        assert!(b.max_lat > 0.0);
        assert!(b.max_lon > 0.0);
        assert!(b.min_lat < 0.1);
        assert!(b.min_lon < 0.1);

        let b = b + (-0.1, -0.1);
        assert!(b.max_lat > 0.0);
        assert!(b.max_lon > 0.0);
        assert!(b.min_lat < 0.0);
        assert!(b.min_lon < 0.0);
    }
}

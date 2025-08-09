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
    /// Return the bottom left and the top right point of the rectangle.
    pub fn points(&self) -> (Point, Point) {
        let min = Point::new(self.min_lon, self.min_lat);
        let max = Point::new(self.max_lon, self.max_lat);
        (min, max)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TransmitterLocation {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,

    pub lat: f64,
    pub lon: f64,
    pub accuracy: f64,
    pub total_weight: f64,
}

impl TransmitterLocation {
    /// Create a new `TransmitterLocation` struct around a single point.
    pub fn new(lat: f64, lon: f64, accuracy: f64, weight: f64) -> Self {
        Self {
            min_lat: lat,
            min_lon: lon,
            max_lat: lat,
            max_lon: lon,

            lat: lat,
            lon: lon,
            accuracy: accuracy,
            total_weight: weight,
        }
    }

    /// Return the bottom left and the top right point of the rectangle.
    pub fn points(&self) -> (Point, Point) {
        let min = Point::new(self.min_lon, self.min_lat);
        let max = Point::new(self.max_lon, self.max_lat);
        (min, max)
    }

    /// Add new data to the weighted average
    pub fn update(mut self, lat: f64, lon: f64, accuracy: f64, weight: f64) -> Self {
        // TODO: Add tests
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

        self.lat = ((self.lat * self.total_weight) + (lat * weight)) / (self.total_weight + weight);
        self.lon = ((self.lon * self.total_weight) + (lon * weight)) / (self.total_weight + weight);
        self.accuracy = ((self.accuracy * self.total_weight) + (accuracy * weight)) / (self.total_weight + weight);

        self.total_weight += weight;

        self
    }
}

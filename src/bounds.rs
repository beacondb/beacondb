//! A module to handle geospatial bounding boxes and basic operations.

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
        self.accuracy = ((self.accuracy * self.total_weight) + (accuracy * weight))
            / (self.total_weight + weight);

        self.total_weight += weight;

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transmitter_location_update() {
        // Values were chosen so all floats are rounds, to be easier to test
        let location = TransmitterLocation::new(0.0, 0.0, 20.0, 1.0);
        let location = location.update(1.8, 0.9, 5.0, 2.0);

        assert_eq!(location.max_lat, 1.8);
        assert_eq!(location.max_lon, 0.9);
        assert_eq!(location.min_lat, 0.0);
        assert_eq!(location.min_lon, 0.0);
        assert_eq!(location.lat, 1.2);
        assert_eq!(location.lon, 0.6);
        assert_eq!(location.accuracy, 10.0);
        assert_eq!(location.total_weight, 3.0);

        let location = location.update(-7.2, -4.5, 5.0, 2.0);

        assert_eq!(location.max_lat, 1.8);
        assert_eq!(location.max_lon, 0.9);
        assert_eq!(location.min_lat, -7.2);
        assert_eq!(location.min_lon, -4.5);
        assert_eq!(location.lat, -2.16);
        assert_eq!(location.lon, -1.44);
        assert_eq!(location.accuracy, 8.0);
        assert_eq!(location.total_weight, 5.0);
    }
}

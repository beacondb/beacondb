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

    pub fn add(&mut self, x: f64, y: f64) -> bool {
        let mut expanded = false;
        if x > self.max.x() {
            self.max.set_x(x);
            expanded = true;
        } else if x < self.min.x() {
            self.min.set_x(x);
            expanded = true;
        }
        if y > self.max.y() {
            self.max.set_y(y);
            expanded = true;
        } else if y < self.min.y() {
            self.min.set_y(y);
            expanded = true;
        }
        expanded
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanded() {
        let mut b = Bounds::new(0.0, 0.0, 50.0);
        assert!(b.add(0.1, 0.0));
        assert!(!b.add(0.1, 0.0));
        assert!(b.add(0.0, 0.1));
    }
}

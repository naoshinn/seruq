#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    x: f32,
    y: f32,
}

impl Point {
    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn new(x: f32, y: f32) -> Result<Self, GeometryError> {
        if x.is_finite() && y.is_finite() {
            Ok(Self { x, y })
        } else {
            Err(GeometryError::NonFinite)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn new(width: f32, height: f32) -> Result<Self, GeometryError> {
        let is_finite = width.is_finite() && height.is_finite();
        let is_non_negative = width >= 0.0 && height >= 0.0;

        if is_finite && is_non_negative {
            Ok(Self { width, height })
        } else if !is_finite {
            Err(GeometryError::NonFinite)
        } else {
            Err(GeometryError::Negative)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryError {
    NonFinite,
    Negative,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_matches;

    #[test]
    fn point_preserves_coordinates() {
        for (x, y) in [(100.0, 200.0), (0.25, 0.5)] {
            let point = Point::new(x, y).unwrap();
            assert_eq!(point.x(), x);
            assert_eq!(point.y(), y);
        }
    }

    #[test]
    fn point_accepts_negative_coordinates() {
        for (x, y) in [(-100.0, 200.0), (100.0, -200.0), (-100.0, -200.0)] {
            let point = Point::new(x, y).unwrap();
            assert_eq!(point.x(), x);
            assert_eq!(point.y(), y);
        }
    }

    #[test]
    fn point_accepts_zero_coordinates() {
        for (x, y) in [(0.0, 200.0), (100.0, 0.0), (0.0, 0.0)] {
            let point = Point::new(x, y).unwrap();
            assert_eq!(point.x(), x);
            assert_eq!(point.y(), y);
        }
    }

    #[test]
    fn point_rejects_non_finite_x() {
        for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_matches!(Point::new(x, 200.0), Err(GeometryError::NonFinite));
        }
    }

    #[test]
    fn point_rejects_non_finite_y() {
        for y in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_matches!(Point::new(100.0, y), Err(GeometryError::NonFinite));
        }
    }

    #[test]
    fn size_preserves_dimensions() {
        for (width, height) in [(100.0, 200.0), (0.25, 0.5)] {
            let size = Size::new(width, height).unwrap();
            assert_eq!(size.width(), width);
            assert_eq!(size.height(), height);
        }
    }

    #[test]
    fn size_accepts_zero_dimensions() {
        for (width, height) in [(0.0, 200.0), (100.0, 0.0), (0.0, 0.0)] {
            let size = Size::new(width, height).unwrap();
            assert_eq!(size.width(), width);
            assert_eq!(size.height(), height);
        }
    }

    #[test]
    fn size_rejects_negative_width() {
        for width in [-1.0, -f32::EPSILON, f32::MIN] {
            assert_matches!(Size::new(width, 200.0), Err(GeometryError::Negative));
        }
    }

    #[test]
    fn size_rejects_negative_height() {
        for height in [-1.0, -f32::EPSILON, f32::MIN] {
            assert_matches!(Size::new(100.0, height), Err(GeometryError::Negative));
        }
    }

    #[test]
    fn size_rejects_non_finite_width() {
        for width in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_matches!(Size::new(width, 200.0), Err(GeometryError::NonFinite));
        }
    }

    #[test]
    fn size_rejects_non_finite_height() {
        for height in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_matches!(Size::new(100.0, height), Err(GeometryError::NonFinite));
        }
    }

    #[test]
    fn size_prioritizes_non_finite_errors_over_negative_dimensions() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_matches!(Size::new(value, -1.0), Err(GeometryError::NonFinite));
            assert_matches!(Size::new(-1.0, value), Err(GeometryError::NonFinite));
        }
    }
}

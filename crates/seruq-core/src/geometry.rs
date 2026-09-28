use std::{error::Error, fmt};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point {
    x: f32,
    y: f32,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Size {
    width: f32,
    height: f32,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Rect {
    origin: Point,
    size: Size,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PhysicalSize {
    width: u32,
    height: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum GeometryError {
    NonFinite,
    NegativeSize,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => formatter.write_str("coordinate is not finite"),
            Self::NegativeSize => formatter.write_str("size dimension is negative"),
        }
    }
}

impl Error for GeometryError {}

impl Point {
    pub fn new(x: f32, y: f32) -> Result<Self, GeometryError> {
        if !x.is_finite() || !y.is_finite() {
            Err(GeometryError::NonFinite)
        } else {
            Ok(Self { x, y })
        }
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }
}

impl Size {
    pub fn new(width: f32, height: f32) -> Result<Self, GeometryError> {
        if !width.is_finite() || !height.is_finite() {
            Err(GeometryError::NonFinite)
        } else if width < 0.0 || height < 0.0 {
            Err(GeometryError::NegativeSize)
        } else {
            Ok(Self { width, height })
        }
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

impl Rect {
    pub fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    pub fn origin(&self) -> Point {
        self.origin
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size.is_empty()
    }
}

impl PhysicalSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width() == 0 || self.height() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod point {
        use super::*;

        #[test]
        fn accepts_finite_coordinates() {
            assert!(Point::new(100.0, 200.0).is_ok());
        }

        #[test]
        fn accepts_negative_values() {
            assert!(Point::new(-100.0, -200.0).is_ok());
        }

        #[test]
        fn exposes_coordinates() {
            let point = Point::new(-100.0, 200.0).unwrap();

            assert_eq!(point.x(), -100.0);
            assert_eq!(point.y(), 200.0);
        }

        #[test]
        fn rejects_non_finite_x() {
            for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                assert!(matches!(
                    Point::new(x, 200.0),
                    Err(GeometryError::NonFinite)
                ));
            }
        }

        #[test]
        fn rejects_non_finite_y() {
            for y in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                assert!(matches!(
                    Point::new(100.0, y),
                    Err(GeometryError::NonFinite)
                ));
            }
        }
    }

    mod size {
        use super::*;

        #[test]
        fn accepts_non_negative_finite_values() {
            assert!(Size::new(100.0, 200.0).is_ok());
        }

        #[test]
        fn accepts_zero_dimensions() {
            assert!(Size::new(0.0, 0.0).is_ok());
        }

        #[test]
        fn exposes_dimensions() {
            let size = Size::new(100.0, 200.0).unwrap();

            assert_eq!(size.width(), 100.0);
            assert_eq!(size.height(), 200.0);
        }

        #[test]
        fn is_empty_when_either_dimension_is_zero() {
            assert!(Size::new(0.0, 0.0).unwrap().is_empty());
            assert!(Size::new(0.0, 100.0).unwrap().is_empty());
            assert!(Size::new(100.0, 0.0).unwrap().is_empty());
            assert!(!Size::new(100.0, 100.0).unwrap().is_empty());
        }

        #[test]
        fn rejects_negative_width() {
            assert!(matches!(
                Size::new(-1.0, 100.0),
                Err(GeometryError::NegativeSize)
            ));
        }

        #[test]
        fn rejects_negative_height() {
            assert!(matches!(
                Size::new(100.0, -1.0),
                Err(GeometryError::NegativeSize)
            ));
        }

        #[test]
        fn rejects_non_finite_width() {
            for width in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                assert!(matches!(
                    Size::new(width, 100.0),
                    Err(GeometryError::NonFinite)
                ));
            }
        }

        #[test]
        fn rejects_non_finite_height() {
            for height in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                assert!(matches!(
                    Size::new(100.0, height),
                    Err(GeometryError::NonFinite)
                ));
            }
        }
    }

    mod rect {
        use super::*;

        #[test]
        fn exposes_origin_and_size() {
            let origin = Point::new(-100.0, 200.0).unwrap();
            let size = Size::new(300.0, 400.0).unwrap();
            let rect = Rect::new(origin, size);

            assert_eq!(rect.origin().x(), -100.0);
            assert_eq!(rect.origin().y(), 200.0);
            assert_eq!(rect.size().width(), 300.0);
            assert_eq!(rect.size().height(), 400.0);
        }

        #[test]
        fn is_empty_follows_its_size() {
            let origin = Point::new(10.0, 20.0).unwrap();

            assert!(Rect::new(origin, Size::new(0.0, 400.0).unwrap()).is_empty());
            assert!(!Rect::new(origin, Size::new(300.0, 400.0).unwrap()).is_empty());
        }
    }

    mod physical_size {
        use super::*;

        #[test]
        fn exposes_dimensions() {
            let size = PhysicalSize::new(100, 200);

            assert_eq!(size.width(), 100);
            assert_eq!(size.height(), 200);
        }

        #[test]
        fn is_empty_when_either_dimension_is_zero() {
            assert!(PhysicalSize::new(0, 0).is_empty());
            assert!(PhysicalSize::new(0, 100).is_empty());
            assert!(PhysicalSize::new(100, 0).is_empty());
            assert!(!PhysicalSize::new(100, 100).is_empty());
        }
    }
}

use std::{error::Error, fmt};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum ColorError {
    NonFinite,
    OutOfRange,
}

impl fmt::Display for ColorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => formatter.write_str("color channel is not finite"),
            Self::OutOfRange => formatter.write_str("color channel is outside 0.0..=1.0"),
        }
    }
}

impl Error for ColorError {}

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Result<Self, ColorError> {
        if !r.is_finite() || !g.is_finite() || !b.is_finite() {
            Err(ColorError::NonFinite)
        } else if !(0.0..=1.0).contains(&r)
            || !(0.0..=1.0).contains(&g)
            || !(0.0..=1.0).contains(&b)
        {
            Err(ColorError::OutOfRange)
        } else {
            Ok(Color { r, g, b })
        }
    }

    pub fn r(&self) -> f32 {
        self.r
    }

    pub fn g(&self) -> f32 {
        self.g
    }

    pub fn b(&self) -> f32 {
        self.b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_inclusive_range() {
        assert!(Color::new(0.0, 0.0, 0.0).is_ok());
        assert!(Color::new(1.0, 1.0, 1.0).is_ok());
    }

    #[test]
    fn exposes_channels() {
        let color = Color::new(0.1, 0.5, 0.9).unwrap();

        assert_eq!(color.r(), 0.1);
        assert_eq!(color.g(), 0.5);
        assert_eq!(color.b(), 0.9);
    }

    #[test]
    fn rejects_out_of_range_red() {
        for red in [-0.1, 1.1] {
            assert!(matches!(
                Color::new(red, 0.5, 0.5),
                Err(ColorError::OutOfRange)
            ));
        }
    }

    #[test]
    fn rejects_out_of_range_green() {
        for green in [-0.1, 1.1] {
            assert!(matches!(
                Color::new(0.5, green, 0.5),
                Err(ColorError::OutOfRange)
            ));
        }
    }

    #[test]
    fn rejects_out_of_range_blue() {
        for blue in [-0.1, 1.1] {
            assert!(matches!(
                Color::new(0.5, 0.5, blue),
                Err(ColorError::OutOfRange)
            ));
        }
    }

    #[test]
    fn rejects_non_finite_red() {
        for red in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(matches!(
                Color::new(red, 0.5, 0.5),
                Err(ColorError::NonFinite)
            ));
        }
    }

    #[test]
    fn rejects_non_finite_green() {
        for green in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(matches!(
                Color::new(0.5, green, 0.5),
                Err(ColorError::NonFinite)
            ));
        }
    }

    #[test]
    fn rejects_non_finite_blue() {
        for blue in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(matches!(
                Color::new(0.5, 0.5, blue),
                Err(ColorError::NonFinite)
            ));
        }
    }

    #[test]
    fn errors_are_displayable() {
        assert_eq!(
            ColorError::NonFinite.to_string(),
            "color channel is not finite"
        );
        assert_eq!(
            ColorError::OutOfRange.to_string(),
            "color channel is outside 0.0..=1.0"
        );
    }
}

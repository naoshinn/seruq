#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontSize(f32);

impl FontSize {
    pub fn get(&self) -> f32 {
        self.0
    }

    pub fn new(value: f32) -> Result<Self, FontSizeError> {
        if !value.is_finite() {
            Err(FontSizeError::NonFinite)
        } else if value <= 0.0 {
            Err(FontSizeError::NonPositive)
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSizeError {
    NonFinite,
    NonPositive,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_matches;

    #[test]
    fn font_size_preserves_positive_finite_values() {
        for value in [f32::MIN_POSITIVE, 0.5, 16.0, f32::MAX] {
            assert_eq!(FontSize::new(value).unwrap().get(), value);
        }
    }

    #[test]
    fn font_size_rejects_zero_and_negative_values() {
        for value in [0.0, -0.0, -f32::EPSILON, -1.0, f32::MIN] {
            assert_matches!(FontSize::new(value), Err(FontSizeError::NonPositive));
        }
    }

    #[test]
    fn font_size_rejects_non_finite_values() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_matches!(FontSize::new(value), Err(FontSizeError::NonFinite));
        }
    }
}

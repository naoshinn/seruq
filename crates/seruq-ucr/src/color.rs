use std::{fmt, str::FromStr};

/// An opaque color with 8-bit sRGB channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "#{:02x}{:02x}{:02x}",
            self.red, self.green, self.blue
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorError;

impl fmt::Display for ColorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("expected an sRGB color in #RRGGBB format")
    }
}

impl std::error::Error for ColorError {}

impl FromStr for Color {
    type Err = ColorError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bytes = text.as_bytes();
        if bytes.len() != 7 || bytes[0] != b'#' || !bytes[1..].iter().all(u8::is_ascii_hexdigit) {
            return Err(ColorError);
        }
        Ok(Self::rgb(
            u8::from_str_radix(&text[1..3], 16).map_err(|_| ColorError)?,
            u8::from_str_radix(&text[3..5], 16).map_err(|_| ColorError)?,
            u8::from_str_radix(&text[5..7], 16).map_err(|_| ColorError)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexadecimal_colors_round_trip_in_lowercase() {
        for (text, expected) in [
            ("#000000", Color::BLACK),
            ("#ffffff", Color::WHITE),
            ("#12AbEF", Color::rgb(18, 171, 239)),
        ] {
            let color = text.parse::<Color>().unwrap();
            assert_eq!(color, expected);
            assert_eq!(color.to_string(), text.to_ascii_lowercase());
        }
    }

    #[test]
    fn rejects_other_formats_and_non_ascii_input() {
        for text in [
            "",
            "red",
            "#fff",
            "#ffffffff",
            "123456",
            " #123456",
            "#123456\n",
            "#gg0000",
            "#é0000",
        ] {
            assert_eq!(text.parse::<Color>(), Err(ColorError));
        }
    }
}

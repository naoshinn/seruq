/// A relative asset reference using `/` separators.
///
/// The path cannot leave the asset base supplied by the application.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPath(String);

impl AssetPath {
    pub fn new(path: impl Into<String>) -> Result<Self, AssetPathError> {
        let path = path.into();

        if path.is_empty() {
            Err(AssetPathError::Empty)
        } else if path.contains(|c: char| c == '\\' || c == ':' || c.is_control()) {
            Err(AssetPathError::InvalidCharacter)
        } else if path.starts_with('/') {
            Err(AssetPathError::Absolute)
        } else if path
            .split('/')
            .any(|segment| matches!(segment, "" | "." | ".."))
        {
            Err(AssetPathError::InvalidSegment)
        } else {
            Ok(Self(path))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetPathError {
    Empty,
    Absolute,
    InvalidSegment,
    InvalidCharacter,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_matches;

    #[test]
    fn accepts_relative_paths() {
        for path in ["font.ttf", "fonts/default.ttf", "a/b/c.png", "..hidden/a b"] {
            assert_eq!(AssetPath::new(path).unwrap().as_str(), path);
        }
    }

    #[test]
    fn rejects_empty_path() {
        assert_matches!(AssetPath::new(""), Err(AssetPathError::Empty));
    }

    #[test]
    fn rejects_absolute_paths() {
        for path in ["/", "/etc/passwd", "//host/share"] {
            assert_matches!(AssetPath::new(path), Err(AssetPathError::Absolute));
        }
    }

    #[test]
    fn rejects_parent_current_and_empty_segments() {
        for path in [
            "..",
            "../a.png",
            "a/../b.png",
            ".",
            "a/./b.png",
            "a//b",
            "a/",
        ] {
            assert_matches!(AssetPath::new(path), Err(AssetPathError::InvalidSegment));
        }
    }

    #[test]
    fn rejects_platform_specific_and_control_characters() {
        for path in [
            "a\\b.png",
            "C:/a.png",
            "C:a.png",
            "https://a/b.png",
            "a\0b",
            "a\nb",
        ] {
            assert_matches!(AssetPath::new(path), Err(AssetPathError::InvalidCharacter));
        }
    }
}

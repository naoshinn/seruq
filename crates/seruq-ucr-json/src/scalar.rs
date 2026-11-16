use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use seruq_ucr::ElementId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct JsonId(pub ElementId);

impl Serialize for JsonId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for JsonId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let id = text.parse::<ElementId>().map_err(de::Error::custom)?;
        if text != id.to_string() {
            return Err(de::Error::custom("expected a canonical decimal u64 ID"));
        }
        Ok(Self(id))
    }
}

/// A finite f32. Non-finite values have no JSON representation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct JsonFloat(pub f32);

impl Serialize for JsonFloat {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f32(self.0)
    }
}

impl<'de> Deserialize<'de> for JsonFloat {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let number = f64::deserialize(deserializer)? as f32;
        if !number.is_finite() {
            return Err(de::Error::custom(
                "JSON number is outside the finite f32 range",
            ));
        }
        Ok(Self(number))
    }
}

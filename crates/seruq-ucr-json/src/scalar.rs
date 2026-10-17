use std::fmt;

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct JsonFloat(pub f32);

impl Serialize for JsonFloat {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0.is_nan() {
            serializer.serialize_str("NaN")
        } else if self.0 == f32::INFINITY {
            serializer.serialize_str("inf")
        } else if self.0 == f32::NEG_INFINITY {
            serializer.serialize_str("-inf")
        } else {
            serializer.serialize_f32(self.0)
        }
    }
}

impl<'de> Deserialize<'de> for JsonFloat {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FloatVisitor;

        impl de::Visitor<'_> for FloatVisitor {
            type Value = JsonFloat;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a finite f32 number or one of \"NaN\", \"inf\", \"-inf\"")
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                let number = value as f32;
                if !number.is_finite() {
                    return Err(E::custom("JSON number is outside the finite f32 range"));
                }
                Ok(JsonFloat(number))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                self.visit_f64(value as f64)
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                self.visit_f64(value as f64)
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                match value {
                    "NaN" => Ok(JsonFloat(f32::NAN)),
                    "inf" => Ok(JsonFloat(f32::INFINITY)),
                    "-inf" => Ok(JsonFloat(f32::NEG_INFINITY)),
                    _ => Err(E::invalid_value(de::Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_any(FloatVisitor)
    }
}

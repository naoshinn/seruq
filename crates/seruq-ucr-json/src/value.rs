use std::fmt;

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, DeserializeSeed},
    ser::{SerializeMap, SerializeSeq},
};
use seruq_ucr::{Number, Value, ValueMap};

/// A `state` or `data` object. Duplicate keys are rejected at every depth.
#[derive(Default)]
pub(crate) struct JsonValueMap(pub ValueMap);

impl JsonValueMap {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for JsonValueMap {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        MapRef(&self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JsonValueMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(MapVisitor).map(Self)
    }
}

struct ValueRef<'a>(&'a Value);

impl Serialize for ValueRef<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::Number(value) => serializer.serialize_f64(value.get()),
            Value::String(value) => serializer.serialize_str(value),
            Value::List(values) => {
                let mut list = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    list.serialize_element(&ValueRef(value))?;
                }
                list.end()
            }
            Value::Map(values) => MapRef(values).serialize(serializer),
        }
    }
}

struct MapRef<'a>(&'a ValueMap);

impl Serialize for MapRef<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in self.0 {
            map.serialize_entry(key, &ValueRef(value))?;
        }
        map.end()
    }
}

struct ValueSeed;

impl<'de> DeserializeSeed<'de> for ValueSeed {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> de::Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a boolean, finite number, string, array, or object")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Number::new(value)
            .map(Value::Number)
            .map_err(|_| E::custom("JSON number is outside the finite f64 range"))
    }

    // Integers beyond 2^53 round to the nearest f64.
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        self.visit_f64(value as f64)
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        self.visit_f64(value as f64)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(ValueSeed)? {
            values.push(value);
        }
        Ok(Value::List(values))
    }

    fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Value, A::Error> {
        MapVisitor.visit_map(map).map(Value::Map)
    }
}

struct MapVisitor;

impl<'de> de::Visitor<'de> for MapVisitor {
    type Value = ValueMap;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object with unique keys")
    }

    fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<ValueMap, A::Error> {
        let mut values = ValueMap::new();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value_seed(ValueSeed)?;
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate key {key:?}")));
            }
            values.insert(key, value);
        }
        Ok(values)
    }
}

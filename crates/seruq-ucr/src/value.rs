use std::collections::BTreeMap;

/// Application data stored in a document's `state` or an element's `data`.
///
/// There is no null: an absent value is an absent key.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Number(Number),
    String(String),
    List(Vec<Value>),
    Map(ValueMap),
}

/// Keys are unique and their order has no meaning.
pub type ValueMap = BTreeMap<String, Value>;

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(value.get()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Self::List(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&ValueMap> {
        match self {
            Self::Map(value) => Some(value),
            _ => None,
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        Self::Number(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Number(value.into())
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::Number(value.into())
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Self::List(value)
    }
}

impl From<ValueMap> for Value {
    fn from(value: ValueMap) -> Self {
        Self::Map(value)
    }
}

/// A finite binary64 number. Integers are exact up to 2^53.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number(f64);

impl Number {
    pub fn get(&self) -> f64 {
        self.0
    }

    pub fn new(value: f64) -> Result<Self, NumberError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(NumberError::NonFinite)
        }
    }
}

impl From<i32> for Number {
    fn from(value: i32) -> Self {
        Self(value.into())
    }
}

impl From<u32> for Number {
    fn from(value: u32) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberError {
    NonFinite,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_matches;

    #[test]
    fn number_preserves_finite_values() {
        for value in [0.0, -0.0, 0.1, -1.5, f64::MIN_POSITIVE, f64::MAX, f64::MIN] {
            assert_eq!(Number::new(value).unwrap().get().to_bits(), value.to_bits());
        }
    }

    #[test]
    fn number_rejects_non_finite_values() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_matches!(Number::new(value), Err(NumberError::NonFinite));
        }
    }

    #[test]
    fn number_converts_integers_exactly() {
        assert_eq!(Number::from(i32::MIN).get(), -2_147_483_648.0);
        assert_eq!(Number::from(u32::MAX).get(), 4_294_967_295.0);
    }

    #[test]
    fn accessors_match_only_their_own_type() {
        let map = ValueMap::from([("key".to_owned(), Value::from(1))]);
        let values = [
            Value::from(true),
            Value::from(2),
            Value::from("text"),
            Value::from(vec![Value::from(false)]),
            Value::from(map.clone()),
        ];
        assert_eq!(values[0].as_bool(), Some(true));
        assert_eq!(values[1].as_number(), Some(2.0));
        assert_eq!(values[2].as_str(), Some("text"));
        assert_eq!(values[3].as_list(), Some(&[Value::Bool(false)][..]));
        assert_eq!(values[4].as_map(), Some(&map));

        for (index, value) in values.iter().enumerate() {
            let matches = [
                value.as_bool().is_some(),
                value.as_number().is_some(),
                value.as_str().is_some(),
                value.as_list().is_some(),
                value.as_map().is_some(),
            ];
            assert_eq!(matches.iter().filter(|matched| **matched).count(), 1);
            assert!(matches[index]);
        }
    }
}

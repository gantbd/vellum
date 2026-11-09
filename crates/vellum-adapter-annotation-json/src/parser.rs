//! A bounded duplicate-detecting serde seed. Serde JSON owns lexical/Unicode
//! parsing; this layer does not invent another JSON lexer or trust Value's
//! default last-member-wins map deserialization.
use super::{CodecError, CodecLimits};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
struct Budget {
    limits: CodecLimits,
    nodes: usize,
    strings: usize,
    failure: Option<CodecError>,
}
impl Budget {
    fn reject<E: de::Error>(&mut self, error: CodecError) -> E {
        self.failure.get_or_insert(error);
        E::custom("annotation JSON refused")
    }
    fn string<E: de::Error>(&mut self, text: &str) -> Result<String, E> {
        self.strings = self
            .strings
            .checked_add(text.len())
            .filter(|n| *n <= self.limits.max_string_bytes)
            .ok_or_else(|| self.reject(CodecError::Limit))?;
        let mut owned = String::new();
        owned
            .try_reserve_exact(text.len())
            .map_err(|_| self.reject(CodecError::Capacity))?;
        owned.push_str(text);
        Ok(owned)
    }
}
struct Seed<'a> {
    budget: &'a mut Budget,
    depth: usize,
}
impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        if self.depth > self.budget.limits.max_depth
            || self.budget.nodes == self.budget.limits.max_nodes
        {
            return Err(self.budget.reject(CodecError::Limit));
        }
        self.budget.nodes += 1;
        deserializer.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Seed<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a bounded JSON value")
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(Number::from(value)))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(Number::from(value)))
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| self.budget.reject(CodecError::InvalidJson))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        self.budget.string(value).map(Value::String)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let mut result = Vec::new();
        while let Some(value) = sequence.next_element_seed(Seed {
            budget: self.budget,
            depth: self.depth + 1,
        })? {
            result
                .try_reserve(1)
                .map_err(|_| self.budget.reject(CodecError::Capacity))?;
            result.push(value);
        }
        Ok(Value::Array(result))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut result = Map::new();
        while let Some(key) = map.next_key_seed(KeySeed(self.budget))? {
            if result.contains_key(&key) {
                return Err(self.budget.reject(CodecError::DuplicateMember));
            }
            let value = map.next_value_seed(Seed {
                budget: self.budget,
                depth: self.depth + 1,
            })?;
            result.insert(key, value);
        }
        Ok(Value::Object(result))
    }
}
struct KeySeed<'a>(&'a mut Budget);
impl<'de> DeserializeSeed<'de> for KeySeed<'_> {
    type Value = String;
    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<String, D::Error> {
        deserializer.deserialize_str(self)
    }
}
impl Visitor<'_> for KeySeed<'_> {
    type Value = String;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a bounded object member name")
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<String, E> {
        self.0.string(value)
    }
}
pub(super) fn parse(bytes: &[u8], limits: CodecLimits) -> Result<Value, CodecError> {
    if bytes.starts_with(b"\xef\xbb\xbf") {
        return Err(CodecError::InvalidJson);
    }
    let mut budget = Budget {
        limits,
        nodes: 0,
        strings: 0,
        failure: None,
    };
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = Seed {
        budget: &mut budget,
        depth: 1,
    }
    .deserialize(&mut deserializer)
    .map_err(|_| budget.failure.unwrap_or(CodecError::InvalidJson))?;
    deserializer.end().map_err(|_| CodecError::InvalidJson)?;
    Ok(value)
}

//! Core value types and operations.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Value represents a JSON/YAML value that can be any of the supported types.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Map(Map),
}

/// Map represents a key-value map where keys are strings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Map {
    #[serde(flatten)]
    pub fields: std::collections::BTreeMap<String, Value>,
}

/// Field represents a single key-value pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub value: Value,
}

/// FieldList is a sorted list of fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldList {
    pub fields: Vec<Field>,
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Value::List(_))
    }

    pub fn is_map(&self) -> bool {
        matches!(self, Value::Map(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<Value>> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&Map> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        fn type_order(v: &Value) -> u8 {
            match v {
                Value::Null => 0,
                Value::Bool(_) => 1,
                Value::Int(_) => 2,
                Value::Float(_) => 3,
                Value::String(_) => 4,
                Value::List(_) => 5,
                Value::Map(_) => 6,
            }
        }

        let type_cmp = type_order(self).cmp(&type_order(other));
        if type_cmp != Ordering::Equal {
            return type_cmp;
        }

        match (self, other) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => {
                a.partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::List(a), Value::List(b)) => a.cmp(b),
            (Value::Map(a), Value::Map(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }
}

impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        self.fields == other.fields
    }
}

impl Eq for Map {}

impl PartialOrd for Map {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Map {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fields.cmp(&other.fields)
    }
}

impl Map {
    pub fn new() -> Self {
        Map {
            fields: std::collections::BTreeMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    pub fn set(&mut self, key: String, value: Value) {
        self.fields.insert(key, value);
    }

    pub fn has(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    pub fn delete(&mut self, key: &str) -> Option<Value> {
        self.fields.remove(key)
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.fields.iter()
    }
}

impl FieldList {
    pub fn new() -> Self {
        FieldList { fields: Vec::new() }
    }

    pub fn with_fields(fields: Vec<Field>) -> Self {
        let mut fl = FieldList { fields };
        fl.sort();
        fl
    }

    pub fn sort(&mut self) {
        self.fields.sort_by(|a, b| a.name.cmp(&b.name));
    }

    pub fn get(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Compare compares two FieldLists lexicographically.
    /// Returns Ordering::Less, Ordering::Equal, or Ordering::Greater.
    pub fn compare(&self, other: &FieldList) -> Ordering {
        let mut i = 0;
        let mut j = 0;

        while i < self.fields.len() && j < other.fields.len() {
            let a = &self.fields[i];
            let b = &other.fields[j];

            // Compare field names first
            match a.name.cmp(&b.name) {
                Ordering::Less => return Ordering::Less,
                Ordering::Greater => return Ordering::Greater,
                Ordering::Equal => {
                    // Names are equal, compare values
                    match a.value.cmp(&b.value) {
                        Ordering::Equal => {
                            i += 1;
                            j += 1;
                        }
                        other => return other,
                    }
                }
            }
        }

        // If we've exhausted one list, the shorter one is "less"
        self.fields.len().cmp(&other.fields.len())
    }

    /// Less returns true if this FieldList is lexicographically less than other.
    pub fn less(&self, other: &FieldList) -> bool {
        self.compare(other) == Ordering::Less
    }

    /// Equals returns true if both FieldLists have the same fields with the same values.
    pub fn equals(&self, other: &FieldList) -> bool {
        if self.fields.len() != other.fields.len() {
            return false;
        }
        for (a, b) in self.fields.iter().zip(other.fields.iter()) {
            if a.name != b.name || a.value != b.value {
                return false;
            }
        }
        true
    }

    /// Returns an iterator over the fields.
    pub fn iter(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter()
    }
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Bool(b) => b.hash(state),
            Value::Int(i) => i.hash(state),
            Value::Float(f) => f.to_bits().hash(state),
            Value::String(s) => s.hash(state),
            Value::List(l) => l.hash(state),
            Value::Map(m) => {
                for (k, v) in &m.fields {
                    k.hash(state);
                    v.hash(state);
                }
            }
        }
    }
}

impl std::hash::Hash for FieldList {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for field in &self.fields {
            field.name.hash(state);
            field.value.hash(state);
        }
    }
}

impl PartialOrd for FieldList {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FieldList {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

/// Parse a value from JSON.
pub fn from_json(json: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(json)
}

/// Serialize a value to JSON.
pub fn to_json(value: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

/// Parse a value from YAML.
pub fn from_yaml(yaml: &str) -> Result<Value, serde_yaml::Error> {
    serde_yaml::from_str(yaml)
}

/// Serialize a value to YAML.
pub fn to_yaml(value: &Value) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_types() {
        assert!(Value::Null.is_null());
        assert!(Value::Bool(true).is_bool());
        assert!(Value::Int(42).is_int());
        assert!(Value::Float(3.14).is_float());
        assert!(Value::String("hello".into()).is_string());
        assert!(Value::List(vec![]).is_list());
        assert!(Value::Map(Map::new()).is_map());
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Null, Value::Null);
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_eq!(Value::String("hello".into()), Value::String("hello".into()));
    }

    #[test]
    fn test_map_operations() {
        let mut map = Map::new();
        assert!(map.is_empty());

        map.set("key".into(), Value::String("value".into()));
        assert!(!map.is_empty());
        assert!(map.has("key"));
        assert_eq!(map.get("key"), Some(&Value::String("value".into())));

        map.delete("key");
        assert!(!map.has("key"));
    }

    #[test]
    fn test_json_roundtrip() {
        let value = Value::Map({
            let mut m = Map::new();
            m.set("name".into(), Value::String("test".into()));
            m.set("count".into(), Value::Int(42));
            m
        });

        let json = to_json(&value).unwrap();
        let parsed = from_json(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_field_list_compare() {
        let fl1 = FieldList::with_fields(vec![
            Field { name: "a".into(), value: Value::Int(1) },
            Field { name: "b".into(), value: Value::Int(2) },
        ]);
        let fl2 = FieldList::with_fields(vec![
            Field { name: "a".into(), value: Value::Int(1) },
            Field { name: "b".into(), value: Value::Int(2) },
        ]);
        let fl3 = FieldList::with_fields(vec![
            Field { name: "a".into(), value: Value::Int(1) },
            Field { name: "c".into(), value: Value::Int(2) },
        ]);
        let fl4 = FieldList::with_fields(vec![
            Field { name: "a".into(), value: Value::Int(1) },
        ]);

        // Equal lists
        assert_eq!(fl1.compare(&fl2), Ordering::Equal);
        assert!(fl1.equals(&fl2));
        assert!(!fl1.less(&fl2));

        // Different field names: "b" < "c"
        assert_eq!(fl1.compare(&fl3), Ordering::Less);
        assert!(fl1.less(&fl3));
        assert!(!fl1.equals(&fl3));

        // Different lengths: shorter list is less
        assert_eq!(fl4.compare(&fl1), Ordering::Less);
        assert!(fl4.less(&fl1));
    }

    #[test]
    fn test_field_list_compare_values() {
        let fl1 = FieldList::with_fields(vec![
            Field { name: "a".into(), value: Value::Int(1) },
        ]);
        let fl2 = FieldList::with_fields(vec![
            Field { name: "a".into(), value: Value::Int(2) },
        ]);

        // Same name, different values: 1 < 2
        assert_eq!(fl1.compare(&fl2), Ordering::Less);
        assert!(fl1.less(&fl2));
        assert!(!fl1.equals(&fl2));
    }

    #[test]
    fn test_field_list_ord() {
        let fl1 = FieldList::with_fields(vec![
            Field { name: "a".into(), value: Value::Int(1) },
        ]);
        let fl2 = FieldList::with_fields(vec![
            Field { name: "b".into(), value: Value::Int(1) },
        ]);

        // Test Ord trait
        assert!(fl1 < fl2);
        assert!(fl2 > fl1);
        assert!(fl1 <= fl1);
        assert!(fl1 >= fl1);
    }
}

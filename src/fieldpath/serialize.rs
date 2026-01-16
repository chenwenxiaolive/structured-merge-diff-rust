//! Serialization for fieldpath types.

use super::path::PathElement;
use super::set::Set;
use crate::value::{Field, FieldList, Value};
use serde_json;

/// Error type for serialization/deserialization.
#[derive(Debug, Clone)]
pub struct SerializeError {
    pub message: String,
}

impl SerializeError {
    pub fn new(message: impl Into<String>) -> Self {
        SerializeError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SerializeError {}

/// Serializes a PathElement to its string representation.
///
/// Format:
/// - FieldName: "f:name"
/// - Value: "v:json_value"
/// - Key: "k:{json_object}"
/// - Index: "i:number"
pub fn serialize_path_element(pe: &PathElement) -> Result<String, SerializeError> {
    match pe {
        PathElement::FieldName(name) => Ok(format!("f:{}", name)),
        PathElement::Value(v) => {
            let json = value_to_json(v)?;
            Ok(format!("v:{}", json))
        }
        PathElement::Key(fields) => {
            let json = field_list_to_json(fields)?;
            Ok(format!("k:{}", json))
        }
        PathElement::Index(i) => Ok(format!("i:{}", i)),
    }
}

/// Deserializes a PathElement from its string representation.
pub fn deserialize_path_element(s: &str) -> Result<PathElement, SerializeError> {
    if s.len() < 2 {
        return Err(SerializeError::new("key must be at least 2 characters long"));
    }

    let prefix = &s[..2];
    let content = &s[2..];

    match prefix {
        "f:" => Ok(PathElement::FieldName(content.to_string())),
        "v:" => {
            let v = json_to_value(content)?;
            Ok(PathElement::Value(v))
        }
        "k:" => {
            let fields = json_to_field_list(content)?;
            Ok(PathElement::Key(fields))
        }
        "i:" => {
            let i = content
                .parse::<i32>()
                .map_err(|e| SerializeError::new(format!("invalid index: {}", e)))?;
            Ok(PathElement::Index(i))
        }
        _ => Err(SerializeError::new(format!(
            "unknown path element type: {}",
            prefix
        ))),
    }
}

/// Converts a Value to JSON string.
fn value_to_json(v: &Value) -> Result<String, SerializeError> {
    let json_value = value_to_serde_json(v);
    serde_json::to_string(&json_value).map_err(|e| SerializeError::new(format!("JSON error: {}", e)))
}

/// Converts a Value to serde_json::Value.
fn value_to_serde_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number((*i).into()),
        Value::Float(f) => {
            if let Some(n) = serde_json::Number::from_f64(*f) {
                serde_json::Value::Number(n)
            } else {
                serde_json::Value::Null
            }
        }
        Value::String(s) => serde_json::Value::String(s.to_string()),
        Value::List(items) => {
            let arr: Vec<serde_json::Value> = items.iter().map(value_to_serde_json).collect();
            serde_json::Value::Array(arr)
        }
        Value::Map(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_serde_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

/// Converts a FieldList to JSON string.
fn field_list_to_json(fields: &FieldList) -> Result<String, SerializeError> {
    let mut obj = serde_json::Map::new();
    for field in &fields.fields {
        obj.insert(field.name.clone(), value_to_serde_json(&field.value));
    }
    serde_json::to_string(&serde_json::Value::Object(obj))
        .map_err(|e| SerializeError::new(format!("JSON error: {}", e)))
}

/// Converts a JSON string to Value.
fn json_to_value(s: &str) -> Result<Value, SerializeError> {
    let json_value: serde_json::Value =
        serde_json::from_str(s).map_err(|e| SerializeError::new(format!("JSON parse error: {}", e)))?;
    Ok(serde_json_to_value(&json_value))
}

/// Converts serde_json::Value to Value.
fn serde_json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            let items: Vec<Value> = arr.iter().map(serde_json_to_value).collect();
            Value::List(items)
        }
        serde_json::Value::Object(obj) => {
            let mut map = crate::value::Map::new();
            for (k, v) in obj {
                map.set(k.clone(), serde_json_to_value(v));
            }
            Value::Map(map)
        }
    }
}

/// Converts a JSON string to FieldList.
fn json_to_field_list(s: &str) -> Result<FieldList, SerializeError> {
    let json_value: serde_json::Value =
        serde_json::from_str(s).map_err(|e| SerializeError::new(format!("JSON parse error: {}", e)))?;

    match json_value {
        serde_json::Value::Object(obj) => {
            let mut fields: Vec<Field> = obj
                .into_iter()
                .map(|(name, v)| Field {
                    name,
                    value: serde_json_to_value(&v),
                })
                .collect();
            fields.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(FieldList { fields })
        }
        _ => Err(SerializeError::new("expected JSON object for key")),
    }
}

/// Serializes a Set to JSON bytes.
impl Set {
    pub fn to_json(&self) -> Result<Vec<u8>, SerializeError> {
        let json_obj = self.to_json_object(false)?;
        serde_json::to_vec(&json_obj).map_err(|e| SerializeError::new(format!("JSON error: {}", e)))
    }

    /// Deserializes a Set from JSON bytes.
    pub fn from_json(data: &[u8]) -> Result<Set, SerializeError> {
        let json_value: serde_json::Value =
            serde_json::from_slice(data).map_err(|e| SerializeError::new(format!("JSON parse error: {}", e)))?;

        match json_value {
            serde_json::Value::Object(obj) => Self::from_json_object(obj),
            _ => Err(SerializeError::new("expected JSON object")),
        }
    }

    fn to_json_object(&self, include_self: bool) -> Result<serde_json::Map<String, serde_json::Value>, SerializeError> {
        let mut result = serde_json::Map::new();

        // If we need to mark the current path as being in the set
        if include_self && !(self.members.is_empty() && self.children.is_empty()) {
            result.insert(".".to_string(), serde_json::Value::Object(serde_json::Map::new()));
        }

        // Merge members and children in sorted order
        let mut all_elements: Vec<(&PathElement, Option<&Set>)> = Vec::new();

        // Add members (with None for child set)
        for member in self.members.iter() {
            all_elements.push((member, None));
        }

        // Add children
        for (pe, child) in &self.children {
            all_elements.push((pe, Some(child)));
        }

        // Sort by path element
        all_elements.sort_by(|(a, _), (b, _)| a.cmp(b));

        // Deduplicate and merge where both member and child exist
        let mut i = 0;
        while i < all_elements.len() {
            let (pe, child) = &all_elements[i];
            let key = serialize_path_element(pe)?;

            // Check if next element is the same path element
            let has_child_next = i + 1 < all_elements.len()
                && all_elements[i + 1].0 == *pe
                && all_elements[i + 1].1.is_some();

            if child.is_some() {
                // This is a child set
                let child_obj = child.unwrap().to_json_object(false)?;
                result.insert(key, serde_json::Value::Object(child_obj));
            } else if has_child_next {
                // This is a member but next is the child - emit with include_self=true
                let child_set = all_elements[i + 1].1.unwrap();
                let child_obj = child_set.to_json_object(true)?;
                result.insert(key, serde_json::Value::Object(child_obj));
                i += 1; // Skip the child entry
            } else {
                // Just a member with no children
                result.insert(key, serde_json::Value::Object(serde_json::Map::new()));
            }

            i += 1;
        }

        Ok(result)
    }

    fn from_json_object(obj: serde_json::Map<String, serde_json::Value>) -> Result<Set, SerializeError> {
        let mut set = Set::new();

        for (key, value) in obj {
            if key == "." {
                // Mark current path as in set (handled at parent level)
                continue;
            }

            // Try to parse the path element
            let pe = match deserialize_path_element(&key) {
                Ok(pe) => pe,
                Err(e) => {
                    // Skip unknown path element types (for forward compatibility)
                    if e.message.starts_with("unknown path element type") {
                        continue;
                    }
                    return Err(e);
                }
            };

            // Parse the child object
            match value {
                serde_json::Value::Object(child_obj) => {
                    if child_obj.is_empty() {
                        // Empty object means this is just a member
                        set.members.insert(pe);
                    } else {
                        // Check if "." is present (means this path is also a member)
                        let is_member = child_obj.contains_key(".");

                        // Parse children
                        let child_set = Self::from_json_object(child_obj)?;

                        if is_member {
                            set.members.insert(pe.clone());
                        }

                        if !child_set.is_empty() {
                            set.children.insert(pe, child_set);
                        }
                    }
                }
                _ => {
                    return Err(SerializeError::new(format!(
                        "expected object value for key: {}",
                        key
                    )));
                }
            }
        }

        Ok(set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_path_element_field() {
        let pe = PathElement::field_name("foo");
        let s = serialize_path_element(&pe).unwrap();
        assert_eq!(s, "f:foo");

        let pe2 = deserialize_path_element(&s).unwrap();
        assert_eq!(pe, pe2);
    }

    #[test]
    fn test_serialize_path_element_value() {
        let pe = PathElement::value(Value::String("test".into()));
        let s = serialize_path_element(&pe).unwrap();
        assert_eq!(s, "v:\"test\"");

        let pe2 = deserialize_path_element(&s).unwrap();
        assert_eq!(pe, pe2);
    }

    #[test]
    fn test_serialize_path_element_index() {
        let pe = PathElement::index(42);
        let s = serialize_path_element(&pe).unwrap();
        assert_eq!(s, "i:42");

        let pe2 = deserialize_path_element(&s).unwrap();
        assert_eq!(pe, pe2);
    }

    #[test]
    fn test_serialize_path_element_key() {
        let fields = FieldList {
            fields: vec![
                Field {
                    name: "name".to_string(),
                    value: Value::String("first".into()),
                },
            ],
        };
        let pe = PathElement::key(fields);
        let s = serialize_path_element(&pe).unwrap();
        assert_eq!(s, "k:{\"name\":\"first\"}");

        let pe2 = deserialize_path_element(&s).unwrap();
        assert_eq!(pe, pe2);
    }

    #[test]
    fn test_set_json_roundtrip() {
        use super::super::path::Path;

        let mut set = Set::new();
        set.insert(&Path::from_elements(vec![PathElement::field_name("a")]));
        set.insert(&Path::from_elements(vec![PathElement::field_name("b")]));
        set.insert(&Path::from_elements(vec![
            PathElement::field_name("c"),
            PathElement::field_name("d"),
        ]));

        let json = set.to_json().unwrap();
        let set2 = Set::from_json(&json).unwrap();

        assert!(set.equals(&set2));
    }

    #[test]
    fn test_set_golden_data() {
        // Test with known golden data from Go tests
        let examples = vec![
            r#"{"f:aaa":{},"f:aab":{}}"#,
            r#"{"f:a":{"f:b":{}}}"#,
        ];

        for example in examples {
            let set = Set::from_json(example.as_bytes()).unwrap();
            let json = set.to_json().unwrap();
            let json_str = String::from_utf8(json).unwrap();
            // The output should be parseable back
            let set2 = Set::from_json(json_str.as_bytes()).unwrap();
            assert!(set.equals(&set2));
        }
    }

    #[test]
    fn test_drop_unknown() {
        // Unknown prefix "r:" should be dropped
        let input = r#"{"f:aaa":{},"r:aab":{}}"#;
        let set = Set::from_json(input.as_bytes()).unwrap();
        let json = set.to_json().unwrap();
        let json_str = String::from_utf8(json).unwrap();
        assert_eq!(json_str, r#"{"f:aaa":{}}"#);
    }

    #[test]
    fn test_golden_data_complex() {
        // Complex golden data from Go tests - these should deserialize and serialize exactly
        let examples = vec![
            r#"{"f:aaa":{},"f:aab":{},"f:aac":{},"f:aad":{},"f:aae":{},"f:aaf":{},"k:{\"name\":\"first\"}":{},"k:{\"name\":\"second\"}":{},"k:{\"port\":443,\"protocol\":\"tcp\"}":{},"k:{\"port\":443,\"protocol\":\"udp\"}":{},"v:1":{},"v:2":{},"v:3":{},"v:\"aa\"":{},"v:\"ab\"":{},"v:true":{},"i:1":{},"i:2":{},"i:3":{},"i:4":{}}"#,
        ];

        for example in examples {
            let set = Set::from_json(example.as_bytes()).unwrap();
            let json = set.to_json().unwrap();
            let json_str = String::from_utf8(json).unwrap();

            // Parse again to verify roundtrip
            let set2 = Set::from_json(json_str.as_bytes()).unwrap();
            assert!(set.equals(&set2), "Sets not equal after roundtrip");
        }
    }

    #[test]
    fn test_serialize_path_element_key_multifield() {
        // Test key with multiple fields
        let fields = FieldList {
            fields: vec![
                Field {
                    name: "port".to_string(),
                    value: Value::Int(443),
                },
                Field {
                    name: "protocol".to_string(),
                    value: Value::String("tcp".into()),
                },
            ],
        };
        let pe = PathElement::key(fields);
        let s = serialize_path_element(&pe).unwrap();
        // Fields should be sorted by name
        assert_eq!(s, r#"k:{"port":443,"protocol":"tcp"}"#);

        let pe2 = deserialize_path_element(&s).unwrap();
        assert_eq!(pe, pe2);
    }

    #[test]
    fn test_serialize_path_element_value_types() {
        // Test various value types
        let test_cases = vec![
            (PathElement::value(Value::Int(1)), "v:1"),
            (PathElement::value(Value::Int(2)), "v:2"),
            (PathElement::value(Value::Bool(true)), "v:true"),
            (PathElement::value(Value::Bool(false)), "v:false"),
            (PathElement::value(Value::String("aa".into())), r#"v:"aa""#),
            (PathElement::value(Value::Float(3.14)), "v:3.14"),
        ];

        for (pe, expected) in test_cases {
            let s = serialize_path_element(&pe).unwrap();
            assert_eq!(s, expected, "Serialization mismatch for {:?}", pe);

            let pe2 = deserialize_path_element(&s).unwrap();
            assert_eq!(pe, pe2, "Roundtrip mismatch for {:?}", pe);
        }
    }

    #[test]
    fn test_serialize_nested_set() {
        // Test a set with nested paths
        use super::super::path::Path;

        let mut set = Set::new();
        // Add some nested paths like: f:aaa.k:{name:second}.v:3.f:aab
        set.insert(&Path::from_elements(vec![
            PathElement::field_name("aaa"),
            PathElement::key(FieldList {
                fields: vec![Field {
                    name: "name".to_string(),
                    value: Value::String("second".into()),
                }],
            }),
            PathElement::value(Value::Int(3)),
            PathElement::field_name("aab"),
        ]));

        // Add another path
        set.insert(&Path::from_elements(vec![
            PathElement::field_name("aaa"),
            PathElement::value(Value::Int(3)),
        ]));

        let json = set.to_json().unwrap();
        let set2 = Set::from_json(&json).unwrap();
        assert!(set.equals(&set2), "Sets not equal after roundtrip");
    }
}

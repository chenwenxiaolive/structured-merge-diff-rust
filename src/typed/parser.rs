//! Parser for creating typed values from YAML schemas and objects.

use crate::schema::{Schema, TypeRef};
use crate::value::Value;
use super::typed_value::{as_typed, TypedValue};
use super::validation::{ValidationErrors, ValidationOption};

/// Parser implements YAML schema parsing and type creation.
#[derive(Debug, Clone)]
pub struct Parser {
    pub schema: Schema,
}

impl Parser {
    /// Creates a new parser from a YAML schema string.
    pub fn new(schema_yaml: &str) -> Result<Parser, ParseError> {
        let schema: Schema = serde_yaml::from_str(schema_yaml)
            .map_err(|e| ParseError::new(format!("failed to parse schema: {}", e)))?;
        Ok(Parser { schema })
    }

    /// Returns the list of type names in this schema.
    pub fn type_names(&self) -> Vec<&str> {
        self.schema.types.iter().map(|t| t.name.as_str()).collect()
    }

    /// Returns a ParseableType helper for the given type name.
    pub fn type_by_name(&self, name: &str) -> ParseableType {
        ParseableType {
            schema: self.schema.clone(),
            type_ref: TypeRef {
                named_type: Some(name.to_string()),
                ..Default::default()
            },
        }
    }
}

/// ParseableType allows for easy production of typed objects.
#[derive(Debug, Clone)]
pub struct ParseableType {
    pub schema: Schema,
    pub type_ref: TypeRef,
}

impl ParseableType {
    /// Returns true if the type is valid in the schema.
    pub fn is_valid(&self) -> bool {
        self.schema.resolve(&self.type_ref).is_some()
    }

    /// Parses a YAML string into a TypedValue.
    pub fn from_yaml(&self, yaml: &str) -> Result<TypedValue, ParseError> {
        self.from_yaml_with_opts(yaml, &[])
    }

    /// Parses a YAML string into a TypedValue with validation options.
    pub fn from_yaml_with_opts(
        &self,
        yaml: &str,
        opts: &[ValidationOption],
    ) -> Result<TypedValue, ParseError> {
        let value: Value = serde_yaml::from_str(yaml)
            .map_err(|e| ParseError::new(format!("failed to parse YAML: {}", e)))?;

        as_typed(value, &self.schema, self.type_ref.clone(), opts)
            .map_err(|e| ParseError::new(format!("validation failed: {}", e)))
    }

    /// Creates a TypedValue from a Value.
    pub fn from_value(&self, value: Value) -> Result<TypedValue, ParseError> {
        self.from_value_with_opts(value, &[])
    }

    /// Creates a TypedValue from a Value with validation options.
    pub fn from_value_with_opts(
        &self,
        value: Value,
        opts: &[ValidationOption],
    ) -> Result<TypedValue, ParseError> {
        as_typed(value, &self.schema, self.type_ref.clone(), opts)
            .map_err(|e| ParseError::new(format!("validation failed: {}", e)))
    }
}

/// Error type for parsing operations.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        ParseError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

impl From<ValidationErrors> for ParseError {
    fn from(e: ValidationErrors) -> Self {
        ParseError::new(format!("{}", e))
    }
}

/// Creates a deduced type parser for untyped/deduced schemas.
pub fn deduced_parseable_type() -> ParseableType {
    let schema_yaml = r#"types:
- name: __untyped_atomic_
  scalar: untyped
  list:
    elementType:
      namedType: __untyped_atomic_
    elementRelationship: atomic
  map:
    elementType:
      namedType: __untyped_atomic_
    elementRelationship: atomic
- name: __untyped_deduced_
  scalar: untyped
  list:
    elementType:
      namedType: __untyped_atomic_
    elementRelationship: atomic
  map:
    elementType:
      namedType: __untyped_deduced_
    elementRelationship: separable
"#;

    let parser = Parser::new(schema_yaml).expect("deduced schema should parse");
    parser.type_by_name("__untyped_deduced_")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SCHEMA: &str = r#"types:
- name: stringPair
  map:
    fields:
    - name: key
      type:
        scalar: string
    - name: value
      type:
        scalar: string
"#;

    #[test]
    fn test_parser_new() {
        let parser = Parser::new(TEST_SCHEMA).unwrap();
        assert!(parser.type_names().contains(&"stringPair"));
    }

    #[test]
    fn test_parseable_type_from_yaml() {
        let parser = Parser::new(TEST_SCHEMA).unwrap();
        let pt = parser.type_by_name("stringPair");

        let tv = pt.from_yaml(r#"{"key": "foo", "value": "bar"}"#).unwrap();
        assert!(tv.value().is_map());
    }

    #[test]
    fn test_parseable_type_is_valid() {
        let parser = Parser::new(TEST_SCHEMA).unwrap();
        assert!(parser.type_by_name("stringPair").is_valid());
        assert!(!parser.type_by_name("nonexistent").is_valid());
    }

    #[test]
    fn test_deduced_parseable_type() {
        let pt = deduced_parseable_type();
        assert!(pt.is_valid());

        let tv = pt.from_yaml(r#"{"a": 1, "b": "hello"}"#).unwrap();
        assert!(tv.value().is_map());
    }
}

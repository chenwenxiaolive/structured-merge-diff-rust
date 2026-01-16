//! Core schema elements and type definitions.

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Schema is a list of named types.
///
/// Schema types are indexed in a map before the first search so this type
/// should be considered immutable.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Schema {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<TypeDef>,

    #[serde(skip)]
    type_map: OnceCell<HashMap<String, TypeDef>>,

    #[serde(skip)]
    resolved_types: Mutex<HashMap<TypeRefKey, Atom>>,
}

impl Clone for Schema {
    fn clone(&self) -> Self {
        Schema {
            types: self.types.clone(),
            type_map: OnceCell::new(),
            resolved_types: Mutex::new(HashMap::new()),
        }
    }
}

/// Key for caching resolved type references.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TypeRefKey {
    named_type: Option<String>,
    element_relationship: Option<ElementRelationship>,
}

impl From<&TypeRef> for TypeRefKey {
    fn from(tr: &TypeRef) -> Self {
        TypeRefKey {
            named_type: tr.named_type.clone(),
            element_relationship: tr.element_relationship,
        }
    }
}

/// A TypeSpecifier references a particular type in a schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeSpecifier {
    #[serde(default, rename = "type")]
    pub type_ref: TypeRef,
    #[serde(default)]
    pub schema: Schema,
}

/// TypeDef represents a named type in a schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeDef {
    /// Top level types should be named. Every type must have a unique name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,

    #[serde(flatten)]
    pub atom: Atom,
}

/// TypeRef either refers to a named type or declares an inlined type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeRef {
    /// Reference to named type in schema.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "namedType")]
    pub named_type: Option<String>,

    /// Inline type definition.
    #[serde(flatten)]
    pub inlined: Box<Atom>,

    /// If this reference refers to a map-type or list-type, this field overrides
    /// the `ElementRelationship` of the referred type when resolved.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "elementRelationship"
    )]
    pub element_relationship: Option<ElementRelationship>,
}

/// Atom represents the smallest possible pieces of the type system.
/// Each set field in the Atom represents a possible type for the object.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Atom {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scalar: Option<Scalar>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<List>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<Map>,
}

/// Scalar (AKA "primitive") represents a type which has a single value which is
/// either numeric, string, or boolean, or untyped for any of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scalar {
    Numeric,
    String,
    Boolean,
    Untyped,
}

/// ElementRelationship is an enum of the different possible relationships
/// between the elements of container types (maps, lists).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElementRelationship {
    /// Associative only applies to lists (see the documentation there).
    Associative,
    /// Atomic makes container types (lists, maps) behave as scalars / leaf fields.
    Atomic,
    /// Separable means the items of the container type have no particular
    /// relationship (default behavior for maps).
    Separable,
}

impl Default for ElementRelationship {
    fn default() -> Self {
        ElementRelationship::Separable
    }
}

/// Map is a key-value pair. Its default semantics are the same as an
/// associative list, but:
/// - It is serialized differently
/// - Keys must be string typed
/// - Keys can't have multiple components
///
/// Maps may also represent a type which is composed of a number of different fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Map {
    /// Each struct field appears exactly once in this list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<StructField>,

    /// A Union is a grouping of fields with special rules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unions: Vec<Union>,

    /// ElementType is the type of the struct's unknown fields.
    #[serde(default, rename = "elementType")]
    pub element_type: TypeRef,

    /// ElementRelationship states the relationship between the map's items.
    #[serde(
        default,
        skip_serializing_if = "is_default_element_relationship",
        rename = "elementRelationship"
    )]
    pub element_relationship: ElementRelationship,

    #[serde(skip)]
    field_map: OnceCell<HashMap<String, StructField>>,
}

fn is_default_element_relationship(er: &ElementRelationship) -> bool {
    *er == ElementRelationship::Separable
}

/// UnionField is a mapping between the fields that are part of the union and
/// their discriminated value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnionField {
    /// FieldName is the name of the field that is part of the union.
    #[serde(default, rename = "fieldName")]
    pub field_name: String,

    /// DiscriminatorValue is the value of the discriminator to select that field.
    #[serde(default, rename = "discriminatorValue")]
    pub discriminator_value: String,
}

/// Union, or oneof, means that only one of multiple fields of a structure can be
/// set at a time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Union {
    /// Discriminator, if present, is the name of the field that discriminates
    /// fields in the union.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<String>,

    /// DeduceInvalidDiscriminator indicates if the discriminator should be
    /// updated automatically based on the fields set.
    #[serde(default, rename = "deduceInvalidDiscriminator")]
    pub deduce_invalid_discriminator: bool,

    /// This is the list of fields that belong to this union.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<UnionField>,
}

/// StructField pairs a field name with a field type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructField {
    /// Name is the field name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,

    /// Type is the field type.
    #[serde(default, rename = "type")]
    pub field_type: TypeRef,

    /// Default value for the field, None if not present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// List represents a type which contains zero or more elements, all of the
/// same subtype.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct List {
    /// ElementType is the type of the list's elements.
    #[serde(default, rename = "elementType")]
    pub element_type: TypeRef,

    /// ElementRelationship states the relationship between the list's elements.
    #[serde(default, rename = "elementRelationship")]
    pub element_relationship: ElementRelationship,

    /// Keys lists the fields of the element's map type which are to be used
    /// as the keys of the list (for associative lists).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<String>,
}

impl Schema {
    /// Creates a new empty schema.
    pub fn new() -> Self {
        Schema::default()
    }

    /// Creates a schema with the given type definitions.
    pub fn with_types(types: Vec<TypeDef>) -> Self {
        Schema {
            types,
            type_map: OnceCell::new(),
            resolved_types: Mutex::new(HashMap::new()),
        }
    }

    /// FindNamedType returns the referenced TypeDef, if it exists.
    pub fn find_named_type(&self, name: &str) -> Option<&TypeDef> {
        let map = self.type_map.get_or_init(|| {
            self.types
                .iter()
                .map(|t| (t.name.clone(), t.clone()))
                .collect()
        });
        map.get(name)
    }

    fn resolve_no_overrides(&self, tr: &TypeRef) -> Option<Atom> {
        if let Some(ref named) = tr.named_type {
            self.find_named_type(named).map(|t| t.atom.clone())
        } else {
            Some((*tr.inlined).clone())
        }
    }

    /// Resolve returns the atom referenced, whether it is inline or named.
    /// Returns None if the type can't be resolved.
    ///
    /// This allows callers to not care about the difference between a (possibly
    /// inlined) reference and a definition.
    pub fn resolve(&self, tr: &TypeRef) -> Option<Atom> {
        // If this is a plain reference with no overrides, just return the type
        if tr.element_relationship.is_none() {
            return self.resolve_no_overrides(tr);
        }

        let key = TypeRefKey::from(tr);

        // Check cache first
        {
            let cache = self.resolved_types.lock().unwrap();
            if let Some(atom) = cache.get(&key) {
                return Some(atom.clone());
            }
        }

        // Calculate result
        let result = self.resolve_no_overrides(tr)?;
        let element_relationship = tr.element_relationship.unwrap();

        let result = match (&result.map, &result.list, &result.scalar) {
            (Some(map), _, _) => {
                let mut map_copy = map.clone();
                map_copy.element_relationship = element_relationship;
                Atom {
                    map: Some(map_copy),
                    list: None,
                    scalar: None,
                }
            }
            (_, Some(list), _) => {
                let mut list_copy = list.clone();
                list_copy.element_relationship = element_relationship;
                Atom {
                    map: None,
                    list: Some(list_copy),
                    scalar: None,
                }
            }
            (_, _, Some(_)) => return None,
            _ => return None,
        };

        // Cache and return
        {
            let mut cache = self.resolved_types.lock().unwrap();
            cache.insert(key, result.clone());
        }

        Some(result)
    }

    /// Copies this schema into the destination.
    pub fn copy_into(&self, dst: &mut Schema) {
        dst.types = self.types.clone();
        // Reset the cache in destination
        dst.type_map = OnceCell::new();
        dst.resolved_types = Mutex::new(HashMap::new());
    }
}

impl Map {
    /// Creates a new empty Map.
    pub fn new() -> Self {
        Map::default()
    }

    /// Creates a new Map with the given fields.
    pub fn with_fields(fields: Vec<StructField>) -> Self {
        Map {
            fields,
            ..Default::default()
        }
    }

    /// Creates a new Map with the given element type.
    pub fn with_element_type(element_type: TypeRef) -> Self {
        Map {
            element_type,
            ..Default::default()
        }
    }

    /// Creates a new Map with the given element type and relationship.
    pub fn with_element_type_and_relationship(element_type: TypeRef, element_relationship: ElementRelationship) -> Self {
        Map {
            element_type,
            element_relationship,
            ..Default::default()
        }
    }

    /// FindField returns the referenced StructField, if it exists.
    pub fn find_field(&self, name: &str) -> Option<&StructField> {
        let map = self.field_map.get_or_init(|| {
            self.fields
                .iter()
                .map(|f| (f.name.clone(), f.clone()))
                .collect()
        });
        map.get(name)
    }

    /// Copies this map into the destination.
    pub fn copy_into(&self, dst: &mut Map) {
        dst.fields = self.fields.clone();
        dst.element_type = self.element_type.clone();
        dst.unions = self.unions.clone();
        dst.element_relationship = self.element_relationship;
        // Reset the cache in destination
        dst.field_map = OnceCell::new();
    }
}

impl Atom {
    /// Returns true if this atom represents a scalar type.
    pub fn is_scalar(&self) -> bool {
        self.scalar.is_some()
    }

    /// Returns true if this atom represents a list type.
    pub fn is_list(&self) -> bool {
        self.list.is_some()
    }

    /// Returns true if this atom represents a map type.
    pub fn is_map(&self) -> bool {
        self.map.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_serialization() {
        assert_eq!(
            serde_json::to_string(&Scalar::Numeric).unwrap(),
            "\"numeric\""
        );
        assert_eq!(
            serde_json::to_string(&Scalar::String).unwrap(),
            "\"string\""
        );
        assert_eq!(
            serde_json::to_string(&Scalar::Boolean).unwrap(),
            "\"boolean\""
        );
        assert_eq!(
            serde_json::to_string(&Scalar::Untyped).unwrap(),
            "\"untyped\""
        );
    }

    #[test]
    fn test_element_relationship_serialization() {
        assert_eq!(
            serde_json::to_string(&ElementRelationship::Associative).unwrap(),
            "\"associative\""
        );
        assert_eq!(
            serde_json::to_string(&ElementRelationship::Atomic).unwrap(),
            "\"atomic\""
        );
        assert_eq!(
            serde_json::to_string(&ElementRelationship::Separable).unwrap(),
            "\"separable\""
        );
    }

    #[test]
    fn test_schema_find_named_type() {
        let schema = Schema::with_types(vec![
            TypeDef {
                name: "string".to_string(),
                atom: Atom {
                    scalar: Some(Scalar::String),
                    ..Default::default()
                },
            },
            TypeDef {
                name: "int".to_string(),
                atom: Atom {
                    scalar: Some(Scalar::Numeric),
                    ..Default::default()
                },
            },
        ]);

        assert!(schema.find_named_type("string").is_some());
        assert!(schema.find_named_type("int").is_some());
        assert!(schema.find_named_type("nonexistent").is_none());
    }

    #[test]
    fn test_map_find_field() {
        let map = Map {
            fields: vec![
                StructField {
                    name: "name".to_string(),
                    field_type: TypeRef {
                        named_type: Some("string".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                StructField {
                    name: "age".to_string(),
                    field_type: TypeRef {
                        named_type: Some("int".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert!(map.find_field("name").is_some());
        assert!(map.find_field("age").is_some());
        assert!(map.find_field("nonexistent").is_none());
    }

    #[test]
    fn test_schema_resolve() {
        let schema = Schema::with_types(vec![TypeDef {
            name: "myMap".to_string(),
            atom: Atom {
                map: Some(Map {
                    element_relationship: ElementRelationship::Separable,
                    ..Default::default()
                }),
                ..Default::default()
            },
        }]);

        // Resolve without override
        let type_ref = TypeRef {
            named_type: Some("myMap".to_string()),
            ..Default::default()
        };
        let resolved = schema.resolve(&type_ref).unwrap();
        assert!(resolved.map.is_some());
        assert_eq!(
            resolved.map.unwrap().element_relationship,
            ElementRelationship::Separable
        );

        // Resolve with override
        let type_ref_override = TypeRef {
            named_type: Some("myMap".to_string()),
            element_relationship: Some(ElementRelationship::Atomic),
            ..Default::default()
        };
        let resolved = schema.resolve(&type_ref_override).unwrap();
        assert!(resolved.map.is_some());
        assert_eq!(
            resolved.map.unwrap().element_relationship,
            ElementRelationship::Atomic
        );
    }
}

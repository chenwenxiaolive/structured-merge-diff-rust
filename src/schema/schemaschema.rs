//! Schema schema definition - a schema that can validate other schemas.

/// SchemaSchemaYAML is a schema against which you can validate other schemas.
/// It will validate itself. It can be unmarshalled into a Schema type.
pub const SCHEMA_SCHEMA_YAML: &str = r#"types:
- name: schema
  map:
    fields:
      - name: types
        type:
          list:
            elementRelationship: associative
            elementType:
              namedType: typeDef
            keys:
            - name
- name: typeDef
  map:
    fields:
    - name: name
      type:
        scalar: string
    - name: scalar
      type:
        scalar: string
    - name: map
      type:
        namedType: map
    - name: list
      type:
        namedType: list
    - name: untyped
      type:
        namedType: untyped
- name: typeRef
  map:
    fields:
    - name: namedType
      type:
        scalar: string
    - name: scalar
      type:
        scalar: string
    - name: map
      type:
        namedType: map
    - name: list
      type:
        namedType: list
    - name: untyped
      type:
        namedType: untyped
    - name: elementRelationship
      type:
        scalar: string
- name: scalar
  scalar: string
- name: map
  map:
    fields:
    - name: fields
      type:
        list:
          elementType:
            namedType: structField
          elementRelationship: associative
          keys: [ "name" ]
    - name: unions
      type:
        list:
          elementType:
            namedType: union
          elementRelationship: atomic
    - name: elementType
      type:
        namedType: typeRef
    - name: elementRelationship
      type:
        scalar: string
- name: unionField
  map:
    fields:
    - name: fieldName
      type:
        scalar: string
    - name: discriminatorValue
      type:
        scalar: string
- name: union
  map:
    fields:
    - name: discriminator
      type:
        scalar: string
    - name: deduceInvalidDiscriminator
      type:
        scalar: boolean
    - name: fields
      type:
        list:
          elementRelationship: associative
          elementType:
            namedType: unionField
          keys:
          - fieldName
- name: structField
  map:
    fields:
    - name: name
      type:
        scalar: string
    - name: type
      type:
        namedType: typeRef
    - name: default
      type:
        namedType: __untyped_atomic_
- name: list
  map:
    fields:
    - name: elementType
      type:
        namedType: typeRef
    - name: elementRelationship
      type:
        scalar: string
    - name: keys
      type:
        list:
          elementType:
            scalar: string
          elementRelationship: atomic
- name: untyped
  map:
    fields:
    - name: elementRelationship
      type:
        scalar: string
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
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Schema;

    #[test]
    fn test_schema_schema_parses() {
        let schema: Result<Schema, _> = serde_yaml::from_str(SCHEMA_SCHEMA_YAML);
        assert!(schema.is_ok(), "Failed to parse schema schema: {:?}", schema.err());

        let schema = schema.unwrap();
        assert!(!schema.types.is_empty());

        // Verify some key types exist
        assert!(schema.find_named_type("schema").is_some());
        assert!(schema.find_named_type("typeDef").is_some());
        assert!(schema.find_named_type("typeRef").is_some());
        assert!(schema.find_named_type("map").is_some());
        assert!(schema.find_named_type("list").is_some());
    }
}

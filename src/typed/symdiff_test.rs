//! Tests for symmetric difference (compare) operations.
//!
//! Based on Go tests from typed/symdiff_test.go

#[cfg(test)]
mod tests {
    use crate::fieldpath::{Path, PathElement};
    use crate::typed::{Parser, ValidationOption};
    use crate::value::{Field, FieldList, Value};

    /// Helper to create a path from field names.
    fn path(elements: Vec<&str>) -> Path {
        Path::from_elements(
            elements
                .into_iter()
                .map(|e| PathElement::field_name(e))
                .collect(),
        )
    }

    /// Helper to create a key-based field path element.
    fn key_element(pairs: Vec<(&str, Value)>) -> PathElement {
        let fields: Vec<Field> = pairs
            .into_iter()
            .map(|(k, v)| Field {
                name: k.to_string(),
                value: v,
            })
            .collect();
        PathElement::Key(FieldList::with_fields(fields))
    }

    #[test]
    fn test_symdiff_simple_pair_same() {
        let schema = r#"types:
- name: stringPair
  map:
    fields:
    - name: key
      type:
        scalar: string
    - name: value
      type:
        namedType: __untyped_atomic_
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
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("stringPair");

        // Same values should have no diff
        let lhs = pt.from_yaml(r#"{"key":"foo","value":1}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"key":"foo","value":1}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(comparison.removed.is_empty(), "Expected no removed fields");
        assert!(comparison.modified.is_empty(), "Expected no modified fields");
        assert!(comparison.added.is_empty(), "Expected no added fields");
    }

    #[test]
    fn test_symdiff_simple_pair_value_modified() {
        let schema = r#"types:
- name: stringPair
  map:
    fields:
    - name: key
      type:
        scalar: string
    - name: value
      type:
        namedType: __untyped_atomic_
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
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("stringPair");

        // Different value field
        let lhs = pt.from_yaml(r#"{"key":"foo","value":{}}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"key":"foo","value":1}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(comparison.removed.is_empty(), "Expected no removed fields");
        assert!(
            comparison.modified.has(&path(vec!["value"])),
            "Expected value to be modified"
        );
        assert!(comparison.added.is_empty(), "Expected no added fields");
    }

    #[test]
    fn test_symdiff_simple_pair_field_change() {
        let schema = r#"types:
- name: stringPair
  map:
    fields:
    - name: key
      type:
        scalar: string
    - name: value
      type:
        namedType: __untyped_atomic_
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
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("stringPair");

        // Key removed, value added
        let lhs = pt.from_yaml(r#"{"key":"foo"}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"value":true}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(
            comparison.removed.has(&path(vec!["key"])),
            "Expected key to be removed"
        );
        assert!(comparison.modified.is_empty(), "Expected no modified fields");
        assert!(
            comparison.added.has(&path(vec!["value"])),
            "Expected value to be added"
        );
    }

    #[test]
    fn test_symdiff_null_empty_map() {
        let schema = r#"types:
- name: nestedMap
  map:
    fields:
    - name: inner
      type:
        map:
          elementType:
            namedType: __untyped_atomic_
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
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("nestedMap");

        // Empty to inner empty map
        let lhs = pt.from_yaml(r#"{}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"inner":{}}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(comparison.removed.is_empty(), "Expected no removed fields");
        assert!(comparison.modified.is_empty(), "Expected no modified fields");
        assert!(
            comparison.added.has(&path(vec!["inner"])),
            "Expected inner to be added"
        );
    }

    #[test]
    fn test_symdiff_null_vs_empty_map() {
        let schema = r#"types:
- name: nestedMap
  map:
    fields:
    - name: inner
      type:
        map:
          elementType:
            namedType: __untyped_atomic_
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
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("nestedMap");

        // null vs empty map is modification
        let lhs = pt.from_yaml(r#"{"inner":null}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"inner":{}}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(comparison.removed.is_empty(), "Expected no removed fields");
        assert!(
            comparison.modified.has(&path(vec!["inner"])),
            "Expected inner to be modified"
        );
        assert!(comparison.added.is_empty(), "Expected no added fields");
    }

    #[test]
    fn test_symdiff_map_merge() {
        let schema = r#"types:
- name: nestedMap
  map:
    elementType:
      namedType: nestedMap
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("nestedMap");

        // Same maps
        let lhs = pt.from_yaml(r#"{"a":{},"b":{}}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"a":{},"b":{}}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(comparison.removed.is_empty());
        assert!(comparison.modified.is_empty());
        assert!(comparison.added.is_empty());
    }

    #[test]
    fn test_symdiff_map_key_swap() {
        let schema = r#"types:
- name: nestedMap
  map:
    elementType:
      namedType: nestedMap
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("nestedMap");

        // Key a removed, key b added
        let lhs = pt.from_yaml(r#"{"a":{}}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"b":{}}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(
            comparison.removed.has(&path(vec!["a"])),
            "Expected a to be removed"
        );
        assert!(comparison.modified.is_empty());
        assert!(
            comparison.added.has(&path(vec!["b"])),
            "Expected b to be added"
        );
    }

    #[test]
    fn test_symdiff_nested_map_removal() {
        let schema = r#"types:
- name: nestedMap
  map:
    elementType:
      namedType: nestedMap
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("nestedMap");

        // Nested key removed
        let lhs = pt.from_yaml(r#"{"a":{"b":{"c":{}}}}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"a":{"b":{}}}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        // Create path for a.b.c
        let abc_path = Path::from_elements(vec![
            PathElement::field_name("a"),
            PathElement::field_name("b"),
            PathElement::field_name("c"),
        ]);

        assert!(
            comparison.removed.has(&abc_path),
            "Expected a.b.c to be removed"
        );
        assert!(comparison.modified.is_empty());
        assert!(comparison.added.is_empty());
    }

    #[test]
    fn test_symdiff_nested_map_addition() {
        let schema = r#"types:
- name: nestedMap
  map:
    elementType:
      namedType: nestedMap
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("nestedMap");

        // Nested key added
        let lhs = pt.from_yaml(r#"{"a":{}}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"a":{"b":{}}}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        // Create path for a.b
        let ab_path = Path::from_elements(vec![
            PathElement::field_name("a"),
            PathElement::field_name("b"),
        ]);

        assert!(comparison.removed.is_empty());
        assert!(comparison.modified.is_empty());
        assert!(
            comparison.added.has(&ab_path),
            "Expected a.b to be added"
        );
    }

    #[test]
    fn test_symdiff_struct_numeric_change() {
        let schema = r#"types:
- name: myStruct
  map:
    fields:
    - name: numeric
      type:
        scalar: numeric
    - name: string
      type:
        scalar: string
    - name: bool
      type:
        scalar: boolean
    - name: setStr
      type:
        list:
          elementType:
            scalar: string
          elementRelationship: associative
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("myStruct");

        // Numeric value changed
        let lhs = pt.from_yaml(r#"{"numeric":1}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"numeric":3.14159}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(comparison.removed.is_empty());
        assert!(
            comparison.modified.has(&path(vec!["numeric"])),
            "Expected numeric to be modified"
        );
        assert!(comparison.added.is_empty());
    }

    #[test]
    fn test_symdiff_struct_field_swap() {
        let schema = r#"types:
- name: myStruct
  map:
    fields:
    - name: numeric
      type:
        scalar: numeric
    - name: string
      type:
        scalar: string
    - name: bool
      type:
        scalar: boolean
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("myStruct");

        // string removed, bool added
        let lhs = pt.from_yaml(r#"{"string":"aoeu"}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"bool":true}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(
            comparison.removed.has(&path(vec!["string"])),
            "Expected string to be removed"
        );
        assert!(comparison.modified.is_empty());
        assert!(
            comparison.added.has(&path(vec!["bool"])),
            "Expected bool to be added"
        );
    }

    #[test]
    fn test_symdiff_set_field_addition() {
        let schema = r#"types:
- name: myStruct
  map:
    fields:
    - name: setStr
      type:
        list:
          elementType:
            scalar: string
          elementRelationship: associative
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("myStruct");

        // Set element added
        let lhs = pt.from_yaml(r#"{"setStr":["a","b"]}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"setStr":["a","b","c"]}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        // Create path for setStr[v="c"]
        let c_path = Path::from_elements(vec![
            PathElement::field_name("setStr"),
            PathElement::value(Value::String("c".into())),
        ]);

        assert!(comparison.removed.is_empty());
        assert!(comparison.modified.is_empty());
        assert!(
            comparison.added.has(&c_path),
            "Expected setStr[v=\"c\"] to be added"
        );
    }

    #[test]
    fn test_symdiff_set_field_removal() {
        let schema = r#"types:
- name: myStruct
  map:
    fields:
    - name: setStr
      type:
        list:
          elementType:
            scalar: string
          elementRelationship: associative
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("myStruct");

        // All set elements removed
        let lhs = pt.from_yaml(r#"{"setStr":["a","b","c"]}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"setStr":[]}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        // Create paths for removed elements
        let a_path = Path::from_elements(vec![
            PathElement::field_name("setStr"),
            PathElement::value(Value::String("a".into())),
        ]);
        let b_path = Path::from_elements(vec![
            PathElement::field_name("setStr"),
            PathElement::value(Value::String("b".into())),
        ]);
        let c_path = Path::from_elements(vec![
            PathElement::field_name("setStr"),
            PathElement::value(Value::String("c".into())),
        ]);

        assert!(
            comparison.removed.has(&a_path),
            "Expected setStr[v=\"a\"] to be removed"
        );
        assert!(
            comparison.removed.has(&b_path),
            "Expected setStr[v=\"b\"] to be removed"
        );
        assert!(
            comparison.removed.has(&c_path),
            "Expected setStr[v=\"c\"] to be removed"
        );
        assert!(comparison.modified.is_empty());
        assert!(comparison.added.is_empty());
    }

    #[test]
    fn test_symdiff_associative_list_element_change() {
        let schema = r#"types:
- name: myRoot
  map:
    fields:
    - name: list
      type:
        namedType: myList
    - name: atomicList
      type:
        namedType: mySequence
- name: myList
  list:
    elementType:
      namedType: myElement
    elementRelationship: associative
    keys:
    - key
    - id
- name: mySequence
  list:
    elementType:
      scalar: string
    elementRelationship: atomic
- name: myElement
  map:
    fields:
    - name: key
      type:
        scalar: string
    - name: id
      type:
        scalar: numeric
    - name: value
      type:
        namedType: myValue
    - name: bv
      type:
        scalar: boolean
    - name: nv
      type:
        scalar: numeric
- name: myValue
  map:
    elementType:
      scalar: string
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("myRoot");

        // Value modified within list element
        let lhs = pt
            .from_yaml(r#"{"list":[{"key":"a","id":1,"value":{"a":"a"}}]}"#)
            .unwrap();
        let rhs = pt
            .from_yaml(r#"{"list":[{"key":"a","id":1,"value":{"a":"b"}}]}"#)
            .unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        // Create path for list[key=a,id=1].value.a
        let key_pe = key_element(vec![
            ("key", Value::String("a".into())),
            ("id", Value::Int(1)),
        ]);
        let value_a_path = Path::from_elements(vec![
            PathElement::field_name("list"),
            key_pe,
            PathElement::field_name("value"),
            PathElement::field_name("a"),
        ]);

        assert!(comparison.removed.is_empty());
        assert!(
            comparison.modified.has(&value_a_path),
            "Expected list[key=a,id=1].value.a to be modified"
        );
        assert!(comparison.added.is_empty());
    }

    #[test]
    fn test_symdiff_atomic_list_change() {
        let schema = r#"types:
- name: myRoot
  map:
    fields:
    - name: atomicList
      type:
        namedType: mySequence
- name: mySequence
  list:
    elementType:
      scalar: string
    elementRelationship: atomic
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("myRoot");

        // Atomic list null vs content is modification
        let lhs = pt.from_yaml(r#"{"atomicList":["a","a","a"]}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"atomicList":null}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(comparison.removed.is_empty());
        assert!(
            comparison.modified.has(&path(vec!["atomicList"])),
            "Expected atomicList to be modified"
        );
        assert!(comparison.added.is_empty());
    }

    #[test]
    fn test_symdiff_atomic_list_length_change() {
        let schema = r#"types:
- name: myRoot
  map:
    fields:
    - name: atomicList
      type:
        namedType: mySequence
- name: mySequence
  list:
    elementType:
      scalar: string
    elementRelationship: atomic
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("myRoot");

        // Atomic list content change is modification
        let lhs = pt.from_yaml(r#"{"atomicList":["a","a","a"]}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"atomicList":["a","a"]}"#).unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        assert!(comparison.removed.is_empty());
        assert!(
            comparison.modified.has(&path(vec!["atomicList"])),
            "Expected atomicList to be modified"
        );
        assert!(comparison.added.is_empty());
    }

    #[test]
    fn test_symdiff_reverse_symmetry() {
        // Test that compare is symmetric for removed/added
        let schema = r#"types:
- name: myStruct
  map:
    fields:
    - name: a
      type:
        scalar: string
    - name: b
      type:
        scalar: string
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("myStruct");

        let lhs = pt.from_yaml(r#"{"a":"1"}"#).unwrap();
        let rhs = pt.from_yaml(r#"{"b":"2"}"#).unwrap();

        let forward = lhs.compare(&rhs).unwrap();
        let reverse = rhs.compare(&lhs).unwrap();

        // Forward: a removed, b added
        assert!(forward.removed.has(&path(vec!["a"])));
        assert!(forward.added.has(&path(vec!["b"])));

        // Reverse: b removed, a added
        assert!(reverse.removed.has(&path(vec!["b"])));
        assert!(reverse.added.has(&path(vec!["a"])));

        // Modified should be the same both ways
        assert!(forward.modified.is_empty());
        assert!(reverse.modified.is_empty());
    }

    #[test]
    fn test_symdiff_with_duplicates() {
        // Test with duplicate elements in set (using AllowDuplicates)
        let schema = r#"types:
- name: myStruct
  map:
    fields:
    - name: setStr
      type:
        list:
          elementType:
            scalar: string
          elementRelationship: associative
"#;
        let parser = Parser::new(schema).unwrap();
        let pt = parser.type_by_name("myStruct");

        // With duplicates - both before and after deduplication we see same values
        let lhs = pt
            .from_yaml_with_opts(r#"{"setStr":["a"]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        let rhs = pt
            .from_yaml_with_opts(r#"{"setStr":["a","b","b"]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();

        let comparison = lhs.compare(&rhs).unwrap();

        // b should be added
        let b_path = Path::from_elements(vec![
            PathElement::field_name("setStr"),
            PathElement::value(Value::String("b".into())),
        ]);

        assert!(comparison.removed.is_empty());
        assert!(comparison.modified.is_empty());
        assert!(
            comparison.added.has(&b_path),
            "Expected setStr[v=\"b\"] to be added"
        );
    }
}

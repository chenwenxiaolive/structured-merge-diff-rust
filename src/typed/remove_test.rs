//! Tests for RemoveItems and ExtractItems functionality.
//!
//! Based on Go tests from typed/remove_test.go

#[cfg(test)]
mod tests {
    use crate::fieldpath::{Path, PathElement, Set};
    use crate::typed::{Parser, ValidationOption};
    use crate::value::{Field, FieldList, Value};

    /// Helper to create a Set from paths.
    fn new_set(paths: Vec<Path>) -> Set {
        let mut set = Set::new();
        for path in paths {
            set.insert(&path);
        }
        set
    }

    /// Helper to create a path from elements.
    fn path(elements: Vec<PathElement>) -> Path {
        Path::from_elements(elements)
    }

    /// Helper to create a field name path element.
    fn field(name: &str) -> PathElement {
        PathElement::field_name(name)
    }

    /// Helper to create a value path element.
    fn value(v: Value) -> PathElement {
        PathElement::value(v)
    }

    /// Helper to create a key path element with fields.
    fn key_by_fields(fields: Vec<(&str, Value)>) -> PathElement {
        let mut sorted_fields: Vec<_> = fields
            .into_iter()
            .map(|(name, value)| Field {
                name: name.to_string(),
                value,
            })
            .collect();
        sorted_fields.sort_by(|a, b| a.name.cmp(&b.name));
        PathElement::key(FieldList {
            fields: sorted_fields,
        })
    }

    const SIMPLE_PAIR_SCHEMA: &str = r#"types:
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

    const STRUCT_GRAB_BAG_SCHEMA: &str = r#"types:
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
    - name: setBool
      type:
        list:
          elementType:
            scalar: boolean
          elementRelationship: associative
    - name: setNumeric
      type:
        list:
          elementType:
            scalar: numeric
          elementRelationship: associative
"#;

    const ASSOCIATIVE_AND_ATOMIC_SCHEMA: &str = r#"types:
- name: myRoot
  map:
    fields:
    - name: list
      type:
        namedType: myList
    - name: atomicList
      type:
        namedType: mySequence
    - name: atomicMap
      type:
        namedType: myAtomicMap
- name: myList
  list:
    elementType:
      namedType: myElement
    elementRelationship: associative
    keys:
    - key
    - id
- name: myAtomicMap
  map:
    elementType:
      scalar: string
    elementRelationship: atomic
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

    const NESTED_TYPES_SCHEMA: &str = r#"types:
- name: type
  map:
    fields:
      - name: listOfLists
        type:
          namedType: listOfLists
      - name: listOfMaps
        type:
          namedType: listOfMaps
      - name: mapOfLists
        type:
          namedType: mapOfLists
      - name: mapOfMaps
        type:
          namedType: mapOfMaps
      - name: mapOfMapsRecursive
        type:
          namedType: mapOfMapsRecursive
      - name: struct
        type:
          namedType: struct
- name: struct
  map:
    fields:
    - name: name
      type:
        scalar: string
    - name: value
      type:
        scalar: numeric
- name: listOfLists
  list:
    elementType:
      map:
        fields:
        - name: name
          type:
            scalar: string
        - name: value
          type:
            namedType: list
    elementRelationship: associative
    keys:
    - name
- name: list
  list:
    elementType:
      scalar: string
    elementRelationship: associative
- name: listOfMaps
  list:
    elementType:
      map:
        fields:
        - name: name
          type:
            scalar: string
        - name: value
          type:
            namedType: map
    elementRelationship: associative
    keys:
    - name
- name: map
  map:
    elementType:
      scalar: string
    elementRelationship: associative
- name: mapOfLists
  map:
    elementType:
      namedType: list
    elementRelationship: associative
- name: mapOfMaps
  map:
    elementType:
      namedType: map
    elementRelationship: associative
- name: mapOfMapsRecursive
  map:
    elementType:
      namedType: mapOfMapsRecursive
    elementRelationship: associative
"#;

    #[test]
    fn test_remove_simple_pair() {
        let parser = Parser::new(SIMPLE_PAIR_SCHEMA).unwrap();
        let pt = parser.type_by_name("stringPair");

        // Test: remove key from {key: foo}
        let tv = pt.from_yaml(r#"{"key":"foo"}"#).unwrap();
        let set = new_set(vec![path(vec![field("key")])]);
        let removed = tv.remove_items(&set);
        // Should be empty after removing 'key'
        assert!(removed.value().as_map().map(|m| m.is_empty()).unwrap_or(false) || removed.value().is_null());

        // Extracted should be {key: foo}
        let extracted = tv.extract_items(&set);
        let expected = pt.from_yaml(r#"{"key":"foo"}"#).unwrap();
        assert_eq!(extracted.value(), expected.value());

        // Test: remove nothing from {key: foo}
        let tv = pt.from_yaml(r#"{"key":"foo"}"#).unwrap();
        let set = new_set(vec![]);
        let removed = tv.remove_items(&set);
        let expected = pt.from_yaml(r#"{"key":"foo"}"#).unwrap();
        assert_eq!(removed.value(), expected.value());

        // Extracted should be empty
        let extracted = tv.extract_items(&set);
        assert!(extracted.value().as_map().map(|m| m.is_empty()).unwrap_or(false) || extracted.value().is_null());

        // Test: remove key from {key: foo, value: true}
        let tv = pt.from_yaml(r#"{"key":"foo","value":true}"#).unwrap();
        let set = new_set(vec![path(vec![field("key")])]);
        let removed = tv.remove_items(&set);
        let expected = pt.from_yaml(r#"{"value":true}"#).unwrap();
        assert_eq!(removed.value(), expected.value());

        // Extracted should be {key: foo}
        let extracted = tv.extract_items(&set);
        let expected = pt.from_yaml(r#"{"key":"foo"}"#).unwrap();
        assert_eq!(extracted.value(), expected.value());

        // Test: remove value from {key: foo, value: {a: b}}
        let tv = pt.from_yaml(r#"{"key":"foo","value":{"a": "b"}}"#).unwrap();
        let set = new_set(vec![path(vec![field("value")])]);
        let removed = tv.remove_items(&set);
        let expected = pt.from_yaml(r#"{"key":"foo"}"#).unwrap();
        assert_eq!(removed.value(), expected.value());

        // Extracted should be {value: {a: b}}
        let extracted = tv.extract_items(&set);
        let expected = pt.from_yaml(r#"{"value":{"a": "b"}}"#).unwrap();
        assert_eq!(extracted.value(), expected.value());
    }

    #[test]
    fn test_remove_struct_grab_bag() {
        let parser = Parser::new(STRUCT_GRAB_BAG_SCHEMA).unwrap();
        let pt = parser.type_by_name("myStruct");

        // Test: remove setBool[false] from {setBool: [false]}
        let tv = pt
            .from_yaml_with_opts(r#"{"setBool":[false]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        let set = new_set(vec![path(vec![field("setBool"), value(Value::Bool(false))])]);
        let removed = tv.remove_items(&set);
        // Should have setBool with empty list or null
        let removed_val = removed.value();
        if let Value::Map(m) = &removed_val {
            if let Some(set_bool) = m.get("setBool") {
                assert!(set_bool.is_null() || (set_bool.is_list() && set_bool.as_list().unwrap().is_empty()));
            }
        }

        // Extracted should be {setBool: [false]}
        let extracted = tv.extract_items(&set);
        let expected = pt
            .from_yaml_with_opts(r#"{"setBool":[false]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        assert_eq!(extracted.value(), expected.value());

        // Test: remove setBool[true] from {setBool: [false]} (not present)
        let tv = pt
            .from_yaml_with_opts(r#"{"setBool":[false]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        let set = new_set(vec![path(vec![field("setBool"), value(Value::Bool(true))])]);
        let removed = tv.remove_items(&set);
        // Should still have setBool: [false]
        let expected = pt
            .from_yaml_with_opts(r#"{"setBool":[false]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        assert_eq!(removed.value(), expected.value());

        // Extracted should be {setBool: null}
        let extracted = tv.extract_items(&set);
        if let Value::Map(m) = extracted.value() {
            if let Some(set_bool) = m.get("setBool") {
                assert!(set_bool.is_null() || (set_bool.is_list() && set_bool.as_list().unwrap().is_empty()));
            }
        }

        // Test: remove setBool[true] from {setBool: [true, false]}
        let tv = pt
            .from_yaml_with_opts(r#"{"setBool":[true,false]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        let set = new_set(vec![path(vec![field("setBool"), value(Value::Bool(true))])]);
        let removed = tv.remove_items(&set);
        let expected = pt
            .from_yaml_with_opts(r#"{"setBool":[false]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        assert_eq!(removed.value(), expected.value());

        // Extracted should be {setBool: [true]}
        let extracted = tv.extract_items(&set);
        let expected = pt
            .from_yaml_with_opts(r#"{"setBool":[true]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        assert_eq!(extracted.value(), expected.value());

        // Test: remove setStr["a"] from {setStr: ["a", "b", "c"]}
        let tv = pt.from_yaml(r#"{"setStr":["a","b","c"]}"#).unwrap();
        let set = new_set(vec![path(vec![field("setStr"), value(Value::String("a".into()))])]);
        let removed = tv.remove_items(&set);
        let expected = pt.from_yaml(r#"{"setStr":["b","c"]}"#).unwrap();
        assert_eq!(removed.value(), expected.value());

        // Extracted should be {setStr: ["a"]}
        let extracted = tv.extract_items(&set);
        let expected = pt.from_yaml(r#"{"setStr":["a"]}"#).unwrap();
        assert_eq!(extracted.value(), expected.value());

        // Test: remove multiple from setNumeric
        let tv = pt.from_yaml(r#"{"setNumeric":[1,2,3,4.5]}"#).unwrap();
        let set = new_set(vec![
            path(vec![field("setNumeric"), value(Value::Int(1))]),
            path(vec![field("setNumeric"), value(Value::Float(4.5))]),
        ]);
        let removed = tv.remove_items(&set);
        let expected = pt.from_yaml(r#"{"setNumeric":[2,3]}"#).unwrap();
        assert_eq!(removed.value(), expected.value());

        // Extracted should be {setNumeric: [1, 4.5]}
        let extracted = tv.extract_items(&set);
        let expected = pt.from_yaml(r#"{"setNumeric":[1,4.5]}"#).unwrap();
        assert_eq!(extracted.value(), expected.value());
    }

    #[test]
    fn test_remove_associative_list() {
        let parser = Parser::new(ASSOCIATIVE_AND_ATOMIC_SCHEMA).unwrap();
        let pt = parser.type_by_name("myRoot");

        // Test: extract key and id from a struct in associative list
        let tv = pt
            .from_yaml(r#"{"list":[{"key":"a","id":1},{"key":"a","id":2},{"key":"b","id":1}]}"#)
            .unwrap();
        let set = new_set(vec![
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(1)), ("key", Value::String("a".into()))]),
                field("key"),
            ]),
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(1)), ("key", Value::String("a".into()))]),
                field("id"),
            ]),
        ]);

        // Extracted should be {list: [{key: "a", id: 1}]}
        let extracted = tv.extract_items(&set);
        let expected = pt.from_yaml(r#"{"list":[{"key":"a","id":1}]}"#).unwrap();
        assert_eq!(
            extracted.value(),
            expected.value(),
            "got {:?}, expected {:?}",
            extracted.value(),
            expected.value()
        );

        // Test: remove struct from associative list
        let tv = pt
            .from_yaml(r#"{"list":[{"key":"a","id":1},{"key":"a","id":2},{"key":"b","id":1}]}"#)
            .unwrap();
        let set = new_set(vec![path(vec![
            field("list"),
            key_by_fields(vec![("id", Value::Int(1)), ("key", Value::String("a".into()))]),
        ])]);
        let removed = tv.remove_items(&set);
        let expected = pt
            .from_yaml(r#"{"list":[{"key":"a","id":2},{"key":"b","id":1}]}"#)
            .unwrap();
        assert_eq!(
            removed.value(),
            expected.value(),
            "got {:?}, expected {:?}",
            removed.value(),
            expected.value()
        );
    }

    #[test]
    fn test_remove_atomic_list() {
        let parser = Parser::new(ASSOCIATIVE_AND_ATOMIC_SCHEMA).unwrap();
        let pt = parser.type_by_name("myRoot");

        // Test: remove atomicList
        let tv = pt
            .from_yaml_with_opts(r#"{"atomicList":["a", "a", "a"]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        let set = new_set(vec![path(vec![field("atomicList")])]);
        let removed = tv.remove_items(&set);
        // Should be empty after removing atomicList
        assert!(
            removed.value().as_map().map(|m| m.is_empty()).unwrap_or(false) || removed.value().is_null()
        );

        // Extracted should be {atomicList: ["a", "a", "a"]}
        let extracted = tv.extract_items(&set);
        let expected = pt
            .from_yaml_with_opts(r#"{"atomicList":["a", "a", "a"]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        assert_eq!(extracted.value(), expected.value());
    }

    #[test]
    fn test_remove_atomic_map() {
        let parser = Parser::new(ASSOCIATIVE_AND_ATOMIC_SCHEMA).unwrap();
        let pt = parser.type_by_name("myRoot");

        // Test: remove atomicMap
        let tv = pt.from_yaml(r#"{"atomicMap":{"a": "c", "b": "d"}}"#).unwrap();
        let set = new_set(vec![path(vec![field("atomicMap")])]);
        let removed = tv.remove_items(&set);
        // Should be empty after removing atomicMap
        assert!(
            removed.value().as_map().map(|m| m.is_empty()).unwrap_or(false) || removed.value().is_null()
        );

        // Extracted should be {atomicMap: {a: c, b: d}}
        let extracted = tv.extract_items(&set);
        let expected = pt.from_yaml(r#"{"atomicMap":{"a": "c", "b": "d"}}"#).unwrap();
        assert_eq!(extracted.value(), expected.value());
    }

    #[test]
    fn test_remove_nested_types_list_of_lists() {
        let parser = Parser::new(NESTED_TYPES_SCHEMA).unwrap();
        let pt = parser.type_by_name("type");

        // Test: extract everything from listOfLists
        let tv = pt
            .from_yaml(r#"{"listOfLists": [{"name": "a", "value": ["b", "c"]}, {"name": "d"}]}"#)
            .unwrap();
        let set = new_set(vec![
            path(vec![
                field("listOfLists"),
                key_by_fields(vec![("name", Value::String("a".into()))]),
                field("name"),
            ]),
            path(vec![
                field("listOfLists"),
                key_by_fields(vec![("name", Value::String("a".into()))]),
                field("value"),
                value(Value::String("b".into())),
            ]),
            path(vec![
                field("listOfLists"),
                key_by_fields(vec![("name", Value::String("a".into()))]),
                field("value"),
                value(Value::String("c".into())),
            ]),
            path(vec![
                field("listOfLists"),
                key_by_fields(vec![("name", Value::String("d".into()))]),
                field("name"),
            ]),
        ]);
        let extracted = tv.extract_items(&set);
        let expected = pt
            .from_yaml(r#"{"listOfLists": [{"name": "a", "value": ["b", "c"]}, {"name": "d"}]}"#)
            .unwrap();
        assert_eq!(
            extracted.value(),
            expected.value(),
            "got {:?}, expected {:?}",
            extracted.value(),
            expected.value()
        );

        // Test: remove a top-level element from listOfLists
        let tv = pt
            .from_yaml(r#"{"listOfLists": [{"name": "a", "value": ["b", "c"]}, {"name": "d"}]}"#)
            .unwrap();
        let set = new_set(vec![path(vec![
            field("listOfLists"),
            key_by_fields(vec![("name", Value::String("d".into()))]),
        ])]);
        let removed = tv.remove_items(&set);
        let expected = pt
            .from_yaml(r#"{"listOfLists": [{"name": "a", "value": ["b", "c"]}]}"#)
            .unwrap();
        assert_eq!(
            removed.value(),
            expected.value(),
            "got {:?}, expected {:?}",
            removed.value(),
            expected.value()
        );
    }

    #[test]
    fn test_remove_nested_types_map_of_maps() {
        let parser = Parser::new(NESTED_TYPES_SCHEMA).unwrap();
        let pt = parser.type_by_name("type");

        // Test: extract everything from mapOfMaps
        let tv = pt
            .from_yaml(r#"{"mapOfMaps": {"b":{"a":"x","c":"z"}, "d":{"e":"y", "f":"w"}}}"#)
            .unwrap();
        let set = new_set(vec![
            path(vec![field("mapOfMaps"), field("b"), field("a")]),
            path(vec![field("mapOfMaps"), field("b"), field("c")]),
            path(vec![field("mapOfMaps"), field("d"), field("e")]),
            path(vec![field("mapOfMaps"), field("d"), field("f")]),
        ]);
        let extracted = tv.extract_items(&set);
        let expected = pt
            .from_yaml(r#"{"mapOfMaps": {"b":{"a":"x","c":"z"}, "d":{"e":"y", "f":"w"}}}"#)
            .unwrap();
        assert_eq!(extracted.value(), expected.value());

        // Test: remove a top-level element from mapOfMaps
        let set = new_set(vec![path(vec![field("mapOfMaps"), field("b")])]);
        let removed = tv.remove_items(&set);
        let expected = pt
            .from_yaml(r#"{"mapOfMaps": {"d":{"e":"y", "f":"w"}}}"#)
            .unwrap();
        assert_eq!(removed.value(), expected.value());

        // Test: extract just one leaf
        let set = new_set(vec![path(vec![field("mapOfMaps"), field("b"), field("a")])]);
        let extracted = tv.extract_items(&set);
        let expected = pt.from_yaml(r#"{"mapOfMaps": {"b":{"a":"x"}}}"#).unwrap();
        assert_eq!(extracted.value(), expected.value());

        // Test: remove just one leaf
        let removed = tv.remove_items(&set);
        let expected = pt
            .from_yaml(r#"{"mapOfMaps": {"b":{"c":"z"},"d":{"e":"y", "f":"w"}}}"#)
            .unwrap();
        assert_eq!(removed.value(), expected.value());
    }

    #[test]
    fn test_remove_nested_types_map_of_lists() {
        let parser = Parser::new(NESTED_TYPES_SCHEMA).unwrap();
        let pt = parser.type_by_name("type");

        // Test: remove a top-level element from mapOfLists
        let tv = pt
            .from_yaml(r#"{"mapOfLists": {"b":["a","c"], "d":["e", "f"]}}"#)
            .unwrap();
        let set = new_set(vec![path(vec![field("mapOfLists"), field("b")])]);
        let removed = tv.remove_items(&set);
        let expected = pt
            .from_yaml(r#"{"mapOfLists": {"d":["e", "f"]}}"#)
            .unwrap();
        assert_eq!(removed.value(), expected.value());

        // Test: remove just one leaf from list
        let set = new_set(vec![path(vec![
            field("mapOfLists"),
            field("b"),
            value(Value::String("a".into())),
        ])]);
        let removed = tv.remove_items(&set);
        let expected = pt
            .from_yaml(r#"{"mapOfLists":{"b":["c"],"d":["e", "f"]}}"#)
            .unwrap();
        assert_eq!(removed.value(), expected.value());

        // Test: extract just one leaf from list
        let extracted = tv.extract_items(&set);
        let expected = pt
            .from_yaml(r#"{"mapOfLists":{"b":["a"]}}"#)
            .unwrap();
        assert_eq!(extracted.value(), expected.value());
    }

    #[test]
    fn test_remove_recursive_map() {
        let parser = Parser::new(NESTED_TYPES_SCHEMA).unwrap();
        let pt = parser.type_by_name("type");

        // Test: remove root element
        let tv = pt
            .from_yaml(r#"{"mapOfMapsRecursive": {"a":{"b":{"c":null}}}}"#)
            .unwrap();
        let set = new_set(vec![path(vec![field("mapOfMapsRecursive")])]);
        let removed = tv.remove_items(&set);
        assert!(
            removed.value().as_map().map(|m| m.is_empty()).unwrap_or(false) || removed.value().is_null()
        );

        // Test: remove second-level map
        let set = new_set(vec![path(vec![
            field("mapOfMapsRecursive"),
            field("a"),
            field("b"),
        ])]);
        let removed = tv.remove_items(&set);
        let expected = pt
            .from_yaml(r#"{"mapOfMapsRecursive":{"a":null}}"#)
            .unwrap();
        assert_eq!(removed.value(), expected.value());

        // Test: remove third-level map
        let set = new_set(vec![path(vec![
            field("mapOfMapsRecursive"),
            field("a"),
            field("b"),
            field("c"),
        ])]);
        let removed = tv.remove_items(&set);
        let expected = pt
            .from_yaml(r#"{"mapOfMapsRecursive":{"a":{"b":null}}}"#)
            .unwrap();
        assert_eq!(removed.value(), expected.value());
    }
}

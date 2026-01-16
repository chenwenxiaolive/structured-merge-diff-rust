//! Tests for to_field_set functionality.
//!
//! Based on Go tests from typed/toset_test.go

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
        let field_list = FieldList {
            fields: fields
                .into_iter()
                .map(|(name, value)| Field {
                    name: name.to_string(),
                    value,
                })
                .collect(),
        };
        PathElement::key(field_list)
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
    - name: color
      type:
        map:
          fields:
          - name: R
            type:
              scalar: numeric
          - name: G
            type:
              scalar: numeric
          - name: B
            type:
              scalar: numeric
          elementRelationship: atomic
    - name: arbitraryWavelengthColor
      type:
        map:
          elementType:
            scalar: numeric
          elementRelationship: atomic
    - name: args
      type:
        list:
          elementType:
            map:
              fields:
              - name: key
                type:
                  scalar: string
              - name: value
                type:
                  scalar: string
          elementRelationship: atomic
"#;

    const ASSOCIATIVE_LIST_SCHEMA: &str = r#"types:
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

    #[test]
    fn test_toset_simple_pair() {
        let parser = Parser::new(SIMPLE_PAIR_SCHEMA).unwrap();
        let pt = parser.type_by_name("stringPair");

        // Test: {"key":"foo","value":1}
        let tv = pt.from_yaml(r#"{"key":"foo","value":1}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("key")]), path(vec![field("value")])]);
        assert!(fs.equals(&expected), "got {:?}, expected {:?}", fs, expected);

        // Test: {"key":"foo","value":{"a": "b"}}
        let tv = pt.from_yaml(r#"{"key":"foo","value":{"a": "b"}}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("key")]), path(vec![field("value")])]);
        assert!(fs.equals(&expected));

        // Test: {"key":"foo","value":null}
        let tv = pt.from_yaml(r#"{"key":"foo","value":null}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("key")]), path(vec![field("value")])]);
        assert!(fs.equals(&expected));

        // Test: {"key":"foo"}
        let tv = pt.from_yaml(r#"{"key":"foo"}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("key")])]);
        assert!(fs.equals(&expected));

        // Test: {"key":"foo","value":true}
        let tv = pt.from_yaml(r#"{"key":"foo","value":true}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("key")]), path(vec![field("value")])]);
        assert!(fs.equals(&expected));
    }

    #[test]
    fn test_toset_struct_grab_bag() {
        let parser = Parser::new(STRUCT_GRAB_BAG_SCHEMA).unwrap();
        let pt = parser.type_by_name("myStruct");

        // Test numeric
        let tv = pt.from_yaml(r#"{"numeric":1}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("numeric")])]);
        assert!(fs.equals(&expected));

        // Test float
        let tv = pt.from_yaml(r#"{"numeric":3.14159}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("numeric")])]);
        assert!(fs.equals(&expected));

        // Test string
        let tv = pt.from_yaml(r#"{"string":"aoeu"}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("string")])]);
        assert!(fs.equals(&expected));

        // Test bool true
        let tv = pt.from_yaml(r#"{"bool":true}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("bool")])]);
        assert!(fs.equals(&expected));

        // Test bool false
        let tv = pt.from_yaml(r#"{"bool":false}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("bool")])]);
        assert!(fs.equals(&expected));

        // Test setStr
        let tv = pt.from_yaml(r#"{"setStr":["a","b","c"]}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![
            path(vec![field("setStr"), value(Value::String("a".into()))]),
            path(vec![field("setStr"), value(Value::String("b".into()))]),
            path(vec![field("setStr"), value(Value::String("c".into()))]),
        ]);
        assert!(fs.equals(&expected));

        // Test setBool
        let tv = pt
            .from_yaml_with_opts(r#"{"setBool":[true,false,true]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![
            path(vec![field("setBool"), value(Value::Bool(true))]),
            path(vec![field("setBool"), value(Value::Bool(false))]),
        ]);
        assert!(fs.equals(&expected));

        // Test setNumeric
        let tv = pt.from_yaml(r#"{"setNumeric":[1,2,3,3.14159]}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![
            path(vec![field("setNumeric"), value(Value::Int(1))]),
            path(vec![field("setNumeric"), value(Value::Int(2))]),
            path(vec![field("setNumeric"), value(Value::Int(3))]),
            path(vec![field("setNumeric"), value(Value::Float(3.14159))]),
        ]);
        assert!(fs.equals(&expected));

        // Test color (atomic map)
        let tv = pt.from_yaml(r#"{"color":{}}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("color")])]);
        assert!(fs.equals(&expected));

        let tv = pt.from_yaml(r#"{"color":null}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("color")])]);
        assert!(fs.equals(&expected));

        let tv = pt.from_yaml(r#"{"color":{"R":255,"G":0,"B":0}}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("color")])]);
        assert!(fs.equals(&expected));

        // Test arbitraryWavelengthColor (atomic map with element type)
        let tv = pt.from_yaml(r#"{"arbitraryWavelengthColor":{}}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("arbitraryWavelengthColor")])]);
        assert!(fs.equals(&expected));

        let tv = pt.from_yaml(r#"{"arbitraryWavelengthColor":null}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("arbitraryWavelengthColor")])]);
        assert!(fs.equals(&expected));

        let tv = pt.from_yaml(r#"{"arbitraryWavelengthColor":{"IR":255}}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("arbitraryWavelengthColor")])]);
        assert!(fs.equals(&expected));

        // Test args (atomic list)
        let tv = pt.from_yaml(r#"{"args":[]}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("args")])]);
        assert!(fs.equals(&expected));

        let tv = pt.from_yaml(r#"{"args":null}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("args")])]);
        assert!(fs.equals(&expected));

        let tv = pt.from_yaml(r#"{"args":[{"key":"a","value":"b"},{"key":"c","value":"d"}]}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("args")])]);
        assert!(fs.equals(&expected));
    }

    #[test]
    fn test_toset_associative_list() {
        let parser = Parser::new(ASSOCIATIVE_LIST_SCHEMA).unwrap();
        let pt = parser.type_by_name("myRoot");

        // Test empty list
        let tv = pt.from_yaml(r#"{"list":[]}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![]);
        assert!(fs.equals(&expected));

        // Test list with one element
        let tv = pt.from_yaml(r#"{"list":[{"key":"a","id":1,"value":{"a":"a"}}]}"#).unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(1)), ("key", Value::String("a".into()))]),
            ]),
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
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(1)), ("key", Value::String("a".into()))]),
                field("value"),
                field("a"),
            ]),
        ]);
        assert!(fs.equals(&expected), "got {:?}, expected {:?}", fs, expected);

        // Test list with multiple elements
        let tv = pt
            .from_yaml(r#"{"list":[{"key":"a","id":1},{"key":"a","id":2},{"key":"b","id":1}]}"#)
            .unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(1)), ("key", Value::String("a".into()))]),
            ]),
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(2)), ("key", Value::String("a".into()))]),
            ]),
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(1)), ("key", Value::String("b".into()))]),
            ]),
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
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(2)), ("key", Value::String("a".into()))]),
                field("key"),
            ]),
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(2)), ("key", Value::String("a".into()))]),
                field("id"),
            ]),
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(1)), ("key", Value::String("b".into()))]),
                field("key"),
            ]),
            path(vec![
                field("list"),
                key_by_fields(vec![("id", Value::Int(1)), ("key", Value::String("b".into()))]),
                field("id"),
            ]),
        ]);
        assert!(fs.equals(&expected));

        // Test atomic list
        let tv = pt
            .from_yaml_with_opts(r#"{"atomicList":["a","a","a"]}"#, &[ValidationOption::AllowDuplicates])
            .unwrap();
        let fs = tv.to_field_set().unwrap();
        let expected = new_set(vec![path(vec![field("atomicList")])]);
        assert!(fs.equals(&expected));
    }
}

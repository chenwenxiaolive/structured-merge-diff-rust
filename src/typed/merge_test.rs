//! Tests for typed merge operations.
//!
//! Based on Go tests from typed/merge_test.go

#[cfg(test)]
mod tests {
    use crate::typed::{Parser, ValidationOption};

    /// Test case for merge operations.
    struct MergeTestCase {
        name: &'static str,
        root_type_name: &'static str,
        schema: &'static str,
        triplets: Vec<MergeTriplet>,
    }

    struct MergeTriplet {
        lhs: &'static str,
        rhs: &'static str,
        out: &'static str,
    }

    fn run_merge_test_case(tc: MergeTestCase) {
        let parser = Parser::new(tc.schema)
            .expect(&format!("Failed to parse schema for test: {}", tc.name));

        let pt = parser.type_by_name(tc.root_type_name);

        for (i, triplet) in tc.triplets.iter().enumerate() {
            // Parse with AllowDuplicates for lhs (former object may have duplicates in sets)
            let lhs = pt.from_yaml_with_opts(triplet.lhs, &[ValidationOption::AllowDuplicates])
                .expect(&format!("Failed to parse lhs for {}-{}: {}", tc.name, i, triplet.lhs));

            let rhs = pt.from_yaml(triplet.rhs)
                .expect(&format!("Failed to parse rhs for {}-{}: {}", tc.name, i, triplet.rhs));

            let expected = pt.from_yaml_with_opts(triplet.out, &[ValidationOption::AllowDuplicates])
                .expect(&format!("Failed to parse out for {}-{}: {}", tc.name, i, triplet.out));

            let result = lhs.merge(&rhs);
            assert!(result.is_ok(), "Merge failed for {}-{}: {:?}", tc.name, i, result.err());

            let merged = result.unwrap();
            assert_eq!(
                merged.value(),
                expected.value(),
                "Merge result mismatch for {}-{}.\nLHS: {}\nRHS: {}\nExpected: {}\nGot: {:?}",
                tc.name, i, triplet.lhs, triplet.rhs, triplet.out, merged.value()
            );
        }
    }

    #[test]
    fn test_merge_simple_pair() {
        run_merge_test_case(MergeTestCase {
            name: "simple pair",
            root_type_name: "stringPair",
            schema: r#"types:
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
"#,
            triplets: vec![
                MergeTriplet {
                    lhs: r#"{"key":"foo","value":{}}"#,
                    rhs: r#"{"key":"foo","value":1}"#,
                    out: r#"{"key":"foo","value":1}"#,
                },
                MergeTriplet {
                    lhs: r#"{"key":"foo","value":1}"#,
                    rhs: r#"{"key":"foo","value":{}}"#,
                    out: r#"{"key":"foo","value":{}}"#,
                },
                MergeTriplet {
                    lhs: r#"{"key":"foo","value":null}"#,
                    rhs: r#"{"key":"foo","value":{}}"#,
                    out: r#"{"key":"foo","value":{}}"#,
                },
                MergeTriplet {
                    lhs: r#"{"key":"foo"}"#,
                    rhs: r#"{"value":true}"#,
                    out: r#"{"key":"foo","value":true}"#,
                },
            ],
        });
    }

    #[test]
    fn test_merge_null_empty_map() {
        run_merge_test_case(MergeTestCase {
            name: "null/empty map",
            root_type_name: "nestedMap",
            schema: r#"types:
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
"#,
            triplets: vec![
                MergeTriplet {
                    lhs: r#"{}"#,
                    rhs: r#"{"inner":{}}"#,
                    out: r#"{"inner":{}}"#,
                },
                MergeTriplet {
                    lhs: r#"{}"#,
                    rhs: r#"{"inner":null}"#,
                    out: r#"{"inner":null}"#,
                },
                MergeTriplet {
                    lhs: r#"{"inner":null}"#,
                    rhs: r#"{"inner":{}}"#,
                    out: r#"{"inner":{}}"#,
                },
                MergeTriplet {
                    lhs: r#"{"inner":{}}"#,
                    rhs: r#"{"inner":null}"#,
                    out: r#"{"inner":null}"#,
                },
                MergeTriplet {
                    lhs: r#"{"inner":{}}"#,
                    rhs: r#"{"inner":{}}"#,
                    out: r#"{"inner":{}}"#,
                },
            ],
        });
    }

    #[test]
    fn test_merge_null_empty_struct() {
        run_merge_test_case(MergeTestCase {
            name: "null/empty struct",
            root_type_name: "nestedStruct",
            schema: r#"types:
- name: nestedStruct
  map:
    fields:
    - name: inner
      type:
        map:
          fields:
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
"#,
            triplets: vec![
                MergeTriplet {
                    lhs: r#"{}"#,
                    rhs: r#"{"inner":{}}"#,
                    out: r#"{"inner":{}}"#,
                },
                MergeTriplet {
                    lhs: r#"{}"#,
                    rhs: r#"{"inner":null}"#,
                    out: r#"{"inner":null}"#,
                },
                MergeTriplet {
                    lhs: r#"{"inner":null}"#,
                    rhs: r#"{"inner":{}}"#,
                    out: r#"{"inner":{}}"#,
                },
                MergeTriplet {
                    lhs: r#"{"inner":{}}"#,
                    rhs: r#"{"inner":null}"#,
                    out: r#"{"inner":null}"#,
                },
                MergeTriplet {
                    lhs: r#"{"inner":{}}"#,
                    rhs: r#"{"inner":{}}"#,
                    out: r#"{"inner":{}}"#,
                },
            ],
        });
    }

    #[test]
    fn test_merge_null_empty_list() {
        run_merge_test_case(MergeTestCase {
            name: "null/empty list",
            root_type_name: "nestedList",
            schema: r#"types:
- name: nestedList
  map:
    fields:
    - name: inner
      type:
        list:
          elementType:
            namedType: __untyped_atomic_
          elementRelationship: atomic
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
"#,
            triplets: vec![
                MergeTriplet {
                    lhs: r#"{}"#,
                    rhs: r#"{"inner":[]}"#,
                    out: r#"{"inner":[]}"#,
                },
                MergeTriplet {
                    lhs: r#"{}"#,
                    rhs: r#"{"inner":null}"#,
                    out: r#"{"inner":null}"#,
                },
                MergeTriplet {
                    lhs: r#"{"inner":null}"#,
                    rhs: r#"{"inner":[]}"#,
                    out: r#"{"inner":[]}"#,
                },
                MergeTriplet {
                    lhs: r#"{"inner":[]}"#,
                    rhs: r#"{"inner":null}"#,
                    out: r#"{"inner":null}"#,
                },
                MergeTriplet {
                    lhs: r#"{"inner":[]}"#,
                    rhs: r#"{"inner":[]}"#,
                    out: r#"{"inner":[]}"#,
                },
            ],
        });
    }

    #[test]
    fn test_merge_struct_grab_bag() {
        run_merge_test_case(MergeTestCase {
            name: "struct grab bag",
            root_type_name: "myStruct",
            schema: r#"types:
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
"#,
            triplets: vec![
                // Numeric changes
                MergeTriplet {
                    lhs: r#"{"numeric":1}"#,
                    rhs: r#"{"numeric":3.14159}"#,
                    out: r#"{"numeric":3.14159}"#,
                },
                MergeTriplet {
                    lhs: r#"{"numeric":3.14159}"#,
                    rhs: r#"{"numeric":1}"#,
                    out: r#"{"numeric":1}"#,
                },
                // Add different field
                MergeTriplet {
                    lhs: r#"{"string":"aoeu"}"#,
                    rhs: r#"{"bool":true}"#,
                    out: r#"{"string":"aoeu","bool":true}"#,
                },
                // Set operations - union semantics
                MergeTriplet {
                    lhs: r#"{"setStr":["a","b","c"]}"#,
                    rhs: r#"{"setStr":["a","b"]}"#,
                    out: r#"{"setStr":["a","b","c"]}"#,
                },
                MergeTriplet {
                    lhs: r#"{"setStr":["a","b"]}"#,
                    rhs: r#"{"setStr":["a","b","c"]}"#,
                    out: r#"{"setStr":["a","b","c"]}"#,
                },
                // Empty set preserves
                MergeTriplet {
                    lhs: r#"{"setStr":["a","b","c"]}"#,
                    rhs: r#"{"setStr":[]}"#,
                    out: r#"{"setStr":["a","b","c"]}"#,
                },
                MergeTriplet {
                    lhs: r#"{"setStr":[]}"#,
                    rhs: r#"{"setStr":["a","b","c"]}"#,
                    out: r#"{"setStr":["a","b","c"]}"#,
                },
                // Order from RHS
                MergeTriplet {
                    lhs: r#"{"setStr":["a","b"]}"#,
                    rhs: r#"{"setStr":["b","a"]}"#,
                    out: r#"{"setStr":["b","a"]}"#,
                },
                // Disjoint sets
                MergeTriplet {
                    lhs: r#"{"setStr":["a","b","c"]}"#,
                    rhs: r#"{"setStr":["d","e","f"]}"#,
                    out: r#"{"setStr":["a","b","c","d","e","f"]}"#,
                },
                // Bool set
                MergeTriplet {
                    lhs: r#"{"setBool":[true]}"#,
                    rhs: r#"{"setBool":[false]}"#,
                    out: r#"{"setBool":[true,false]}"#,
                },
            ],
        });
    }

    #[test]
    fn test_merge_associative_list() {
        run_merge_test_case(MergeTestCase {
            name: "associative list",
            root_type_name: "myRoot",
            schema: r#"types:
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
"#,
            triplets: vec![
                // Same key/id - merge
                MergeTriplet {
                    lhs: r#"{"list":[{"key":"a","id":1,"value":{"a":"a"}}]}"#,
                    rhs: r#"{"list":[{"key":"a","id":1,"value":{"a":"a"}}]}"#,
                    out: r#"{"list":[{"key":"a","id":1,"value":{"a":"a"}}]}"#,
                },
                // Different id - add
                MergeTriplet {
                    lhs: r#"{"list":[{"key":"a","id":1,"value":{"a":"a"}}]}"#,
                    rhs: r#"{"list":[{"key":"a","id":2,"value":{"a":"a"}}]}"#,
                    out: r#"{"list":[{"key":"a","id":1,"value":{"a":"a"}},{"key":"a","id":2,"value":{"a":"a"}}]}"#,
                },
                // Atomic list - replace
                MergeTriplet {
                    lhs: r#"{"atomicList":["a","a","a"]}"#,
                    rhs: r#"{"atomicList":null}"#,
                    out: r#"{"atomicList":null}"#,
                },
                MergeTriplet {
                    lhs: r#"{"atomicList":["a","b","c"]}"#,
                    rhs: r#"{"atomicList":[]}"#,
                    out: r#"{"atomicList":[]}"#,
                },
                MergeTriplet {
                    lhs: r#"{"atomicList":["a","a","a"]}"#,
                    rhs: r#"{"atomicList":["a","a"]}"#,
                    out: r#"{"atomicList":["a","a"]}"#,
                },
            ],
        });
    }

    #[test]
    fn test_merge_set_ordering() {
        // Test that set ordering from RHS is preserved
        run_merge_test_case(MergeTestCase {
            name: "set ordering",
            root_type_name: "myStruct",
            schema: r#"types:
- name: myStruct
  map:
    fields:
    - name: setStr
      type:
        list:
          elementType:
            scalar: string
          elementRelationship: associative
"#,
            triplets: vec![
                // Overlapping sets with reordering
                MergeTriplet {
                    lhs: r#"{"setStr":["c","a","g","f"]}"#,
                    rhs: r#"{"setStr":["c","f","a","g"]}"#,
                    out: r#"{"setStr":["c","f","a","g"]}"#,
                },
                MergeTriplet {
                    lhs: r#"{"setStr":["a","b","c","d"]}"#,
                    rhs: r#"{"setStr":["d","e","f","a"]}"#,
                    out: r#"{"setStr":["b","c","d","e","f","a"]}"#,
                },
            ],
        });
    }

    #[test]
    fn test_merge_set_with_duplicates() {
        // Test handling of duplicates in sets (LHS may have duplicates from old data)
        run_merge_test_case(MergeTestCase {
            name: "set with duplicates",
            root_type_name: "myStruct",
            schema: r#"types:
- name: myStruct
  map:
    fields:
    - name: setStr
      type:
        list:
          elementType:
            scalar: string
          elementRelationship: associative
"#,
            triplets: vec![
                // Duplicate in LHS, not in RHS
                MergeTriplet {
                    lhs: r#"{"setStr":["a","b","b"]}"#,
                    rhs: r#"{"setStr":["c"]}"#,
                    out: r#"{"setStr":["a","b","b","c"]}"#,
                },
                // Duplicate in LHS, also in RHS
                MergeTriplet {
                    lhs: r#"{"setStr":["a","b","b"]}"#,
                    rhs: r#"{"setStr":["b"]}"#,
                    out: r#"{"setStr":["a","b"]}"#,
                },
                // Multiple duplicates in LHS
                MergeTriplet {
                    lhs: r#"{"setStr":["a","a","b","b"]}"#,
                    rhs: r#"{"setStr":["b","c","d"]}"#,
                    out: r#"{"setStr":["a","a","b","c","d"]}"#,
                },
                // All duplicates in LHS, empty RHS
                MergeTriplet {
                    lhs: r#"{"setStr":["a","a","b","b"]}"#,
                    rhs: r#"{"setStr":[]}"#,
                    out: r#"{"setStr":["a","a","b","b"]}"#,
                },
            ],
        });
    }

    #[test]
    fn test_merge_list_reordering() {
        // Test more complex list reordering scenarios
        run_merge_test_case(MergeTestCase {
            name: "list reordering",
            root_type_name: "myRoot",
            schema: r#"types:
- name: myRoot
  map:
    fields:
    - name: list
      type:
        namedType: myList
- name: myList
  list:
    elementType:
      namedType: myElement
    elementRelationship: associative
    keys:
    - key
    - id
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
- name: myValue
  map:
    elementType:
      scalar: string
"#,
            triplets: vec![
                // Add item in middle
                MergeTriplet {
                    lhs: r#"{"list":[{"key":"b","id":2}]}"#,
                    rhs: r#"{"list":[{"key":"a","id":1},{"key":"b","id":2},{"key":"c","id":3}]}"#,
                    out: r#"{"list":[{"key":"a","id":1},{"key":"b","id":2},{"key":"c","id":3}]}"#,
                },
                // Reorder existing items
                MergeTriplet {
                    lhs: r#"{"list":[{"key":"a","id":1},{"key":"b","id":2},{"key":"c","id":3}]}"#,
                    rhs: r#"{"list":[{"key":"c","id":3},{"key":"b","id":2}]}"#,
                    out: r#"{"list":[{"key":"a","id":1},{"key":"c","id":3},{"key":"b","id":2}]}"#,
                },
                MergeTriplet {
                    lhs: r#"{"list":[{"key":"a","id":1},{"key":"b","id":2},{"key":"c","id":3}]}"#,
                    rhs: r#"{"list":[{"key":"c","id":3},{"key":"a","id":1}]}"#,
                    out: r#"{"list":[{"key":"b","id":2},{"key":"c","id":3},{"key":"a","id":1}]}"#,
                },
            ],
        });
    }
}

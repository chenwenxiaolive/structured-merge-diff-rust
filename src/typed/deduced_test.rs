//! Tests for deduced type operations.
//!
//! Based on Go tests from typed/deduced_test.go

#[cfg(test)]
mod tests {
    use crate::fieldpath::{Path, PathElement, Set};
    use crate::typed::deduced_parseable_type;

    /// Helper to create a path from field names.
    fn path(elements: Vec<&str>) -> Path {
        let path_elements: Vec<PathElement> = elements
            .into_iter()
            .map(|s| PathElement::field_name(s))
            .collect();
        Path::from_elements(path_elements)
    }

    /// Helper to create a set from paths.
    fn set_from_paths(paths: Vec<Path>) -> Set {
        let mut set = Set::new();
        for p in paths {
            set.insert(&p);
        }
        set
    }

    // ============ Validate Deduced Type Tests ============

    #[test]
    fn test_validate_deduced_null() {
        let pt = deduced_parseable_type();
        let result = pt.from_yaml(r#"{"a": null}"#);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let tv = result.unwrap();
        let validation = tv.validate(&[]);
        assert!(validation.is_ok(), "Validation failed: {:?}", validation.err());
    }

    #[test]
    fn test_validate_deduced_list() {
        let pt = deduced_parseable_type();
        let result = pt.from_yaml(r#"{"a": ["a", "b"]}"#);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let tv = result.unwrap();
        let validation = tv.validate(&[]);
        assert!(validation.is_ok(), "Validation failed: {:?}", validation.err());
    }

    #[test]
    fn test_validate_deduced_nested() {
        let pt = deduced_parseable_type();
        let result = pt.from_yaml(r#"{"a": {"b": [], "c": 2, "d": {"f": "string"}}}"#);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let tv = result.unwrap();
        let validation = tv.validate(&[]);
        assert!(validation.is_ok(), "Validation failed: {:?}", validation.err());
    }

    // ============ Merge Deduced Tests ============

    struct MergeTriplet {
        lhs: &'static str,
        rhs: &'static str,
        out: &'static str,
    }

    fn test_merge_triplet(triplet: MergeTriplet) {
        let pt = deduced_parseable_type();

        let lhs = pt.from_yaml(triplet.lhs)
            .expect(&format!("Failed to parse lhs: {}", triplet.lhs));
        let rhs = pt.from_yaml(triplet.rhs)
            .expect(&format!("Failed to parse rhs: {}", triplet.rhs));
        let expected = pt.from_yaml(triplet.out)
            .expect(&format!("Failed to parse out: {}", triplet.out));

        let result = lhs.merge(&rhs);
        assert!(result.is_ok(), "Merge failed: {:?}", result.err());

        let merged = result.unwrap();
        assert_eq!(
            merged.value(),
            expected.value(),
            "Merge result mismatch.\nLHS: {}\nRHS: {}\nExpected: {}\nGot: {:?}",
            triplet.lhs, triplet.rhs, triplet.out, merged.value()
        );
    }

    #[test]
    fn test_merge_deduced_type_change_empty_to_scalar() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"key":"foo","value":{}}"#,
            rhs: r#"{"key":"foo","value":1}"#,
            out: r#"{"key":"foo","value":1}"#,
        });
    }

    #[test]
    fn test_merge_deduced_type_change_scalar_to_empty() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"key":"foo","value":1}"#,
            rhs: r#"{"key":"foo","value":{}}"#,
            out: r#"{"key":"foo","value":{}}"#,
        });
    }

    #[test]
    fn test_merge_deduced_null_to_empty() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"key":"foo","value":null}"#,
            rhs: r#"{"key":"foo","value":{}}"#,
            out: r#"{"key":"foo","value":{}}"#,
        });
    }

    #[test]
    fn test_merge_deduced_add_field() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"key":"foo"}"#,
            rhs: r#"{"value":true}"#,
            out: r#"{"key":"foo","value":true}"#,
        });
    }

    #[test]
    fn test_merge_deduced_empty_to_inner() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{}"#,
            rhs: r#"{"inner":{}}"#,
            out: r#"{"inner":{}}"#,
        });
    }

    #[test]
    fn test_merge_deduced_empty_to_inner_null() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{}"#,
            rhs: r#"{"inner":null}"#,
            out: r#"{"inner":null}"#,
        });
    }

    #[test]
    fn test_merge_deduced_inner_null_to_empty() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"inner":null}"#,
            rhs: r#"{"inner":{}}"#,
            out: r#"{"inner":{}}"#,
        });
    }

    #[test]
    fn test_merge_deduced_inner_empty_to_null() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"inner":{}}"#,
            rhs: r#"{"inner":null}"#,
            out: r#"{"inner":null}"#,
        });
    }

    #[test]
    fn test_merge_deduced_inner_empty_same() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"inner":{}}"#,
            rhs: r#"{"inner":{}}"#,
            out: r#"{"inner":{}}"#,
        });
    }

    #[test]
    fn test_merge_deduced_empty_to_inner_list() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{}"#,
            rhs: r#"{"inner":[]}"#,
            out: r#"{"inner":[]}"#,
        });
    }

    #[test]
    fn test_merge_deduced_inner_null_to_list() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"inner":null}"#,
            rhs: r#"{"inner":[]}"#,
            out: r#"{"inner":[]}"#,
        });
    }

    #[test]
    fn test_merge_deduced_inner_list_to_null() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"inner":[]}"#,
            rhs: r#"{"inner":null}"#,
            out: r#"{"inner":null}"#,
        });
    }

    #[test]
    fn test_merge_deduced_inner_list_same() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"inner":[]}"#,
            rhs: r#"{"inner":[]}"#,
            out: r#"{"inner":[]}"#,
        });
    }

    #[test]
    fn test_merge_deduced_numeric_int_to_float() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"numeric":1}"#,
            rhs: r#"{"numeric":3.14159}"#,
            out: r#"{"numeric":3.14159}"#,
        });
    }

    #[test]
    fn test_merge_deduced_numeric_float_to_int() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"numeric":3.14159}"#,
            rhs: r#"{"numeric":1}"#,
            out: r#"{"numeric":1}"#,
        });
    }

    #[test]
    fn test_merge_deduced_add_different_field() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"string":"aoeu"}"#,
            rhs: r#"{"bool":true}"#,
            out: r#"{"string":"aoeu","bool":true}"#,
        });
    }

    #[test]
    fn test_merge_deduced_atomic_list_replace_shorter() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"atomic":["a","b","c"]}"#,
            rhs: r#"{"atomic":["a","b"]}"#,
            out: r#"{"atomic":["a","b"]}"#,
        });
    }

    #[test]
    fn test_merge_deduced_atomic_list_replace_longer() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"atomic":["a","b"]}"#,
            rhs: r#"{"atomic":["a","b","c"]}"#,
            out: r#"{"atomic":["a","b","c"]}"#,
        });
    }

    #[test]
    fn test_merge_deduced_atomic_list_replace_empty() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"atomic":["a","b","c"]}"#,
            rhs: r#"{"atomic":[]}"#,
            out: r#"{"atomic":[]}"#,
        });
    }

    #[test]
    fn test_merge_deduced_atomic_list_from_empty() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"atomic":[]}"#,
            rhs: r#"{"atomic":["a","b","c"]}"#,
            out: r#"{"atomic":["a","b","c"]}"#,
        });
    }

    #[test]
    fn test_merge_deduced_atomic_list_to_null() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"atomicList":["a","a","a"]}"#,
            rhs: r#"{"atomicList":null}"#,
            out: r#"{"atomicList":null}"#,
        });
    }

    #[test]
    fn test_merge_deduced_atomic_list_shorter() {
        test_merge_triplet(MergeTriplet {
            lhs: r#"{"atomicList":["a","a","a"]}"#,
            rhs: r#"{"atomicList":["a","a"]}"#,
            out: r#"{"atomicList":["a","a"]}"#,
        });
    }

    // ============ ToSet Deduced Tests ============

    fn test_to_set(yaml: &str, expected_paths: Vec<Vec<&str>>) {
        let pt = deduced_parseable_type();
        let tv = pt.from_yaml(yaml).expect(&format!("Failed to parse: {}", yaml));

        let fs = tv.to_field_set().expect("Failed to get field set");

        for path_elements in &expected_paths {
            let p = path(path_elements.clone());
            assert!(
                fs.has(&p),
                "Expected set to contain path {:?} for yaml: {}",
                path_elements,
                yaml
            );
        }
    }

    #[test]
    fn test_toset_deduced_simple() {
        test_to_set(
            r#"{"key":"foo","value":1}"#,
            vec![vec!["key"], vec!["value"]],
        );
    }

    #[test]
    fn test_toset_deduced_nested() {
        test_to_set(
            r#"{"key":"foo","value":{"a": "b"}}"#,
            vec![vec!["key"], vec!["value"], vec!["value", "a"]],
        );
    }

    #[test]
    fn test_toset_deduced_null() {
        test_to_set(
            r#"{"key":"foo","value":null}"#,
            vec![vec!["key"], vec!["value"]],
        );
    }

    #[test]
    fn test_toset_deduced_bool() {
        test_to_set(r#"{"bool":true}"#, vec![vec!["bool"]]);
        test_to_set(r#"{"bool":false}"#, vec![vec!["bool"]]);
    }

    #[test]
    fn test_toset_deduced_numeric() {
        test_to_set(r#"{"numeric":1}"#, vec![vec!["numeric"]]);
        test_to_set(r#"{"numeric":3.14159}"#, vec![vec!["numeric"]]);
    }

    #[test]
    fn test_toset_deduced_string() {
        test_to_set(r#"{"string":"aoeu"}"#, vec![vec!["string"]]);
    }

    #[test]
    fn test_toset_deduced_list() {
        // Lists are atomic in deduced schema, so we only track the list field itself
        test_to_set(r#"{"list":["a","b","c"]}"#, vec![vec!["list"]]);
    }

    #[test]
    fn test_toset_deduced_color_empty() {
        test_to_set(r#"{"color":{}}"#, vec![vec!["color"]]);
    }

    #[test]
    fn test_toset_deduced_color_null() {
        test_to_set(r#"{"color":null}"#, vec![vec!["color"]]);
    }

    #[test]
    fn test_toset_deduced_color_rgb() {
        test_to_set(
            r#"{"color":{"R":255,"G":0,"B":0}}"#,
            vec![vec!["color"], vec!["color", "R"], vec!["color", "G"], vec!["color", "B"]],
        );
    }

    #[test]
    fn test_toset_deduced_args_empty() {
        test_to_set(r#"{"args":[]}"#, vec![vec!["args"]]);
    }

    #[test]
    fn test_toset_deduced_args_null() {
        test_to_set(r#"{"args":null}"#, vec![vec!["args"]]);
    }

    // ============ Symdiff Deduced Tests ============

    struct SymdiffQuint {
        lhs: &'static str,
        rhs: &'static str,
        removed: Vec<Vec<&'static str>>,
        modified: Vec<Vec<&'static str>>,
        added: Vec<Vec<&'static str>>,
    }

    fn test_symdiff(quint: SymdiffQuint) {
        let pt = deduced_parseable_type();

        let lhs = pt.from_yaml(quint.lhs)
            .expect(&format!("Failed to parse lhs: {}", quint.lhs));
        let rhs = pt.from_yaml(quint.rhs)
            .expect(&format!("Failed to parse rhs: {}", quint.rhs));

        let result = lhs.compare(&rhs);
        assert!(result.is_ok(), "Compare failed: {:?}", result.err());

        let comparison = result.unwrap();

        // Check removed
        let expected_removed = set_from_paths(quint.removed.iter().map(|p| path(p.clone())).collect());
        assert!(
            comparison.removed.equals(&expected_removed),
            "Removed mismatch.\nExpected: {:?}\nGot: {:?}",
            expected_removed,
            comparison.removed
        );

        // Check modified
        let expected_modified = set_from_paths(quint.modified.iter().map(|p| path(p.clone())).collect());
        assert!(
            comparison.modified.equals(&expected_modified),
            "Modified mismatch.\nExpected: {:?}\nGot: {:?}",
            expected_modified,
            comparison.modified
        );

        // Check added
        let expected_added = set_from_paths(quint.added.iter().map(|p| path(p.clone())).collect());
        assert!(
            comparison.added.equals(&expected_added),
            "Added mismatch.\nExpected: {:?}\nGot: {:?}",
            expected_added,
            comparison.added
        );

        // Verify reverse operation gives symmetric results
        let reverse = rhs.compare(&lhs).expect("Reverse compare failed");
        assert!(
            reverse.modified.equals(&comparison.modified),
            "Reverse modified not symmetric"
        );
        assert!(
            reverse.removed.equals(&comparison.added),
            "Reverse removed should equal forward added"
        );
        assert!(
            reverse.added.equals(&comparison.removed),
            "Reverse added should equal forward removed"
        );
    }

    #[test]
    fn test_symdiff_deduced_same() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"key":"foo","value":1}"#,
            rhs: r#"{"key":"foo","value":1}"#,
            removed: vec![],
            modified: vec![],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_empty_to_scalar() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"key":"foo","value":{}}"#,
            rhs: r#"{"key":"foo","value":1}"#,
            removed: vec![],
            modified: vec![vec!["value"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_scalar_to_empty() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"key":"foo","value":1}"#,
            rhs: r#"{"key":"foo","value":{}}"#,
            removed: vec![],
            modified: vec![vec!["value"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_scalar_to_nested() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"key":"foo","value":1}"#,
            rhs: r#"{"key":"foo","value":{"deep":{"nested":1}}}"#,
            removed: vec![],
            modified: vec![vec!["value"]],
            added: vec![vec!["value", "deep"], vec!["value", "deep", "nested"]],
        });
    }

    #[test]
    fn test_symdiff_deduced_null_to_empty() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"key":"foo","value":null}"#,
            rhs: r#"{"key":"foo","value":{}}"#,
            removed: vec![],
            modified: vec![vec!["value"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_remove_add_field() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"key":"foo"}"#,
            rhs: r#"{"value":true}"#,
            removed: vec![vec!["key"]],
            modified: vec![],
            added: vec![vec!["value"]],
        });
    }

    #[test]
    fn test_symdiff_deduced_change_and_add_field() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"key":"foot"}"#,
            rhs: r#"{"key":"foo","value":true}"#,
            removed: vec![],
            modified: vec![vec!["key"]],
            added: vec![vec!["value"]],
        });
    }

    #[test]
    fn test_symdiff_deduced_add_inner() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{}"#,
            rhs: r#"{"inner":{}}"#,
            removed: vec![],
            modified: vec![],
            added: vec![vec!["inner"]],
        });
    }

    #[test]
    fn test_symdiff_deduced_add_inner_null() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{}"#,
            rhs: r#"{"inner":null}"#,
            removed: vec![],
            modified: vec![],
            added: vec![vec!["inner"]],
        });
    }

    #[test]
    fn test_symdiff_deduced_inner_null_to_empty() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"inner":null}"#,
            rhs: r#"{"inner":{}}"#,
            removed: vec![],
            modified: vec![vec!["inner"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_inner_empty_to_null() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"inner":{}}"#,
            rhs: r#"{"inner":null}"#,
            removed: vec![],
            modified: vec![vec!["inner"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_inner_same() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"inner":{}}"#,
            rhs: r#"{"inner":{}}"#,
            removed: vec![],
            modified: vec![],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_add_inner_list() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{}"#,
            rhs: r#"{"inner":[]}"#,
            removed: vec![],
            modified: vec![],
            added: vec![vec!["inner"]],
        });
    }

    #[test]
    fn test_symdiff_deduced_inner_null_to_list() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"inner":null}"#,
            rhs: r#"{"inner":[]}"#,
            removed: vec![],
            modified: vec![vec!["inner"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_inner_list_to_null() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"inner":[]}"#,
            rhs: r#"{"inner":null}"#,
            removed: vec![],
            modified: vec![vec!["inner"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_inner_list_same() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"inner":[]}"#,
            rhs: r#"{"inner":[]}"#,
            removed: vec![],
            modified: vec![],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_maps_same() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"a":{},"b":{}}"#,
            rhs: r#"{"a":{},"b":{}}"#,
            removed: vec![],
            modified: vec![],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_replace_map() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"a":{}}"#,
            rhs: r#"{"b":{}}"#,
            removed: vec![vec!["a"]],
            modified: vec![],
            added: vec![vec!["b"]],
        });
    }

    #[test]
    fn test_symdiff_deduced_remove_nested() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"a":{"b":{"c":{}}}}"#,
            rhs: r#"{"a":{"b":{}}}"#,
            removed: vec![vec!["a", "b", "c"]],
            modified: vec![],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_type_change_nested() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"a":{"b":{"c":[true]}}}"#,
            rhs: r#"{"a":{"b":[false]}}"#,
            removed: vec![vec!["a", "b", "c"]],
            modified: vec![vec!["a", "b"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_add_nested() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"a":{}}"#,
            rhs: r#"{"a":{"b":"true"}}"#,
            removed: vec![],
            modified: vec![],
            added: vec![vec!["a", "b"]],
        });
    }

    #[test]
    fn test_symdiff_deduced_numeric_change_int_to_float() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"numeric":1}"#,
            rhs: r#"{"numeric":3.14159}"#,
            removed: vec![],
            modified: vec![vec!["numeric"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_numeric_change_float_to_int() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"numeric":3.14159}"#,
            rhs: r#"{"numeric":1}"#,
            removed: vec![],
            modified: vec![vec!["numeric"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_replace_different_fields() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"string":"aoeu"}"#,
            rhs: r#"{"bool":true}"#,
            removed: vec![vec!["string"]],
            modified: vec![],
            added: vec![vec!["bool"]],
        });
    }

    #[test]
    fn test_symdiff_deduced_list_change() {
        // Lists are atomic in deduced schema
        test_symdiff(SymdiffQuint {
            lhs: r#"{"list":["a","b"]}"#,
            rhs: r#"{"list":["a","b","c"]}"#,
            removed: vec![],
            modified: vec![vec!["list"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_list_add() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{}"#,
            rhs: r#"{"list":[{"key":"a","id":1,"value":{"a":"a"}}]}"#,
            removed: vec![],
            modified: vec![],
            added: vec![vec!["list"]],
        });
    }

    #[test]
    fn test_symdiff_deduced_atomic_list_to_null() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"atomicList":["a","a","a"]}"#,
            rhs: r#"{"atomicList":null}"#,
            removed: vec![],
            modified: vec![vec!["atomicList"]],
            added: vec![],
        });
    }

    #[test]
    fn test_symdiff_deduced_atomic_list_shorter() {
        test_symdiff(SymdiffQuint {
            lhs: r#"{"atomicList":["a","a","a"]}"#,
            rhs: r#"{"atomicList":["a","a"]}"#,
            removed: vec![],
            modified: vec![vec!["atomicList"]],
            added: vec![],
        });
    }
}

//! Schema reconciliation for field sets.
//!
//! When schemas change (e.g., from granular to atomic), field sets need to be
//! reconciled to match the new schema structure.

use crate::fieldpath::{Path, PathElement, Set};
use crate::schema::{ElementRelationship, Map, TypeRef};
use crate::typed::TypedValue;

/// Reconciles a field set with changes to the object's schema.
///
/// Returns the reconciled field set, or None if no changes were made.
///
/// Supports:
/// - Changing types from atomic to granular
/// - Changing types from granular to atomic
pub fn reconcile_field_set_with_schema(
    fieldset: &Set,
    tv: &TypedValue,
) -> Result<Option<Set>, String> {
    let mut walker = ReconcileWalker {
        value: tv,
        fieldset: fieldset.clone(),
        type_ref: tv.type_ref().clone(),
        path: Path::new(),
        is_atomic: false,
        to_remove: None,
        to_add: None,
    };

    walker.reconcile()?;

    // If there are accumulated changes, apply them
    if walker.to_add.is_some() || walker.to_remove.is_some() {
        let mut out = fieldset.clone();
        if let Some(ref to_remove) = walker.to_remove {
            out = out.recursive_difference(to_remove);
        }
        if let Some(ref to_add) = walker.to_add {
            out = out.union(to_add);
        }
        Ok(Some(out))
    } else {
        Ok(None)
    }
}

struct ReconcileWalker<'a> {
    value: &'a TypedValue,
    fieldset: Set,
    type_ref: TypeRef,
    path: Path,
    is_atomic: bool,
    to_remove: Option<Set>,
    to_add: Option<Set>,
}

impl<'a> ReconcileWalker<'a> {
    fn reconcile(&mut self) -> Result<(), String> {
        let atom = match self.value.schema().resolve(&self.type_ref) {
            Some(a) => a,
            None => {
                return Err(format!("could not resolve {:?}", self.type_ref));
            }
        };

        // Handle based on atom type
        if atom.map.is_some() {
            self.do_map(atom.map.as_ref().unwrap())?;
        } else if atom.list.is_some() {
            self.do_list(atom.list.as_ref().unwrap())?;
        }
        // Scalars don't need reconciliation

        Ok(())
    }

    fn do_map(&mut self, map: &Map) -> Result<(), String> {
        // We don't reconcile deduced types (unstructured CRDs) or maps that contain
        // only unknown fields since deduced types do not yet support atomic/granular tags.
        if is_untyped_deduced_map(map) {
            return Ok(());
        }

        // Reconcile maps and structs changed from granular to atomic.
        // Note that migrations from atomic to granular are not recommended and will
        // be treated as if they were always granular.
        //
        // In this case, the manager that owned the previously atomic field (and all subfields),
        // will now own just the top-level field and none of the subfields.
        if !self.is_atomic && map.element_relationship == ElementRelationship::Atomic {
            if self.fieldset.size() > 0 {
                // Remove all root and children fields
                let mut to_remove = Set::new();
                to_remove.insert(&self.path);
                self.to_remove = Some(to_remove);

                // Add the root of the atomic
                let mut to_add = Set::new();
                to_add.insert(&self.path);
                self.to_add = Some(to_add);
            }
            return Ok(());
        }

        // Visit map items
        self.visit_map_items(map)?;
        Ok(())
    }

    fn do_list(&mut self, list: &crate::schema::List) -> Result<(), String> {
        // Reconcile lists changed from granular to atomic.
        if !self.is_atomic && list.element_relationship == ElementRelationship::Atomic {
            // Remove all root and children fields
            let mut to_remove = Set::new();
            to_remove.insert(&self.path);
            self.to_remove = Some(to_remove);

            // Add the root of the atomic
            let mut to_add = Set::new();
            to_add.insert(&self.path);
            self.to_add = Some(to_add);
            return Ok(());
        }

        // Visit list items
        self.visit_list_items(list)?;
        Ok(())
    }

    fn visit_map_items(&mut self, map: &Map) -> Result<(), String> {
        // Get the fieldset at the current path
        let current_set = self.get_fieldset_at_path();

        // Iterate through members and children
        let mut elements_to_visit: Vec<(PathElement, bool)> = Vec::new();

        // Collect children that are not members (intermediate paths)
        current_set.children_iterate(|pe| {
            if !current_set.members_has(pe) {
                elements_to_visit.push((pe.clone(), false));
            }
        });

        // Collect members
        current_set.members_iterate(|pe| {
            elements_to_visit.push((pe.clone(), true));
        });

        // Process each element
        for (pe, is_member) in elements_to_visit {
            // Get the type ref for this path element
            if let Some(tr) = type_ref_at_path(map, &pe) {
                let child_set = current_set.children_get(&pe).cloned().unwrap_or_else(Set::new);
                let has_children = !child_set.is_empty();

                let mut child_walker = ReconcileWalker {
                    value: self.value,
                    fieldset: self.fieldset.clone(), // Pass root fieldset, not child_set
                    type_ref: tr,
                    path: self.path.with(pe),
                    is_atomic: is_member && !has_children,
                    to_remove: None,
                    to_add: None,
                };

                child_walker.reconcile()?;

                // Merge accumulated changes
                self.merge_changes(&child_walker);
            }
        }

        Ok(())
    }

    fn visit_list_items(&mut self, list: &crate::schema::List) -> Result<(), String> {
        let current_set = self.get_fieldset_at_path();

        let mut elements_to_visit: Vec<(PathElement, bool)> = Vec::new();

        // Collect children that are not members
        current_set.children_iterate(|pe| {
            if !current_set.members_has(pe) {
                elements_to_visit.push((pe.clone(), false));
            }
        });

        // Collect members
        current_set.members_iterate(|pe| {
            elements_to_visit.push((pe.clone(), true));
        });

        // Process each element
        for (pe, is_member) in elements_to_visit {
            let child_set = current_set.children_get(&pe).cloned().unwrap_or_else(Set::new);
            let has_children = !child_set.is_empty();

            let mut child_walker = ReconcileWalker {
                value: self.value,
                fieldset: self.fieldset.clone(), // Pass root fieldset, not child_set
                type_ref: list.element_type.clone(),
                path: self.path.with(pe),
                is_atomic: is_member && !has_children,
                to_remove: None,
                to_add: None,
            };

            child_walker.reconcile()?;

            // Merge accumulated changes
            self.merge_changes(&child_walker);
        }

        Ok(())
    }

    fn get_fieldset_at_path(&self) -> Set {
        let mut current = self.fieldset.clone();
        for pe in self.path.as_slice() {
            current = current.children_get(pe).cloned().unwrap_or_else(Set::new);
        }
        current
    }

    fn merge_changes(&mut self, child: &ReconcileWalker) {
        if let Some(ref child_remove) = child.to_remove {
            if let Some(ref mut to_remove) = self.to_remove {
                *to_remove = to_remove.union(child_remove);
            } else {
                self.to_remove = Some(child_remove.clone());
            }
        }
        if let Some(ref child_add) = child.to_add {
            if let Some(ref mut to_add) = self.to_add {
                *to_add = to_add.union(child_add);
            } else {
                self.to_add = Some(child_add.clone());
            }
        }
    }
}

fn type_ref_at_path(map: &Map, pe: &PathElement) -> Option<TypeRef> {
    let tr = if let Some(name) = pe.as_field_name() {
        if let Some(field) = map.find_field(name) {
            field.field_type.clone()
        } else {
            map.element_type.clone()
        }
    } else {
        map.element_type.clone()
    };

    // Return None if the TypeRef is empty
    if tr.named_type.is_none()
        && tr.inlined.scalar.is_none()
        && tr.inlined.list.is_none()
        && tr.inlined.map.is_none()
    {
        None
    } else {
        Some(tr)
    }
}

/// Returns true if m has no fields defined, but allows untyped elements.
fn is_untyped_deduced_map(m: &Map) -> bool {
    is_untyped_deduced_ref(&m.element_type) && m.fields.is_empty()
}

fn is_untyped_deduced_ref(t: &TypeRef) -> bool {
    if let Some(ref name) = t.named_type {
        return name == "__untyped_deduced_";
    }
    if let Some(ref scalar) = t.inlined.scalar {
        return *scalar == crate::schema::Scalar::Untyped;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed::Parser;

    fn granular_schema(version: &str) -> String {
        format!(
            r#"types:
- name: {}
  map:
    fields:
      - name: struct
        type:
          namedType: struct
      - name: list
        type:
          namedType: list
      - name: objectList
        type:
          namedType: objectList
      - name: stringMap
        type:
          namedType: stringMap
      - name: unchanged
        type:
          namedType: unchanged
- name: struct
  map:
    fields:
    - name: numeric
      type:
        scalar: numeric
    - name: string
      type:
        scalar: string
- name: list
  list:
    elementType:
      scalar: string
    elementRelationship: associative
- name: objectList
  list:
    elementType:
      namedType: listItem
    elementRelationship: associative
    keys:
      - keyA
      - keyB
- name: listItem
  map:
    fields:
    - name: keyA
      type:
        scalar: string
    - name: keyB
      type:
        scalar: string
    - name: value
      type:
        scalar: string
- name: stringMap
  map:
    elementType:
      scalar: string
- name: unchanged
  map:
    fields:
    - name: numeric
      type:
        scalar: numeric
- name: empty
  map:
    elementType:
      scalar: untyped
      list:
        elementType:
          namedType: __untyped_atomic_
        elementRelationship: atomic
      map:
        elementType:
          namedType: __untyped_deduced_
        elementRelationship: separable
- name: emptyWithPreserveUnknown
  map:
    fields:
    - name: preserveField
      type:
        map:
          elementType:
            scalar: untyped
            list:
              elementType:
                namedType: __untyped_atomic_
              elementRelationship: atomic
            map:
              elementType:
                namedType: __untyped_deduced_
              elementRelationship: separable
- name: populatedWithPreserveUnknown
  map:
    fields:
    - name: preserveField
      type:
        map:
          fields:
          - name: list
            type:
              namedType: list
          elementType:
            namedType: __untyped_deduced_
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
"#,
            version
        )
    }

    fn atomic_schema(version: &str) -> String {
        format!(
            r#"types:
- name: {}
  map:
    fields:
      - name: struct
        type:
          namedType: struct
      - name: list
        type:
          namedType: list
      - name: objectList
        type:
          namedType: objectList
      - name: stringMap
        type:
          namedType: stringMap
      - name: unchanged
        type:
          namedType: unchanged
- name: struct
  map:
    fields:
    - name: numeric
      type:
        scalar: numeric
    - name: string
      type:
        scalar: string
    elementRelationship: atomic
- name: list
  list:
    elementType:
      scalar: string
    elementRelationship: atomic
- name: objectList
  list:
    elementType:
      namedType: listItem
    elementRelationship: atomic
- name: listItem
  map:
    fields:
    - name: keyA
      type:
        scalar: string
    - name: keyB
      type:
        scalar: string
    - name: value
      type:
        scalar: string
- name: stringMap
  map:
    elementType:
      scalar: string
    elementRelationship: atomic
- name: unchanged
  map:
    fields:
    - name: numeric
      type:
        scalar: numeric
- name: empty
  map:
    elementType:
      scalar: untyped
      list:
        elementType:
          namedType: __untyped_atomic_
        elementRelationship: atomic
      map:
        elementType:
          namedType: __untyped_deduced_
        elementRelationship: separable
- name: emptyWithPreserveUnknown
  map:
    fields:
    - name: preserveField
      type:
        map:
          elementType:
            scalar: untyped
            list:
              elementType:
                namedType: __untyped_atomic_
              elementRelationship: atomic
            map:
              elementType:
                namedType: __untyped_deduced_
              elementRelationship: separable
- name: populatedWithPreserveUnknown
  map:
    fields:
    - name: preserveField
      type:
        map:
          fields:
          - name: list
            type:
              namedType: list
          elementType:
            namedType: __untyped_deduced_
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
"#,
            version
        )
    }

    const BASIC_LIVE_OBJECT: &str = r#"
struct:
  numeric: 1
  string: "two"
list:
  - one
  - two
objectList:
  - keyA: a1
    keyB: b1
    value: v1
  - keyA: a2
    keyB: b2
    value: v2
stringMap:
  key1: value1
unchanged:
  numeric: 10
"#;

    fn path(elements: Vec<PathElement>) -> Path {
        Path::from_elements(elements)
    }

    fn field(name: &str) -> PathElement {
        PathElement::field_name(name)
    }

    fn value_elem(v: crate::value::Value) -> PathElement {
        PathElement::value(v)
    }

    fn key_by_fields(fields: Vec<(&str, crate::value::Value)>) -> PathElement {
        use crate::value::{Field, FieldList};
        PathElement::Key(FieldList {
            fields: fields
                .into_iter()
                .map(|(name, value)| Field {
                    name: name.to_string(),
                    value,
                })
                .collect(),
        })
    }

    fn new_set(paths: Vec<Path>) -> Set {
        let mut set = Set::new();
        for p in paths {
            set.insert(&p);
        }
        set
    }

    #[test]
    fn test_reconcile_granular_to_atomic() {
        use crate::value::Value;

        let new_schema = atomic_schema("v1");
        let parser = Parser::new(&new_schema).unwrap();
        let pt = parser.type_by_name("v1");
        let live_object = pt.from_yaml(BASIC_LIVE_OBJECT).unwrap();

        let old_fields = new_set(vec![
            path(vec![field("struct"), field("numeric")]),
            path(vec![field("list"), value_elem(Value::String("one".into()))]),
            path(vec![field("stringMap"), field("key1")]),
            path(vec![
                field("objectList"),
                key_by_fields(vec![
                    ("keyA", Value::String("a1".into())),
                    ("keyB", Value::String("b1".into())),
                ]),
                field("value"),
            ]),
            path(vec![field("unchanged"), field("numeric")]),
        ]);

        let fixed = reconcile_field_set_with_schema(&old_fields, &live_object).unwrap();
        assert!(fixed.is_some(), "Expected field set to change");

        let fixed = fixed.unwrap();
        let expected = new_set(vec![
            path(vec![field("struct")]),
            path(vec![field("list")]),
            path(vec![field("objectList")]),
            path(vec![field("stringMap")]),
            path(vec![field("unchanged"), field("numeric")]),
        ]);

        assert!(
            fixed.equals(&expected),
            "Expected:\n{:?}\nGot:\n{:?}",
            expected,
            fixed
        );
    }

    #[test]
    fn test_reconcile_no_change_granular() {
        use crate::value::Value;

        let schema = granular_schema("v1");
        let parser = Parser::new(&schema).unwrap();
        let pt = parser.type_by_name("v1");
        let live_object = pt.from_yaml(BASIC_LIVE_OBJECT).unwrap();

        let old_fields = new_set(vec![
            path(vec![field("struct"), field("numeric")]),
            path(vec![field("list"), value_elem(Value::String("one".into()))]),
            path(vec![
                field("objectList"),
                key_by_fields(vec![
                    ("keyA", Value::String("a1".into())),
                    ("keyB", Value::String("b1".into())),
                ]),
                field("value"),
            ]),
            path(vec![field("stringMap"), field("key1")]),
            path(vec![field("unchanged"), field("numeric")]),
        ]);

        let fixed = reconcile_field_set_with_schema(&old_fields, &live_object).unwrap();
        assert!(fixed.is_none(), "Expected no change");
    }

    #[test]
    fn test_reconcile_no_change_atomic() {
        let schema = atomic_schema("v1");
        let parser = Parser::new(&schema).unwrap();
        let pt = parser.type_by_name("v1");
        let live_object = pt.from_yaml(BASIC_LIVE_OBJECT).unwrap();

        let old_fields = new_set(vec![
            path(vec![field("struct")]),
            path(vec![field("list")]),
            path(vec![field("objectList")]),
            path(vec![field("unchanged"), field("numeric")]),
        ]);

        let fixed = reconcile_field_set_with_schema(&old_fields, &live_object).unwrap();
        assert!(fixed.is_none(), "Expected no change");
    }

    #[test]
    fn test_reconcile_no_change_empty_granular() {
        use crate::value::Value;

        let schema = granular_schema("v1");
        let parser = Parser::new(&schema).unwrap();
        let pt = parser.type_by_name("v1");
        let live_yaml = r#"
struct: {}
list: []
objectList:
  - keyA: a1
    keyB: b1
stringMap: {}
unchanged: {}
"#;
        let live_object = pt.from_yaml(live_yaml).unwrap();

        let old_fields = new_set(vec![
            path(vec![field("struct")]),
            path(vec![field("list")]),
            path(vec![field("objectList")]),
            path(vec![
                field("objectList"),
                key_by_fields(vec![
                    ("keyA", Value::String("a1".into())),
                    ("keyB", Value::String("b1".into())),
                ]),
            ]),
            path(vec![
                field("objectList"),
                key_by_fields(vec![
                    ("keyA", Value::String("a1".into())),
                    ("keyB", Value::String("b1".into())),
                ]),
                field("value"),
            ]),
            path(vec![field("unchanged")]),
        ]);

        let fixed = reconcile_field_set_with_schema(&old_fields, &live_object).unwrap();
        assert!(fixed.is_none(), "Expected no change");
    }

    #[test]
    fn test_reconcile_deduced() {
        let schema = granular_schema("v1");
        let parser = Parser::new(&schema).unwrap();
        let pt = parser.type_by_name("empty");
        let live_object = pt.from_yaml(BASIC_LIVE_OBJECT).unwrap();

        let old_fields = new_set(vec![
            path(vec![field("struct")]),
            path(vec![field("list")]),
            path(vec![field("objectList")]),
            path(vec![field("unchanged"), field("numeric")]),
        ]);

        let fixed = reconcile_field_set_with_schema(&old_fields, &live_object).unwrap();
        assert!(fixed.is_none(), "Expected no change for deduced type");
    }

    #[test]
    fn test_reconcile_empty_preserve_unknown() {
        let schema = granular_schema("v1");
        let parser = Parser::new(&schema).unwrap();
        let pt = parser.type_by_name("emptyWithPreserveUnknown");
        let live_yaml = r#"
preserveField:
  arbitrary: abc
"#;
        let live_object = pt.from_yaml(live_yaml).unwrap();

        let old_fields = new_set(vec![
            path(vec![field("preserveField")]),
            path(vec![field("preserveField"), field("arbitrary")]),
        ]);

        let fixed = reconcile_field_set_with_schema(&old_fields, &live_object).unwrap();
        assert!(
            fixed.is_none(),
            "Expected no change for preserve unknown type"
        );
    }

    #[test]
    fn test_reconcile_populated_preserve_unknown() {
        use crate::value::Value;

        let new_schema = atomic_schema("v1");
        let parser = Parser::new(&new_schema).unwrap();
        let pt = parser.type_by_name("populatedWithPreserveUnknown");
        let live_yaml = r#"
preserveField:
  arbitrary: abc
  list:
  - one
"#;
        let live_object = pt.from_yaml(live_yaml).unwrap();

        let old_fields = new_set(vec![
            path(vec![field("preserveField")]),
            path(vec![field("preserveField"), field("arbitrary")]),
            path(vec![field("preserveField"), field("list")]),
            path(vec![
                field("preserveField"),
                field("list"),
                value_elem(Value::String("one".into())),
            ]),
        ]);

        let fixed = reconcile_field_set_with_schema(&old_fields, &live_object).unwrap();
        assert!(fixed.is_some(), "Expected field set to change");

        let fixed = fixed.unwrap();
        let expected = new_set(vec![
            path(vec![field("preserveField")]),
            path(vec![field("preserveField"), field("arbitrary")]),
            path(vec![field("preserveField"), field("list")]),
        ]);

        assert!(
            fixed.equals(&expected),
            "Expected:\n{:?}\nGot:\n{:?}",
            expected,
            fixed
        );
    }
}

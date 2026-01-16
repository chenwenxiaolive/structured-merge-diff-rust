//! Conflict types for merge operations.

use crate::fieldpath::{ManagedFields, Path, Set};
use std::collections::BTreeMap;
use std::fmt;

/// Conflict represents a single field conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// The manager that owns the conflicting field.
    pub manager: String,
    /// The path to the conflicting field.
    pub path: Path,
}

impl Conflict {
    /// Creates a new conflict.
    pub fn new(manager: impl Into<String>, path: Path) -> Self {
        Conflict {
            manager: manager.into(),
            path,
        }
    }
}

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "conflict with manager '{}' at {}", self.manager, self.path)
    }
}

impl std::error::Error for Conflict {}

/// Conflicts is a collection of conflicts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Conflicts {
    conflicts: Vec<Conflict>,
}

impl Conflicts {
    /// Creates a new empty Conflicts collection.
    pub fn new() -> Self {
        Conflicts {
            conflicts: Vec::new(),
        }
    }

    /// Adds a conflict.
    pub fn add(&mut self, conflict: Conflict) {
        self.conflicts.push(conflict);
    }

    /// Returns true if there are no conflicts.
    pub fn is_empty(&self) -> bool {
        self.conflicts.is_empty()
    }

    /// Returns the number of conflicts.
    pub fn len(&self) -> usize {
        self.conflicts.len()
    }

    /// Returns an iterator over the conflicts.
    pub fn iter(&self) -> impl Iterator<Item = &Conflict> {
        self.conflicts.iter()
    }

    /// Converts the conflicts to a Set of paths.
    pub fn to_set(&self) -> Set {
        let mut set = Set::new();
        for conflict in &self.conflicts {
            set.insert(&conflict.path);
        }
        set
    }

    /// Returns the error message in Go-compatible format.
    /// Groups conflicts by manager, sorted alphabetically.
    pub fn error(&self) -> String {
        if self.conflicts.is_empty() {
            return String::new();
        }

        // Group by manager, using BTreeMap for sorted order
        let mut by_manager: BTreeMap<&str, Vec<&Path>> = BTreeMap::new();
        for conflict in &self.conflicts {
            by_manager
                .entry(&conflict.manager)
                .or_default()
                .push(&conflict.path);
        }

        // Sort paths within each manager
        for paths in by_manager.values_mut() {
            paths.sort_by_key(|p| p.to_string());
        }

        // Build output
        let mut result = String::new();
        for (i, (manager, paths)) in by_manager.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(&format!("conflicts with \"{}\":", manager));
            for path in paths {
                result.push_str(&format!("\n- {}", path));
            }
        }
        result
    }
}

impl IntoIterator for Conflicts {
    type Item = Conflict;
    type IntoIter = std::vec::IntoIter<Conflict>;

    fn into_iter(self) -> Self::IntoIter {
        self.conflicts.into_iter()
    }
}

impl fmt::Display for Conflicts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error())
    }
}

impl std::error::Error for Conflicts {}

/// Extracts conflicts from ManagedFields.
/// Creates a Conflict entry for each path owned by each manager.
pub fn conflicts_from_managers(managers: &ManagedFields) -> Conflicts {
    let mut conflicts = Conflicts::new();

    for (manager, vs) in managers.iter() {
        vs.set().iterate(|path| {
            conflicts.add(Conflict::new(manager.clone(), path.clone()));
        });
    }

    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fieldpath::{PathElement, VersionedSet, APIVersion};
    use crate::value::{Field, FieldList, Value};

    fn key_by_fields(fields: Vec<(&str, Value)>) -> PathElement {
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

    fn make_path(elements: Vec<PathElement>) -> Path {
        Path::from_elements(elements)
    }

    fn new_set(paths: Vec<Path>) -> Set {
        let mut set = Set::new();
        for path in paths {
            set.insert(&path);
        }
        set
    }

    #[test]
    fn test_conflict_display() {
        let conflict = Conflict::new(
            "manager1",
            Path::from_elements(vec![PathElement::field_name("field")]),
        );
        assert!(format!("{}", conflict).contains("manager1"));
    }

    #[test]
    fn test_conflicts_collection() {
        let mut conflicts = Conflicts::new();
        assert!(conflicts.is_empty());

        conflicts.add(Conflict::new("m1", Path::new()));
        assert!(!conflicts.is_empty());
        assert_eq!(conflicts.len(), 1);
    }

    // Test from Go: TestNewFromSets
    #[test]
    fn test_new_from_sets() {
        let mut managers = ManagedFields::new();

        let bob_set = new_set(vec![
            make_path(vec![PathElement::field_name("key")]),
            make_path(vec![
                PathElement::field_name("list"),
                key_by_fields(vec![("key", Value::String("a".to_string())), ("id", Value::Int(2))]),
                PathElement::field_name("id"),
            ]),
        ]);
        managers.insert("Bob", VersionedSet::new(bob_set, APIVersion::new("v1"), false));

        let alice_set = new_set(vec![
            make_path(vec![PathElement::field_name("value")]),
            make_path(vec![
                PathElement::field_name("list"),
                key_by_fields(vec![("key", Value::String("a".to_string())), ("id", Value::Int(2))]),
                PathElement::field_name("key"),
            ]),
        ]);
        managers.insert("Alice", VersionedSet::new(alice_set, APIVersion::new("v1"), false));

        let got = conflicts_from_managers(&managers);
        let wanted = r#"conflicts with "Alice":
- .list[id=2,key="a"].key
- .value
conflicts with "Bob":
- .key
- .list[id=2,key="a"].id"#;

        assert_eq!(got.error(), wanted, "Got:\n{}\nWanted:\n{}", got.error(), wanted);
    }

    // Test from Go: TestToSet
    #[test]
    fn test_to_set() {
        let mut managers = ManagedFields::new();

        let bob_set = new_set(vec![
            make_path(vec![PathElement::field_name("key")]),
            make_path(vec![
                PathElement::field_name("list"),
                key_by_fields(vec![("key", Value::String("a".to_string())), ("id", Value::Int(2))]),
                PathElement::field_name("id"),
            ]),
        ]);
        managers.insert("Bob", VersionedSet::new(bob_set, APIVersion::new("v1"), false));

        let alice_set = new_set(vec![
            make_path(vec![PathElement::field_name("value")]),
            make_path(vec![
                PathElement::field_name("list"),
                key_by_fields(vec![("key", Value::String("a".to_string())), ("id", Value::Int(2))]),
                PathElement::field_name("key"),
            ]),
        ]);
        managers.insert("Alice", VersionedSet::new(alice_set, APIVersion::new("v1"), false));

        let conflicts = conflicts_from_managers(&managers);
        let actual = conflicts.to_set();

        let expected = new_set(vec![
            make_path(vec![PathElement::field_name("key")]),
            make_path(vec![PathElement::field_name("value")]),
            make_path(vec![
                PathElement::field_name("list"),
                key_by_fields(vec![("key", Value::String("a".to_string())), ("id", Value::Int(2))]),
                PathElement::field_name("id"),
            ]),
            make_path(vec![
                PathElement::field_name("list"),
                key_by_fields(vec![("key", Value::String("a".to_string())), ("id", Value::Int(2))]),
                PathElement::field_name("key"),
            ]),
        ]);

        assert!(expected.equals(&actual), "Expected:\n{:?}\nActual:\n{:?}", expected, actual);
    }

    // Test from Go: TestConflictsFromManagers
    #[test]
    fn test_conflicts_from_managers() {
        let mut managers = ManagedFields::new();

        let bob_set = new_set(vec![
            make_path(vec![
                PathElement::field_name("obj"),
                PathElement::field_name("template"),
                PathElement::field_name("obj"),
                PathElement::field_name("list"),
                key_by_fields(vec![("name", Value::String("a".to_string()))]),
                PathElement::field_name("id"),
            ]),
            make_path(vec![
                PathElement::field_name("obj"),
                PathElement::field_name("template"),
                PathElement::field_name("obj"),
                PathElement::field_name("list"),
                key_by_fields(vec![("name", Value::String("a".to_string()))]),
                PathElement::field_name("key"),
            ]),
        ]);
        managers.insert("Bob", VersionedSet::new(bob_set, APIVersion::new("v1"), false));

        let got = conflicts_from_managers(&managers);
        let wanted = r#"conflicts with "Bob":
- .obj.template.obj.list[name="a"].id
- .obj.template.obj.list[name="a"].key"#;

        assert_eq!(got.error(), wanted, "Got:\n{}\nWanted:\n{}", got.error(), wanted);
    }
}

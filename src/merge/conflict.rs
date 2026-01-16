//! Conflict types for merge operations.

use crate::fieldpath::{ManagedFields, Path, Set};
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
        for (i, conflict) in self.conflicts.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", conflict)?;
        }
        Ok(())
    }
}

impl std::error::Error for Conflicts {}

/// Extracts conflicts from ManagedFields.
pub fn conflicts_from_managers(_managers: &ManagedFields) -> Conflicts {
    // TODO: Implement conflict detection
    Conflicts::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fieldpath::PathElement;

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
}

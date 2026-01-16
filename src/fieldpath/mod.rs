//! Field path module - Represents and manages field paths in nested structures.
//!
//! This module tracks which manager owns which fields.

mod path;
mod pathelementmap;
mod serialize;
mod set;

pub use path::*;
pub use pathelementmap::*;
pub use serialize::*;
pub use set::*;

use std::collections::HashMap;
use std::fmt;

/// APIVersion represents a version string for field ownership.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct APIVersion(String);

impl APIVersion {
    /// Creates a new APIVersion.
    pub fn new(version: impl Into<String>) -> Self {
        APIVersion(version.into())
    }

    /// Returns the version string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for APIVersion {
    fn from(s: &str) -> Self {
        APIVersion(s.to_string())
    }
}

impl From<String> for APIVersion {
    fn from(s: String) -> Self {
        APIVersion(s)
    }
}

impl std::fmt::Display for APIVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// VersionedSet associates a Set with an API version and "applied" flag.
#[derive(Debug, Clone)]
pub struct VersionedSet {
    pub set: Set,
    pub api_version: APIVersion,
    pub applied: bool,
}

impl VersionedSet {
    /// Creates a new VersionedSet.
    pub fn new(set: Set, api_version: APIVersion, applied: bool) -> Self {
        VersionedSet {
            set,
            api_version,
            applied,
        }
    }

    /// Returns a reference to the Set.
    pub fn set(&self) -> &Set {
        &self.set
    }

    /// Returns a mutable reference to the Set.
    pub fn set_mut(&mut self) -> &mut Set {
        &mut self.set
    }

    /// Returns the API version.
    pub fn api_version(&self) -> &APIVersion {
        &self.api_version
    }

    /// Returns true if this set was applied (vs update).
    pub fn applied(&self) -> bool {
        self.applied
    }
}

impl PartialEq for VersionedSet {
    fn eq(&self, other: &Self) -> bool {
        self.api_version == other.api_version
            && self.applied == other.applied
            && self.set == other.set
    }
}

impl Eq for VersionedSet {}

/// ManagedFields tracks what each manager owns.
#[derive(Debug, Clone, Default)]
pub struct ManagedFields {
    managers: HashMap<String, VersionedSet>,
}

impl ManagedFields {
    /// Creates a new empty ManagedFields.
    pub fn new() -> Self {
        ManagedFields {
            managers: HashMap::new(),
        }
    }

    /// Returns the number of managers.
    pub fn len(&self) -> usize {
        self.managers.len()
    }

    /// Returns true if there are no managers.
    pub fn is_empty(&self) -> bool {
        self.managers.is_empty()
    }

    /// Gets the VersionedSet for a manager.
    pub fn get(&self, manager: &str) -> Option<&VersionedSet> {
        self.managers.get(manager)
    }

    /// Gets a mutable reference to the VersionedSet for a manager.
    pub fn get_mut(&mut self, manager: &str) -> Option<&mut VersionedSet> {
        self.managers.get_mut(manager)
    }

    /// Inserts or updates a manager's VersionedSet.
    pub fn insert(&mut self, manager: impl Into<String>, vs: VersionedSet) {
        self.managers.insert(manager.into(), vs);
    }

    /// Removes a manager's VersionedSet.
    pub fn remove(&mut self, manager: &str) -> Option<VersionedSet> {
        self.managers.remove(manager)
    }

    /// Returns true if the manager exists.
    pub fn contains(&self, manager: &str) -> bool {
        self.managers.contains_key(manager)
    }

    /// Returns an iterator over managers and their VersionedSets.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &VersionedSet)> {
        self.managers.iter()
    }

    /// Returns an iterator over manager names.
    pub fn managers(&self) -> impl Iterator<Item = &String> {
        self.managers.keys()
    }

    /// Returns true if two ManagedFields are equal.
    pub fn equals(&self, other: &ManagedFields) -> bool {
        if self.managers.len() != other.managers.len() {
            return false;
        }

        for (manager, left) in &self.managers {
            match other.managers.get(manager) {
                None => return false,
                Some(right) => {
                    if left.api_version != right.api_version
                        || left.applied != right.applied
                        || !left.set.equals(&right.set)
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Creates a shallow copy of the ManagedFields.
    pub fn copy(&self) -> ManagedFields {
        self.clone()
    }

    /// Returns the symmetric difference between two ManagedFields.
    ///
    /// If a given manager's entry has version X in self and version Y in other,
    /// then the return value for that manager will be from other.
    /// If the difference for a manager is an empty set, that manager will not
    /// be included in the result.
    pub fn difference(&self, other: &ManagedFields) -> ManagedFields {
        let mut diff = ManagedFields::new();

        // Process managers in self
        for (manager, left) in &self.managers {
            match other.managers.get(manager) {
                None => {
                    // Manager only in self
                    if !left.set.empty() {
                        diff.managers.insert(manager.clone(), left.clone());
                    }
                }
                Some(right) => {
                    // Manager in both
                    if left.api_version != right.api_version {
                        // Different versions - keep right version
                        diff.managers.insert(manager.clone(), right.clone());
                    } else {
                        // Same version - compute symmetric difference
                        let new_set = left
                            .set
                            .difference(&right.set)
                            .union(&right.set.difference(&left.set));
                        if !new_set.empty() {
                            diff.managers.insert(
                                manager.clone(),
                                VersionedSet::new(new_set, right.api_version.clone(), false),
                            );
                        }
                    }
                }
            }
        }

        // Process managers only in other
        for (manager, vs) in &other.managers {
            if !self.managers.contains_key(manager) && !vs.set.empty() {
                diff.managers.insert(manager.clone(), vs.clone());
            }
        }

        diff
    }

    /// Removes all managers with empty sets.
    pub fn remove_empty(&mut self) {
        self.managers.retain(|_, vs| !vs.set.is_empty());
    }
}

impl PartialEq for ManagedFields {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl Eq for ManagedFields {}

impl fmt::Display for ManagedFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (manager, vs) in &self.managers {
            writeln!(f, "{}:", manager)?;
            writeln!(f, "- Applied: {}", vs.applied)?;
            writeln!(f, "- APIVersion: {}", vs.api_version)?;
            writeln!(f, "- Set: {:?}", vs.set)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_versioned_set() {
        let mut set = Set::new();
        set.insert(&Path::from_elements(vec![PathElement::field_name("name")]));

        let vs = VersionedSet::new(set.clone(), APIVersion::new("v1"), true);

        assert_eq!(vs.api_version(), &APIVersion::new("v1"));
        assert!(vs.applied());
        assert!(vs.set().has(&Path::from_elements(vec![PathElement::field_name("name")])));
    }

    #[test]
    fn test_managed_fields_basic() {
        let mut mf = ManagedFields::new();
        assert!(mf.is_empty());

        let mut set = Set::new();
        set.insert(&Path::from_elements(vec![PathElement::field_name("name")]));
        let vs = VersionedSet::new(set, APIVersion::new("v1"), true);

        mf.insert("manager1", vs);
        assert_eq!(mf.len(), 1);
        assert!(mf.contains("manager1"));
        assert!(!mf.contains("manager2"));
    }

    #[test]
    fn test_managed_fields_equals() {
        let mut set1 = Set::new();
        set1.insert(&Path::from_elements(vec![PathElement::field_name("name")]));

        let mut mf1 = ManagedFields::new();
        mf1.insert("manager1", VersionedSet::new(set1.clone(), APIVersion::new("v1"), true));

        let mut mf2 = ManagedFields::new();
        mf2.insert("manager1", VersionedSet::new(set1.clone(), APIVersion::new("v1"), true));

        assert!(mf1.equals(&mf2));
        assert_eq!(mf1, mf2);

        // Different applied flag
        let mut mf3 = ManagedFields::new();
        mf3.insert("manager1", VersionedSet::new(set1.clone(), APIVersion::new("v1"), false));
        assert!(!mf1.equals(&mf3));
    }

    #[test]
    fn test_managed_fields_difference() {
        let mut set1 = Set::new();
        set1.insert(&Path::from_elements(vec![PathElement::field_name("a")]));
        set1.insert(&Path::from_elements(vec![PathElement::field_name("b")]));

        let mut set2 = Set::new();
        set2.insert(&Path::from_elements(vec![PathElement::field_name("b")]));
        set2.insert(&Path::from_elements(vec![PathElement::field_name("c")]));

        let mut mf1 = ManagedFields::new();
        mf1.insert("manager1", VersionedSet::new(set1, APIVersion::new("v1"), true));

        let mut mf2 = ManagedFields::new();
        mf2.insert("manager1", VersionedSet::new(set2, APIVersion::new("v1"), true));

        let diff = mf1.difference(&mf2);
        assert!(diff.contains("manager1"));

        let diff_set = &diff.get("manager1").unwrap().set;
        // Symmetric difference should contain "a" and "c"
        assert!(diff_set.has(&Path::from_elements(vec![PathElement::field_name("a")])));
        assert!(diff_set.has(&Path::from_elements(vec![PathElement::field_name("c")])));
        assert!(!diff_set.has(&Path::from_elements(vec![PathElement::field_name("b")])));
    }
}

//! Set types for field path tracking.

use super::path::{Path, PathElement};
use std::collections::BTreeMap;

/// PathElementSet is a sorted set of PathElements for efficient membership testing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PathElementSet {
    members: Vec<PathElement>,
}

impl PathElementSet {
    /// Creates a new empty set.
    pub fn new() -> Self {
        PathElementSet {
            members: Vec::new(),
        }
    }

    /// Creates a set from a vector of elements (will be sorted).
    pub fn from_vec(mut elements: Vec<PathElement>) -> Self {
        elements.sort();
        elements.dedup();
        PathElementSet { members: elements }
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Returns true if the set contains the given element.
    pub fn contains(&self, element: &PathElement) -> bool {
        self.members.binary_search(element).is_ok()
    }

    /// Inserts an element into the set.
    pub fn insert(&mut self, element: PathElement) {
        match self.members.binary_search(&element) {
            Ok(_) => {} // Already present
            Err(pos) => self.members.insert(pos, element),
        }
    }

    /// Removes an element from the set.
    pub fn remove(&mut self, element: &PathElement) -> bool {
        match self.members.binary_search(element) {
            Ok(pos) => {
                self.members.remove(pos);
                true
            }
            Err(_) => false,
        }
    }

    /// Returns an iterator over the elements.
    pub fn iter(&self) -> impl Iterator<Item = &PathElement> {
        self.members.iter()
    }

    /// Returns the union of two sets.
    pub fn union(&self, other: &PathElementSet) -> PathElementSet {
        let mut result = Vec::with_capacity(self.len() + other.len());
        let mut i = 0;
        let mut j = 0;

        while i < self.members.len() && j < other.members.len() {
            match self.members[i].cmp(&other.members[j]) {
                std::cmp::Ordering::Less => {
                    result.push(self.members[i].clone());
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    result.push(other.members[j].clone());
                    j += 1;
                }
                std::cmp::Ordering::Equal => {
                    result.push(self.members[i].clone());
                    i += 1;
                    j += 1;
                }
            }
        }

        result.extend(self.members[i..].iter().cloned());
        result.extend(other.members[j..].iter().cloned());

        PathElementSet { members: result }
    }

    /// Returns the intersection of two sets.
    pub fn intersection(&self, other: &PathElementSet) -> PathElementSet {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < self.members.len() && j < other.members.len() {
            match self.members[i].cmp(&other.members[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    result.push(self.members[i].clone());
                    i += 1;
                    j += 1;
                }
            }
        }

        PathElementSet { members: result }
    }

    /// Returns the difference of two sets (self - other).
    pub fn difference(&self, other: &PathElementSet) -> PathElementSet {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < self.members.len() && j < other.members.len() {
            match self.members[i].cmp(&other.members[j]) {
                std::cmp::Ordering::Less => {
                    result.push(self.members[i].clone());
                    i += 1;
                }
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    i += 1;
                    j += 1;
                }
            }
        }

        result.extend(self.members[i..].iter().cloned());

        PathElementSet { members: result }
    }
}

/// SetNodeMap maps PathElements to child Sets.
pub type SetNodeMap = BTreeMap<PathElement, Set>;

/// Set is a tree structure for tracking field ownership.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Set {
    /// Direct children at this level.
    pub members: PathElementSet,
    /// Nested children for deeper paths.
    pub children: SetNodeMap,
    /// True if the empty path (root itself) is in this set.
    root_in_set: bool,
}

impl Set {
    /// Creates a new empty set.
    pub fn new() -> Self {
        Set {
            members: PathElementSet::new(),
            children: BTreeMap::new(),
            root_in_set: false,
        }
    }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool {
        !self.root_in_set && self.members.is_empty() && self.children.is_empty()
    }

    /// Returns true if the set is empty (alias for Go compatibility).
    pub fn empty(&self) -> bool {
        self.is_empty()
    }

    /// Returns the number of top-level members.
    pub fn size(&self) -> usize {
        self.members.len() + self.children.len()
    }

    /// Returns true if this set equals another set.
    pub fn equals(&self, other: &Set) -> bool {
        self == other
    }

    /// Returns true if the set contains the given path.
    pub fn has(&self, path: &Path) -> bool {
        if path.is_empty() {
            return self.root_in_set;
        }

        let elements = path.as_slice();
        self.has_path_elements(elements)
    }

    fn has_path_elements(&self, elements: &[PathElement]) -> bool {
        if elements.is_empty() {
            return true;
        }

        let first = &elements[0];
        let rest = &elements[1..];

        if rest.is_empty() {
            return self.members.contains(first);
        }

        if let Some(child) = self.children.get(first) {
            return child.has_path_elements(rest);
        }

        false
    }

    /// Inserts a path into the set.
    pub fn insert(&mut self, path: &Path) {
        if path.is_empty() {
            self.root_in_set = true;
            return;
        }

        let elements = path.as_slice();
        self.insert_path_elements(elements);
    }

    fn insert_path_elements(&mut self, elements: &[PathElement]) {
        if elements.is_empty() {
            return;
        }

        let first = &elements[0];
        let rest = &elements[1..];

        if rest.is_empty() {
            self.members.insert(first.clone());
            return;
        }

        let child = self.children.entry(first.clone()).or_insert_with(Set::new);
        child.insert_path_elements(rest);
    }

    /// Returns the union of two sets.
    pub fn union(&self, other: &Set) -> Set {
        let mut result = self.clone();
        result.union_into(other);
        result
    }

    fn union_into(&mut self, other: &Set) {
        self.root_in_set = self.root_in_set || other.root_in_set;
        self.members = self.members.union(&other.members);

        for (key, other_child) in &other.children {
            if let Some(self_child) = self.children.get_mut(key) {
                self_child.union_into(other_child);
            } else {
                self.children.insert(key.clone(), other_child.clone());
            }
        }
    }

    /// Returns the intersection of two sets.
    pub fn intersection(&self, other: &Set) -> Set {
        let root_in_set = self.root_in_set && other.root_in_set;
        let members = self.members.intersection(&other.members);

        let mut children = BTreeMap::new();
        for (key, self_child) in &self.children {
            if let Some(other_child) = other.children.get(key) {
                let child = self_child.intersection(other_child);
                if !child.is_empty() {
                    children.insert(key.clone(), child);
                }
            }
        }

        Set { members, children, root_in_set }
    }

    /// Returns the difference of two sets (self - other).
    pub fn difference(&self, other: &Set) -> Set {
        let root_in_set = self.root_in_set && !other.root_in_set;
        let members = self.members.difference(&other.members);

        let mut children = BTreeMap::new();
        for (key, self_child) in &self.children {
            if let Some(other_child) = other.children.get(key) {
                let child = self_child.difference(other_child);
                if !child.is_empty() {
                    children.insert(key.clone(), child);
                }
            } else {
                children.insert(key.clone(), self_child.clone());
            }
        }

        Set { members, children, root_in_set }
    }

    /// Iterates over all paths in the set.
    pub fn iterate<F>(&self, mut f: F)
    where
        F: FnMut(&Path),
    {
        self.iterate_with_path(&mut Path::new(), &mut f);
    }

    fn iterate_with_path<F>(&self, current_path: &mut Path, f: &mut F)
    where
        F: FnMut(&Path),
    {
        // Visit root if it's in the set
        if self.root_in_set && current_path.is_empty() {
            f(current_path);
        }

        // Visit members
        for member in self.members.iter() {
            current_path.push(member.clone());
            f(current_path);
            current_path.pop();
        }

        // Visit children
        for (key, child) in &self.children {
            current_path.push(key.clone());
            child.iterate_with_path(current_path, f);
            current_path.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_element_set_operations() {
        let mut set1 = PathElementSet::new();
        set1.insert(PathElement::field_name("a"));
        set1.insert(PathElement::field_name("b"));

        let mut set2 = PathElementSet::new();
        set2.insert(PathElement::field_name("b"));
        set2.insert(PathElement::field_name("c"));

        let union = set1.union(&set2);
        assert_eq!(union.len(), 3);
        assert!(union.contains(&PathElement::field_name("a")));
        assert!(union.contains(&PathElement::field_name("b")));
        assert!(union.contains(&PathElement::field_name("c")));

        let intersection = set1.intersection(&set2);
        assert_eq!(intersection.len(), 1);
        assert!(intersection.contains(&PathElement::field_name("b")));

        let difference = set1.difference(&set2);
        assert_eq!(difference.len(), 1);
        assert!(difference.contains(&PathElement::field_name("a")));
    }

    #[test]
    fn test_set_insert_and_has() {
        let mut set = Set::new();
        assert!(set.is_empty());

        let path = Path::from_elements(vec![
            PathElement::field_name("metadata"),
            PathElement::field_name("name"),
        ]);

        set.insert(&path);
        assert!(set.has(&path));

        let partial_path = Path::from_elements(vec![PathElement::field_name("metadata")]);
        assert!(!set.has(&partial_path));
    }

    #[test]
    fn test_set_union() {
        let mut set1 = Set::new();
        set1.insert(&Path::from_elements(vec![
            PathElement::field_name("a"),
            PathElement::field_name("x"),
        ]));

        let mut set2 = Set::new();
        set2.insert(&Path::from_elements(vec![
            PathElement::field_name("a"),
            PathElement::field_name("y"),
        ]));

        let union = set1.union(&set2);
        assert!(union.has(&Path::from_elements(vec![
            PathElement::field_name("a"),
            PathElement::field_name("x"),
        ])));
        assert!(union.has(&Path::from_elements(vec![
            PathElement::field_name("a"),
            PathElement::field_name("y"),
        ])));
    }

    #[test]
    fn test_set_iterate() {
        let mut set = Set::new();
        set.insert(&Path::from_elements(vec![PathElement::field_name("a")]));
        set.insert(&Path::from_elements(vec![
            PathElement::field_name("b"),
            PathElement::field_name("c"),
        ]));

        let mut paths = Vec::new();
        set.iterate(|path| {
            paths.push(path.clone());
        });

        assert_eq!(paths.len(), 2);
    }
}

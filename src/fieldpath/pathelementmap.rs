//! PathElementMap types for mapping PathElements to values.

use super::path::PathElement;
use crate::value::Value;

/// PathElementValue is a tuple of PathElement and a generic value.
#[derive(Debug, Clone)]
struct PathElementValue<T> {
    path_element: PathElement,
    value: T,
}

/// PathElementMap is a sorted map from PathElement to a generic value T.
#[derive(Debug, Clone, Default)]
pub struct PathElementMap<T> {
    members: Vec<PathElementValue<T>>,
}

impl<T: Clone> PathElementMap<T> {
    /// Creates a new empty map with the given initial capacity.
    pub fn new(capacity: usize) -> Self {
        PathElementMap {
            members: Vec::with_capacity(capacity),
        }
    }

    /// Returns the number of entries in the map.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Returns true if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Inserts a PathElement and associated value into the map.
    /// If the PathElement already exists, the value is replaced.
    pub fn insert(&mut self, pe: PathElement, value: T) {
        let loc = self.members.iter().position(|m| m.path_element >= pe);

        match loc {
            None => {
                self.members.push(PathElementValue {
                    path_element: pe,
                    value,
                });
            }
            Some(idx) => {
                if self.members[idx].path_element == pe {
                    self.members[idx].value = value;
                } else {
                    self.members.insert(idx, PathElementValue {
                        path_element: pe,
                        value,
                    });
                }
            }
        }
    }

    /// Gets the value associated with the given PathElement.
    /// Returns None if the PathElement is not in the map.
    pub fn get(&self, pe: &PathElement) -> Option<&T> {
        self.members
            .binary_search_by(|m| m.path_element.cmp(pe))
            .ok()
            .map(|idx| &self.members[idx].value)
    }

    /// Gets a mutable reference to the value associated with the given PathElement.
    pub fn get_mut(&mut self, pe: &PathElement) -> Option<&mut T> {
        self.members
            .binary_search_by(|m| m.path_element.cmp(pe))
            .ok()
            .map(|idx| &mut self.members[idx].value)
    }

    /// Returns true if the map contains the given PathElement.
    pub fn contains(&self, pe: &PathElement) -> bool {
        self.members
            .binary_search_by(|m| m.path_element.cmp(pe))
            .is_ok()
    }

    /// Removes the entry with the given PathElement.
    /// Returns the value if it was present.
    pub fn remove(&mut self, pe: &PathElement) -> Option<T> {
        match self.members.binary_search_by(|m| m.path_element.cmp(pe)) {
            Ok(idx) => Some(self.members.remove(idx).value),
            Err(_) => None,
        }
    }

    /// Returns an iterator over the entries in the map.
    pub fn iter(&self) -> impl Iterator<Item = (&PathElement, &T)> {
        self.members.iter().map(|m| (&m.path_element, &m.value))
    }

    /// Returns an iterator over the keys (PathElements) in the map.
    pub fn keys(&self) -> impl Iterator<Item = &PathElement> {
        self.members.iter().map(|m| &m.path_element)
    }

    /// Returns an iterator over the values in the map.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.members.iter().map(|m| &m.value)
    }
}

/// PathElementValueMap is a specialized map from PathElement to Value.
pub type PathElementValueMap = PathElementMap<Value>;

impl PathElementValueMap {
    /// Creates a new PathElementValueMap with the given capacity.
    pub fn make(capacity: usize) -> Self {
        PathElementMap::new(capacity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_element_map_insert_and_get() {
        let mut map: PathElementMap<i32> = PathElementMap::new(10);

        map.insert(PathElement::field_name("a"), 1);
        map.insert(PathElement::field_name("b"), 2);
        map.insert(PathElement::field_name("c"), 3);

        assert_eq!(map.get(&PathElement::field_name("a")), Some(&1));
        assert_eq!(map.get(&PathElement::field_name("b")), Some(&2));
        assert_eq!(map.get(&PathElement::field_name("c")), Some(&3));
        assert_eq!(map.get(&PathElement::field_name("d")), None);
    }

    #[test]
    fn test_path_element_map_replace() {
        let mut map: PathElementMap<i32> = PathElementMap::new(10);

        map.insert(PathElement::field_name("a"), 1);
        assert_eq!(map.get(&PathElement::field_name("a")), Some(&1));

        map.insert(PathElement::field_name("a"), 100);
        assert_eq!(map.get(&PathElement::field_name("a")), Some(&100));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_path_element_map_remove() {
        let mut map: PathElementMap<i32> = PathElementMap::new(10);

        map.insert(PathElement::field_name("a"), 1);
        map.insert(PathElement::field_name("b"), 2);

        assert_eq!(map.remove(&PathElement::field_name("a")), Some(1));
        assert_eq!(map.len(), 1);
        assert!(!map.contains(&PathElement::field_name("a")));
    }

    #[test]
    fn test_path_element_value_map() {
        let mut map = PathElementValueMap::make(10);

        map.insert(PathElement::field_name("name"), Value::String("test".into()));
        map.insert(PathElement::field_name("count"), Value::Int(42));

        assert_eq!(
            map.get(&PathElement::field_name("name")),
            Some(&Value::String("test".into()))
        );
        assert_eq!(
            map.get(&PathElement::field_name("count")),
            Some(&Value::Int(42))
        );
    }
}

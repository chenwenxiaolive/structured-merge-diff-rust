//! Path element and path types.

use crate::value::{FieldList, Value};
use std::cmp::Ordering;

/// PathElement represents one level of path navigation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathElement {
    /// Field name for map/struct fields.
    FieldName(String),
    /// Key for associative lists (multi-field keys).
    Key(FieldList),
    /// Value for sets (scalar list elements).
    Value(Value),
    /// Index for array indices.
    Index(i32),
}

impl PathElement {
    /// Creates a new field name path element.
    pub fn field_name(name: impl Into<String>) -> Self {
        PathElement::FieldName(name.into())
    }

    /// Creates a new key path element.
    pub fn key(fields: FieldList) -> Self {
        PathElement::Key(fields)
    }

    /// Creates a new value path element.
    pub fn value(v: Value) -> Self {
        PathElement::Value(v)
    }

    /// Creates a new index path element.
    pub fn index(i: i32) -> Self {
        PathElement::Index(i)
    }

    /// Returns true if this is a field name element.
    pub fn is_field_name(&self) -> bool {
        matches!(self, PathElement::FieldName(_))
    }

    /// Returns the field name if this is a field name element.
    pub fn as_field_name(&self) -> Option<&str> {
        match self {
            PathElement::FieldName(name) => Some(name),
            _ => None,
        }
    }
}

impl PartialOrd for PathElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathElement {
    fn cmp(&self, other: &Self) -> Ordering {
        fn type_order(pe: &PathElement) -> u8 {
            match pe {
                PathElement::FieldName(_) => 0,
                PathElement::Key(_) => 1,
                PathElement::Value(_) => 2,
                PathElement::Index(_) => 3,
            }
        }

        let type_cmp = type_order(self).cmp(&type_order(other));
        if type_cmp != Ordering::Equal {
            return type_cmp;
        }

        match (self, other) {
            (PathElement::FieldName(a), PathElement::FieldName(b)) => a.cmp(b),
            (PathElement::Key(a), PathElement::Key(b)) => {
                // Compare field lists by comparing each field
                for (fa, fb) in a.fields.iter().zip(b.fields.iter()) {
                    let name_cmp = fa.name.cmp(&fb.name);
                    if name_cmp != Ordering::Equal {
                        return name_cmp;
                    }
                    let val_cmp = fa.value.cmp(&fb.value);
                    if val_cmp != Ordering::Equal {
                        return val_cmp;
                    }
                }
                a.fields.len().cmp(&b.fields.len())
            }
            (PathElement::Value(a), PathElement::Value(b)) => a.cmp(b),
            (PathElement::Index(a), PathElement::Index(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }
}

/// Path represents a complete path to a nested field.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Path {
    elements: Vec<PathElement>,
}

impl Path {
    /// Creates a new empty path.
    pub fn new() -> Self {
        Path {
            elements: Vec::new(),
        }
    }

    /// Creates a path from a vector of elements.
    pub fn from_elements(elements: Vec<PathElement>) -> Self {
        Path { elements }
    }

    /// Returns the number of elements in the path.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Returns true if the path is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns an iterator over the path elements.
    pub fn iter(&self) -> impl Iterator<Item = &PathElement> {
        self.elements.iter()
    }

    /// Appends a path element.
    pub fn push(&mut self, element: PathElement) {
        self.elements.push(element);
    }

    /// Removes and returns the last path element.
    pub fn pop(&mut self) -> Option<PathElement> {
        self.elements.pop()
    }

    /// Returns the last path element.
    pub fn last(&self) -> Option<&PathElement> {
        self.elements.last()
    }

    /// Creates a new path with the given element appended.
    pub fn with(&self, element: PathElement) -> Self {
        let mut new_path = self.clone();
        new_path.push(element);
        new_path
    }

    /// Returns a slice of the path elements.
    pub fn as_slice(&self) -> &[PathElement] {
        &self.elements
    }
}

impl FromIterator<PathElement> for Path {
    fn from_iter<T: IntoIterator<Item = PathElement>>(iter: T) -> Self {
        Path {
            elements: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for Path {
    type Item = PathElement;
    type IntoIter = std::vec::IntoIter<PathElement>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = &'a PathElement;
    type IntoIter = std::slice::Iter<'a, PathElement>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

impl std::fmt::Display for PathElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathElement::FieldName(name) => write!(f, ".{}", name),
            PathElement::Key(fields) => {
                write!(f, "[")?;
                for (i, field) in fields.fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}={:?}", field.name, field.value)?;
                }
                write!(f, "]")
            }
            PathElement::Value(v) => write!(f, "[={:?}]", v),
            PathElement::Index(i) => write!(f, "[{}]", i),
        }
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for element in &self.elements {
            write!(f, "{}", element)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_element_field_name() {
        let pe = PathElement::field_name("foo");
        assert!(pe.is_field_name());
        assert_eq!(pe.as_field_name(), Some("foo"));
    }

    #[test]
    fn test_path_operations() {
        let mut path = Path::new();
        assert!(path.is_empty());

        path.push(PathElement::field_name("metadata"));
        path.push(PathElement::field_name("name"));
        assert_eq!(path.len(), 2);

        assert_eq!(
            path.last(),
            Some(&PathElement::FieldName("name".to_string()))
        );

        let popped = path.pop();
        assert_eq!(popped, Some(PathElement::FieldName("name".to_string())));
        assert_eq!(path.len(), 1);
    }

    #[test]
    fn test_path_display() {
        let path = Path::from_elements(vec![
            PathElement::field_name("metadata"),
            PathElement::field_name("name"),
        ]);
        assert_eq!(format!("{}", path), ".metadata.name");
    }

    #[test]
    fn test_path_element_ordering() {
        let a = PathElement::field_name("a");
        let b = PathElement::field_name("b");
        assert!(a < b);

        let idx = PathElement::index(0);
        // Field names come before indices
        assert!(a < idx);
    }
}

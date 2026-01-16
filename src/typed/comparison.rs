//! Comparison result types.

use crate::fieldpath::Set;
use std::fmt;

/// Comparison holds the result of comparing two TypedValues.
///
/// No field will appear in more than one of the three fieldsets.
/// If all of the fieldsets are empty, then the objects must have been equal.
#[derive(Debug, Clone, Default)]
pub struct Comparison {
    /// Fields that were in the left-hand side but not the right-hand side.
    pub removed: Set,
    /// Fields that were in both but had different values.
    pub modified: Set,
    /// Fields that were in the right-hand side but not the left-hand side.
    pub added: Set,
}

impl Comparison {
    /// Creates a new empty Comparison.
    pub fn new() -> Self {
        Comparison {
            removed: Set::new(),
            modified: Set::new(),
            added: Set::new(),
        }
    }

    /// Returns true if there are no changes.
    pub fn is_same(&self) -> bool {
        self.removed.is_empty() && self.modified.is_empty() && self.added.is_empty()
    }

    /// Excludes the given fields from the comparison result.
    pub fn exclude_fields(&mut self, fields: &Set) {
        self.removed = self.removed.difference(fields);
        self.modified = self.modified.difference(fields);
        self.added = self.added.difference(fields);
    }

    /// Filters the comparison to only include the given fields.
    pub fn filter_fields(&mut self, fields: &Set) {
        self.removed = self.removed.intersection(fields);
        self.modified = self.modified.intersection(fields);
        self.added = self.added.intersection(fields);
    }

    /// Returns true if any fields were removed.
    pub fn has_removed(&self) -> bool {
        !self.removed.is_empty()
    }

    /// Returns true if any fields were modified.
    pub fn has_modified(&self) -> bool {
        !self.modified.is_empty()
    }

    /// Returns true if any fields were added.
    pub fn has_added(&self) -> bool {
        !self.added.is_empty()
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;

        if !self.modified.is_empty() {
            if !first {
                writeln!(f)?;
            }
            first = false;
            write!(f, "- Modified Fields:")?;
            self.modified.iterate(|path| {
                let _ = write!(f, "\n  {}", path);
            });
        }

        if !self.added.is_empty() {
            if !first {
                writeln!(f)?;
            }
            first = false;
            write!(f, "- Added Fields:")?;
            self.added.iterate(|path| {
                let _ = write!(f, "\n  {}", path);
            });
        }

        if !self.removed.is_empty() {
            if !first {
                writeln!(f)?;
            }
            write!(f, "- Removed Fields:")?;
            self.removed.iterate(|path| {
                let _ = write!(f, "\n  {}", path);
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fieldpath::{Path, PathElement};

    #[test]
    fn test_comparison_is_same() {
        let comp = Comparison::new();
        assert!(comp.is_same());
    }

    #[test]
    fn test_comparison_has_changes() {
        let mut comp = Comparison::new();
        comp.added.insert(&Path::from_elements(vec![PathElement::field_name("new_field")]));

        assert!(!comp.is_same());
        assert!(comp.has_added());
        assert!(!comp.has_removed());
        assert!(!comp.has_modified());
    }

    #[test]
    fn test_comparison_exclude_fields() {
        let mut comp = Comparison::new();
        comp.added.insert(&Path::from_elements(vec![PathElement::field_name("a")]));
        comp.added.insert(&Path::from_elements(vec![PathElement::field_name("b")]));

        let mut exclude = Set::new();
        exclude.insert(&Path::from_elements(vec![PathElement::field_name("a")]));

        comp.exclude_fields(&exclude);

        assert!(!comp.added.has(&Path::from_elements(vec![PathElement::field_name("a")])));
        assert!(comp.added.has(&Path::from_elements(vec![PathElement::field_name("b")])));
    }

    #[test]
    fn test_comparison_display() {
        let mut comp = Comparison::new();
        comp.added.insert(&Path::from_elements(vec![PathElement::field_name("new")]));
        comp.modified.insert(&Path::from_elements(vec![PathElement::field_name("changed")]));

        let display = format!("{}", comp);
        assert!(display.contains("Modified Fields"));
        assert!(display.contains("Added Fields"));
    }
}

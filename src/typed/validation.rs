//! Validation types and errors.

use std::fmt;
use thiserror::Error;

/// ValidationOptions controls validation behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationOption {
    /// Allow duplicate items in sets and associative lists.
    AllowDuplicates,
}

/// ValidationError represents an error during schema validation.
#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    #[error("{path}: type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    #[error("{path}: unknown field: {field}")]
    UnknownField { path: String, field: String },

    #[error("{path}: missing required field: {field}")]
    MissingField { path: String, field: String },

    #[error("{path}: duplicate key in list: {key}")]
    DuplicateKey { path: String, key: String },

    #[error("{path}: {message}")]
    InvalidValue { path: String, message: String },

    #[error("{message}")]
    SchemaError { message: String },
}

impl ValidationError {
    /// Creates a type mismatch error.
    pub fn type_mismatch(path: impl Into<String>, expected: impl Into<String>, actual: impl Into<String>) -> Self {
        ValidationError::TypeMismatch {
            path: path.into(),
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Creates an unknown field error.
    pub fn unknown_field(path: impl Into<String>, field: impl Into<String>) -> Self {
        ValidationError::UnknownField {
            path: path.into(),
            field: field.into(),
        }
    }

    /// Creates a missing field error.
    pub fn missing_field(path: impl Into<String>, field: impl Into<String>) -> Self {
        ValidationError::MissingField {
            path: path.into(),
            field: field.into(),
        }
    }

    /// Creates a duplicate key error.
    pub fn duplicate_key(path: impl Into<String>, key: impl Into<String>) -> Self {
        ValidationError::DuplicateKey {
            path: path.into(),
            key: key.into(),
        }
    }

    /// Creates an invalid value error.
    pub fn invalid_value(path: impl Into<String>, message: impl Into<String>) -> Self {
        ValidationError::InvalidValue {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Creates a schema error.
    pub fn schema_error(message: impl Into<String>) -> Self {
        ValidationError::SchemaError {
            message: message.into(),
        }
    }
}

/// ValidationErrors is a collection of validation errors.
#[derive(Debug, Clone, Default)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

impl ValidationErrors {
    /// Creates a new empty ValidationErrors.
    pub fn new() -> Self {
        ValidationErrors { errors: Vec::new() }
    }

    /// Creates ValidationErrors from a single error.
    pub fn from_error(error: ValidationError) -> Self {
        ValidationErrors {
            errors: vec![error],
        }
    }

    /// Adds an error.
    pub fn add(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Extends with another ValidationErrors.
    pub fn extend(&mut self, other: ValidationErrors) {
        self.errors.extend(other.errors);
    }

    /// Returns true if there are no errors.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns the number of errors.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Returns an iterator over the errors.
    pub fn iter(&self) -> impl Iterator<Item = &ValidationError> {
        self.errors.iter()
    }
}

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;
    type IntoIter = std::vec::IntoIter<ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, err) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", err)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::type_mismatch(".metadata.name", "string", "int");
        assert!(format!("{}", err).contains("type mismatch"));
    }

    #[test]
    fn test_validation_errors_collection() {
        let mut errs = ValidationErrors::new();
        assert!(errs.is_empty());

        errs.add(ValidationError::unknown_field("", "foo"));
        assert_eq!(errs.len(), 1);
        assert!(!errs.is_empty());
    }
}

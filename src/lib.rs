//! # Structured Merge Diff
//!
//! A Rust implementation of structured merge and diff operations for Kubernetes.
//!
//! This library provides structured merge and diff operations on typed YAML/JSON objects.
//! It enables multi-manager field ownership tracking and conflict detection while
//! performing merge operations.
//!
//! ## Modules
//!
//! - [`schema`] - Type schema definition language for structured merge operations
//! - [`value`] - In-memory representation of YAML/JSON objects with type-aware operations
//! - [`fieldpath`] - Field path representation and management for tracking field ownership
//! - [`typed`] - Operations on Values with specific schemas (validation, comparison, merging)
//! - [`merge`] - High-level multi-manager merge and apply operations

pub mod fieldpath;
pub mod merge;
pub mod schema;
pub mod typed;
pub mod value;

pub use fieldpath::{
    APIVersion, ManagedFields, Path, PathElement, PathElementMap, PathElementValueMap,
    Set as FieldPathSet, VersionedSet,
};
pub use merge::{Conflict, Conflicts, Updater, UpdaterBuilder};
pub use schema::Schema;
pub use typed::{Comparison, TypedValue};
pub use value::Value;

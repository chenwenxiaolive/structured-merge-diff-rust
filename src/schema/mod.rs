//! Schema module defines a targeted schema language for structured merges and diffs.
//!
//! This schema was derived by observing the API objects used by Kubernetes, and
//! formalizing a model which allows certain operations ("apply") to be more
//! well defined.

mod elements;
mod equals;
mod schemaschema;

pub use elements::*;
pub use schemaschema::SCHEMA_SCHEMA_YAML;

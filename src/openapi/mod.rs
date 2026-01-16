//! OpenAPI module - Parse OpenAPI v2/v3 documents and convert to SMD schema.
//!
//! This module provides functionality to parse OpenAPI (Swagger) v2 and OpenAPI v3
//! documents and convert them to the structured-merge-diff schema format.

mod schema;
mod converter;

pub use schema::*;
pub use converter::*;

//! Typed module - Operations on Values with specific schemas.
//!
//! This module provides validation, comparison, and merging operations.

mod comparison;
mod typed_value;
mod validation;

pub use comparison::*;
pub use typed_value::*;
pub use validation::*;

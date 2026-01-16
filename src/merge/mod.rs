//! Merge module - High-level multi-manager merge and apply operations.
//!
//! This module provides tracking of field ownership across multiple managers.

mod updater;
mod conflict;

pub use updater::*;
pub use conflict::*;

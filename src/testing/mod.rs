//! Provides a set of testing functions and macros to reduce testing boiler plate
//!
//! ## For testing only
//! All code in this module should only ever be used in testing and not in production.
//!
//! ### How to use the Rivia `testing` module
//! ```
//! use rivia::prelude::*;
//! ```
#[macro_use]
mod assert;

pub use assert::*;

/// Defines the `tests/temp` location in the current project for file based testing if required
pub const TEST_TEMP_DIR: &str = "tests/temp";

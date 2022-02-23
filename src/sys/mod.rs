//! Provides a unified, simplified systems api
//!
//! ### How to use the Rivia `sys` module
//! ```
//! use rivia::prelude::*;
//! ```
mod fs;

// Export contents of modules into sys
pub use fs::*;

// Export directly
pub mod user;

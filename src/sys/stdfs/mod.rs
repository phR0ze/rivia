//! Provides a Vfs backend implementation that wraps the standard library `std::fs` functions for
//! use with Vfs.
mod entry;
pub mod fs;

pub use entry::*;
pub use fs::*;

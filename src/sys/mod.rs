//! Provides multiple filesystem implementations in a unified, extended and simplified way
//!
//! ### How to use the Rivia `sys` module
//! ```
//! use rivia::prelude::*;
//! ```
mod entries;
mod entry;
mod fs;
mod memfs;
mod path;
mod stdfs;

pub use entries::*;
pub use entry::*;
pub use fs::*;
pub use memfs::*;
pub use path::*;
pub use stdfs::*;

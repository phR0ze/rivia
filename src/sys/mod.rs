//! Provides multiple filesystem implementations in a unified, extended and simplified way with
//! including pathing, io and file management helpers.
//!
//! ### How to use the `sys` module from the Rivia crate
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

//! `fs` provides multiple filesystem implementations in a unified, extended and simplified way to
//! work with the filesystems including pathing, io and file management.
//!
//! ### How to use fs module from the Rivia Core crate
//! ```
//! use rivia::prelude::*;
//! ```
mod entry;
mod fs;
mod memfs;
mod stdfs;

pub use entry::*;
pub use fs::*;
pub use memfs::*;
pub use stdfs::*;

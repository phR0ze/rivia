//! `fs` provides multiple filesystem implementations in a unified, extended and simplified way to
//! work with the filesystems including pathing, io and file management.
//!
//! ### How to use fs module from the Rivia Core crate
//! ```
//! use rivia::prelude::*;
//! ```
mod entry;
mod memfs;
mod stdfs;
mod fs;

pub use entry::*;
pub use memfs::*;
pub use stdfs::*;
pub use fs::*;
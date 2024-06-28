//! Provides a unified, simplified systems api including essential macros and extensions to fill in
//! gaps in Rust ergonomics to reduce the amount of boiler plate code required for common tasks.
//! The intent is to provide this while keeping dependencies to a minimum.
//!
//! ## Virtual FileSystem (VFS)
//!
//! ### Switching out VFS backends
//! Using the power of Rust enums we can create a simple virtual file system with multiple backend
//! implementations that can easily be switched out at compile time for testing purposes with
//! almost zero additional overhead by simply passing in a vfs reference to functions needing to
//! manipulate the filesystem. For those wishing for a truely seamless experience see the
//! `rivia-vfs` crate for a global singleton that can be dynamically updated at runtime thus
//! avoiding passing a vfs reference around.
//!
//! ```
//! use rivia::prelude::*;
//!
//! fn main() {
//!     // Simply replace this line with `let vfs = Vfs::stdfs();` for the real filesystem
//!     let vfs = Vfs::memfs();
//!     let config = load_config(vfs);
//!     assert_eq!(config, "this is a test");
//!     println!("VFS test passed");
//! }
//!
//! // Load an example application configuration file using VFS.
//! // This allows you to test with a memory backed VFS implementation during testing and with
//! // the real filesystem during production.
//! fn load_config(vfs: Vfs) -> String {
//!     let dir = PathBuf::from("/etc/xdg");
//!     vfs.mkdir_p(&dir).unwrap();
//!     let filepath = dir.mash("rivia.toml");
//!     vfs.write_all(&filepath, "this is a test").unwrap();
//!     assert_eq!(vfs.config_dir("rivia.toml").unwrap().to_str().unwrap(), "/etc/xdg");
//!
//!     if let Some(config_dir) = vfs.config_dir("rivia.toml") {
//!         let path = config_dir.mash("rivia.toml");
//!         return vfs.read_all(&path).unwrap();
//!     }
//!     "".into()
//! }
//! ```
//!
//! ## Rejected Considerations
//!
//! ### VfsPath
//! `VfsPath` was considered as a way to track and not repeat environment variable and absolute path
//! resolution. Additionally it would be used in the VFS case to track the VFS backend being used to
//! allow for extensions to a Path to chain call additional operations. However the relative amount
//! of overhead of resolving paths is small when compared to disk IO which will usually be being
//! done. What's more the benefit of trackig the VFS for chain calling is dubious when you introduce
//! the complexity of multiple ways to get access to VFS operations. Considering the ergonomic
//! impact and commplexity of such a change. I won't be implementing a notion of a `VfsPath` in
//! favor of a single point of entry into the VFS operations and much cleaner ergonomics i.e. always
//! use the Filesystem backend trait implementation via Vfs for every Filesystem related operation.
//!
//! ### Using Rivia
//! ```
//! use rivia::prelude::*;
//! ```
#[macro_use]
pub mod testing;
#[macro_use]
pub mod core;

pub mod errors;
pub mod sys;

/// All essential symbols in a simple consumable way
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
/// ```
pub mod prelude {
    // Re-exports
    pub use std::{
        io::{Read, Seek, SeekFrom, Write},
        path::{Component, Path, PathBuf},
        sync::Arc,
    };

    // Export macros by name
    pub use crate::{
        assert_vfs_copyfile, assert_vfs_exists, assert_vfs_is_dir, assert_vfs_is_file, assert_vfs_is_symlink,
        assert_vfs_mkdir_m, assert_vfs_mkdir_p, assert_vfs_mkfile, assert_vfs_no_dir, assert_vfs_no_exists,
        assert_vfs_no_file, assert_vfs_no_symlink, assert_vfs_read_all, assert_vfs_readlink,
        assert_vfs_readlink_abs, assert_vfs_remove, assert_vfs_remove_all, assert_vfs_setup, assert_vfs_symlink,
        assert_vfs_write_all, cfgblock, defer, function, function_fqn, panic_compare_msg, panic_msg, trying,
        unwrap_or_false,
    };
    // Export internal types
    pub use crate::{
        core::*,
        errors::*,
        sys::{
            self, user, Chmod, Chown, Copier, Entries, EntriesIter, Entry, Memfs, MemfsEntry, PathExt, ReadSeek,
            Stdfs, StdfsEntry, Vfs, VfsEntry, VirtualFileSystem,
        },
        testing,
    };
}

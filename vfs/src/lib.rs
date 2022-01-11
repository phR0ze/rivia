//! `rivia-vfs` is a virtual filesystem implementation with an emphasis on ergonomics.
//!
//! The intent of `rivia-vfs` is to provide a trait that can be implemented to provide
//! a common set of functionality across different backend technologies e.g. std::fs or
//! memory based implementations and a simple mechanism for switching out one vfs
//! backend provider for another dynamically.
//!
//! ## Switching backend providers
//! By default the vfs backend provider will be set to `Stdfs` which is an implementation wrapping
//! the standard library `std::fs` and related functions to satisfy the `Vfs` trait; however you
//! change the backend provider by simply calling the `sys::vfs()` and pass in an impl for the
//! Vfs trait.
//!
//! ### Example
//! ```no_run
//! use rivia_vfs::prelude::*;
//!
//! vfs::set(Stdfs::new()).unwrap();
//! ```
// mod file;
// mod memfs;
// mod stdfs;
// pub use file::*;
// pub use memfs::*;
// pub use stdfs::*;

use std::{
    fmt::Debug,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use lazy_static::lazy_static;
use rivia::prelude::*;

/// All essential symbols in a simple consumable form
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
/// ```
pub mod prelude
{
    pub use rivia::prelude::*;

    // Nest global vfs functions for ergonomics
    pub mod vfs
    {
        pub use crate::*;
    }

    // Re-exports
    pub use std::path::{Path, PathBuf};
}

lazy_static! {
    /// VFS is a virtual filesystem singleton providing an implementation of Vfs that defaults to
    /// Stdfs but can be changed dynamically to any implementation of the Vfs trait.
    ///
    /// Arc is used here to provide the guarantee that the shared VFS instance is thread safe and
    /// is wrapped in a RwLock to provide the ability to change the VFS backend implementation if
    /// desired following the promoting pattern rather than interior mutability i.e. Arc<RwLock>>.
    /// Since changing the backend will be a rare occurance RwLock is used here rather than Mutex
    /// to provide many readers but only one writer which should be as efficient as possible.
    /// https://blog.sentry.io/2018/04/05/you-cant-rust-that
    pub static ref VFS: RwLock<Arc<Vfs>> = RwLock::new(Arc::new(Vfs::new_stdfs()));
}

// // Vfs trait
// // -------------------------------------------------------------------------------------------------

// /// Vfs provides a set of functions that are implemented by various backend filesystem providers.
// /// For example [`Stdfs`] implements a pass through to the sRust std::fs library that operates
// /// against disk as per usual and [`Memfs`] is an in memory implementation providing the same
// /// functionality only purely in memory.
// pub trait Vfs: Debug+Send+Sync+'static
// {
//     fn abs(&self, path: &Path) -> RvResult<PathBuf>;
//     // fn expand(&self, path: &Path) -> RvResult<PathBuf>;
//     // fn open(&self, path: &Path) -> RvResult<()>;
//     // fn mkfile(&self, path: &Path) -> RvResult<Box<dyn Write>>;
// }

// Vfs convenience functions
// -------------------------------------------------------------------------------------------------

/// Set the current vfs backend being used.
///
/// Following the promoting pattern we can switch the Vfs backend for the given implementation
/// while allowing current consumers that have a reference to the previous Vfs backend
/// implementation to complete their operations safely.
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// vfs::set(Stdfs::new()).unwrap();
/// ```
pub fn set(vfs: Vfs) -> RvResult<()>
{
    *VFS.write().map_err(|_| VfsError::Unavailable)? = Arc::new(vfs);
    Ok(())
}

/// Return the path in an absolute clean form
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// let home = sys::home_dir().unwrap();
/// assert_eq!(sys::abs("~").unwrap(), PathBuf::from(&home));
/// ```
pub fn abs<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
    VFS.read()
        .map_err(|_| VfsError::Unavailable)?
        .clone()
        .abs(path.as_ref())
}

// /// Expand all environment variables in the path as well as the home directory.
// ///
// /// ### Examples
// /// ```
// /// use rivia_vfs::prelude::*;
// ///
// /// let home = sys::home_dir().unwrap();
// /// assert_eq!(vfs::expand(Path::new("~/foo")).unwrap(), PathBuf::from(&home).join("foo"));
// /// assert_eq!(vfs::expand(Path::new("$HOME/foo")).unwrap(), PathBuf::from(&home).join("foo"));
// /// assert_eq!(vfs::expand(Path::new("${HOME}/foo")).unwrap(), PathBuf::from(&home).join("foo"));
// /// ```
// pub fn expand<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
// {
//     VFS.read().map_err(|_| VfsError::Unavailable)?.clone().expand(path.as_ref())
// }

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[test]
    fn test_vfs_abs() -> RvResult<()>
    {
        let cwd = Stdfs::cwd()?;

        assert_eq!(vfs::abs(Path::new("foo"))?, Stdfs::mash(&cwd, "foo"));
        Ok(())
    }
}

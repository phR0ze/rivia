//! # Memfs is a Vfs backend implementation that is purely memory based
//!
//! ### Example
/// ```no_run
/// use rivia_vfs::prelude::*;
/// ```
use crate::{path::VfsPath, Vfs};
use rivia_core::*;
use std::fmt::Debug;
use std::{
    fs::File,
    path::{Path, PathBuf},
};

/// `Memfs` is a Vfs backend implementation that is purely memory based
#[derive(Debug)]
pub struct Memfs;
impl Memfs
{
    /// Create a new instance of the Memfs Vfs backend implementation
    pub fn new() -> Self
    {
        Self
    }
}

impl Vfs for Memfs
{
    /// Return the path in an absolute clean form
    ///
    /// ### Examples
    /// ```
    /// use rivia_vfs::prelude::*;
    ///
    /// let stdfs = vfs::Stdfs::new();
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(stdfs.abs(Path::new("~")).unwrap(), PathBuf::from(&home));
    /// ```
    fn abs(&self, path: &Path) -> RvResult<PathBuf>
    {
        sys::abs(path)
    }

   /// Expand all environment variables in the path as well as the home directory.
    ///
    /// ### Examples
    /// ```
    /// use rivia_vfs::prelude::*;
    ///
    /// let stdfs = vfs::Stdfs::new();
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(stdfs.expand(Path::new("~/foo")).unwrap(), PathBuf::from(&home).join("foo"));
    /// assert_eq!(stdfs.expand(Path::new("$HOME/foo")).unwrap(), PathBuf::from(&home).join("foo"));
    /// assert_eq!(stdfs.expand(Path::new("${HOME}/foo")).unwrap(), PathBuf::from(&home).join("foo"));
    /// ```
    fn expand(&self, path: &Path) -> RvResult<PathBuf>
    {
        sys::expand(path)
    }
}
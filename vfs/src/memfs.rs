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
    /// Expand all environment variables in the path as well as the home directory.
    ///
    /// WARNING: Does not expand partials e.g. "/foo${BAR}ing/blah" only complete components
    /// e.g. "/foo/${BAR}/blah"
    ///
    /// ### Examples
    /// ```
    /// use rivia_core::*;
    /// 
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(PathBuf::from(&home), PathBuf::from("~/foo").expand().unwrap());
    /// ```
    fn expand(&self, path: &Path) -> RvResult<PathBuf>
    {
        sys::expand(path)
    }
}
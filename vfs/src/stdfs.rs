//! # Stdfs is a Vfs backend implementation that wraps the standard library `std::fs`
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

/// `Stdfs` is a Vfs backend implementation that wraps the standard library `std::fs`
/// functions for use with Vfs.
#[derive(Debug)]
pub struct Stdfs;
impl Stdfs
{
    /// Create a new instance of the Stdfs Vfs backend implementation
    pub fn new() -> Self
    {
        Self
    }
}

impl Vfs for Stdfs
{
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


// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_expand() -> RvResult<()> {
        let stdfs = vfs::Stdfs::new();
        assert_eq!(stdfs.expand(Path::new("~/foo"))?, sys::home_dir()?.join("foo"));
        Ok(())
    }
}
//! # Stdfs is a Vfs backend implementation that wraps the standard library `std::fs`
//!
//! ### Example
/// ```no_run
/// use rivia_vfs::prelude::*;
/// ```
use crate::{Vfs};
use rivia_core::*;
use std::fmt::Debug;
use std::{
    fs::File,
    io::Write,
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

    fn mkfile(&self, path: &Path) -> RvResult<Box<dyn Write>> {
        Ok(Box::new(File::create(path)?))
    }

    /// Attempts to open a file in read-only mode.
    ///
    /// ### Examples
    /// ```
    /// use rivia_vfs::prelude::*;
    ///
    /// let stdfs = vfs::Stdfs::new();
    /// ```
    fn open(&self, path: &Path) -> RvResult<()>
    {
        Ok(())
    }
}


// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[test]
    fn test_abs() -> RvResult<()> {
        let stdfs = vfs::Stdfs::new();
        assert_eq!(stdfs.abs(Path::new("~/"))?, sys::home_dir()?);
        Ok(())
    }

    #[test]
    fn test_expand() -> RvResult<()> {
        let stdfs = vfs::Stdfs::new();
        assert_eq!(stdfs.expand(Path::new("~/foo"))?, sys::home_dir()?.join("foo"));
        Ok(())
    }
}
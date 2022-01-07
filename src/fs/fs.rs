use crate::{
    errors::*,
    fs::{Memfs, Stdfs},
};
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

// Vfs trait
// -------------------------------------------------------------------------------------------------

/// Vfs provides a set of functions that are implemented by various backend filesystem providers.
/// For example [`Stdfs`] implements a pass through to the sRust std::fs library that operates
/// against disk as per usual and [`Memfs`] is an in memory implementation providing the same
/// functionality only purely in memory.
pub trait FileSystem: Debug+Send+Sync+'static
{

    /// Return the path in an absolute clean form
    fn abs(&self, path: &Path) -> RvResult<PathBuf>;

    //fn expand(&self, path: &Path) -> RvResult<PathBuf>;
    //fn open(&self, path: &Path) -> RvResult<()>;

    /// Write the given data to to the indicated file creating the file first if it doesn't exist
    /// or truncating it first if it does.
    fn mkfile(&self, path: &Path, data: &[u8]) -> RvResult<()>;

    /// Opens a file for writing, creating if it doesn't exist and truncating if it does
    //fn write(&self, path: &Path) -> RvResult<Box<dyn Write>>;

    /// Up cast the trait type to the enum wrapper
    fn upcast(self) -> Vfs;
}


/// Vfs enum wrapper provides easy access to the underlying filesystem type
#[derive(Debug)]
pub enum Vfs
{
    Stdfs(Stdfs),
    Memfs(Memfs),
}

impl Vfs
{
    /// Create a new instance of Memfs wrapped in the Vfs enum
    pub fn new_memfs() -> Vfs
    {
        Vfs::Memfs(Memfs::new())
    }

    /// Create a new instance of Stdfs wrapped in the Vfs enum
    pub fn new_stdfs() -> Vfs
    {
        Vfs::Stdfs(Stdfs::new())
    }
}

impl FileSystem for Vfs
{
    /// Return the path in an absolute clean form
    fn abs(&self, path: &Path) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.abs(path),
            Vfs::Memfs(x) => x.abs(path),
        }
    }

    /// Write the given data to to the indicated file creating the file first if it doesn't exist
    /// or truncating it first if it does.
    fn mkfile(&self, path: &Path, data: &[u8]) -> RvResult<()>
    {
        match self {
            Vfs::Stdfs(x) => x.mkfile(path, data),
            Vfs::Memfs(x) => x.mkfile(path, data),
        }
    }

    /// Up cast the trait type to the enum wrapper
    fn upcast(self) -> Vfs {
        match self {
            Vfs::Stdfs(x) => x.upcast(),
            Vfs::Memfs(x) => x.upcast(),
        }
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[test]
    fn test_fs_abs() -> RvResult<()> {
        let cwd = Stdfs::cwd()?;
        let vfs = Vfs::Stdfs(Stdfs::new());

        assert_eq!(vfs.abs(Path::new("foo"))?, Stdfs::mash(&cwd, "foo"));
        Ok(())
    }
}
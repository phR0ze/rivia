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
    fn abs(&self, path: &Path) -> RvResult<PathBuf>;
    //fn expand(&self, path: &Path) -> RvResult<PathBuf>;
    //fn open(&self, path: &Path) -> RvResult<()>;
    //fn mkfile(&self, path: &Path) -> RvResult<Box<dyn Write>>;
}


/// Vfs enum wrapper provides easy access to the underlying filesystem type
#[derive(Debug)]
pub enum Vfs
{
    Stdfs(Stdfs),
    Memfs(Memfs),
}

impl Vfs {
    pub fn new_memfs() -> Vfs {
        Vfs::Memfs(Memfs::new())
    }
    pub fn new_stdfs() -> Vfs {
        Vfs::Stdfs(Stdfs::new())
    }
}

impl FileSystem for Vfs
{
    fn abs(&self, path: &Path) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.abs(path),
            Vfs::Memfs(x) => x.abs(path),
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
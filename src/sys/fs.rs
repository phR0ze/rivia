use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use crate::{
    errors::*,
    sys::{Memfs, Stdfs},
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

    /// Returns the current working directory
    fn cwd(&self) -> RvResult<PathBuf>;

    /// Returns true if the `Path` exists. Handles path expansion.
    fn exists(&self, path: &Path) -> bool;

    //fn expand(&self, path: &Path) -> RvResult<PathBuf>;

    /// Creates the given directory and any parent directories needed, handling path expansion and
    /// returning the absolute path of the created directory
    fn mkdir_p(&mut self, path: &Path) -> RvResult<PathBuf>;

    //fn read(&self, path: &Path) -> RvResult<()>;

    /// Opens a file for writing, creating if it doesn't exist and truncating if it does
    //fn write(&self, path: &Path) -> RvResult<Box<dyn Write>>;

    /// Read all data from the given file and return it as a String
    fn read_all(&self, path: &Path) -> RvResult<String>;

    /// Write all the given data to to the indicated file creating the file first if it doesn't
    /// exist or truncating it first if it does.
    fn write_all(&self, path: &Path, data: &[u8]) -> RvResult<()>;

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

    /// Returns the current working directory
    fn cwd(&self) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.cwd(),
            Vfs::Memfs(x) => x.cwd(),
        }
    }

    /// Returns true if the `Path` exists. Handles path expansion.
    fn exists(&self, path: &Path) -> bool
    {
        match self {
            Vfs::Stdfs(x) => x.exists(path),
            Vfs::Memfs(x) => x.exists(path),
        }
    }

    /// Creates the given directory and any parent directories needed, handling path expansion and
    /// returning the absolute path of the created directory
    fn mkdir_p(&mut self, path: &Path) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.mkdir_p(path),
            Vfs::Memfs(x) => x.mkdir_p(path),
        }
    }

    /// Read all data from the given file and return it as a String
    fn read_all(&self, path: &Path) -> RvResult<String>
    {
        match self {
            Vfs::Stdfs(x) => x.read_all(path),
            Vfs::Memfs(x) => x.read_all(path),
        }
    }

    /// Write the given data to to the indicated file creating the file first if it doesn't exist
    /// or truncating it first if it does.
    fn write_all(&self, path: &Path, data: &[u8]) -> RvResult<()>
    {
        match self {
            Vfs::Stdfs(x) => x.write_all(path, data),
            Vfs::Memfs(x) => x.write_all(path, data),
        }
    }

    /// Up cast the trait type to the enum wrapper
    fn upcast(self) -> Vfs
    {
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
    fn test_fs_stdfs_read_write() -> RvResult<()>
    {
        // Manually doing this as I want to show the switching of vfs backends
        let tmpdir = sys::mash(testing::TEST_TEMP_DIR, "test_fs_stdfs_read_write");
        assert_stdfs_remove_all!(&tmpdir);
        assert_stdfs_mkdir_p!(&tmpdir);
        let file1 = sys::mash(&tmpdir, "file1");

        // Create the stdfs instance to test first with. Verify with Stdfs functions
        // directly as we haven't yet implemented the vfs functions.
        let vfs = Vfs::new_stdfs();

        // Write out the data to a new file
        let data_in = b"foobar";
        assert_stdfs_no_exists!(&file1);
        vfs.write_all(&file1, data_in)?;
        assert_stdfs_is_file!(&file1);

        // Read the data back in from th file
        let data_out = vfs.read_all(&file1)?;
        assert_eq!(data_in, data_out.as_bytes());

        assert_stdfs_remove_all!(&tmpdir);
        Ok(())
    }
}

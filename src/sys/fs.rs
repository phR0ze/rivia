use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use crate::{
    errors::*,
    sys::{Entries, Memfs, Stdfs},
};

/// FileSystem provides a set of functions that are implemented by various backend filesystem
/// providers. For example [`Stdfs`] implements a pass through to the sRust std::fs library that
/// operates against disk as per usual and [`Memfs`] is an in memory implementation providing the
/// same functionality only purely in memory.
pub trait FileSystem: Debug+Send+Sync+'static
{
    /// Return the path in an absolute clean form
    ///
    /// ### Provides:
    /// * environment variable expansion
    /// * relative path resolution for `.` and `..`
    /// * no IO resolution so it will work even with paths that don't exist
    ///
    /// ### Errors
    /// * PathError::ParentNotFound(PathBuf) when parent is not found
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(vfs.abs("~").unwrap(), PathBuf::from(&home));
    /// ```
    fn abs<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>;

    /// Returns the current working directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.cwd().unwrap(), PathBuf::from("/"));
    /// vfs.mkdir_p("foo").unwrap();
    /// vfs.set_cwd("foo").unwrap();
    /// assert_eq!(vfs.cwd().unwrap(), PathBuf::from("/foo"));
    /// ```
    fn cwd(&self) -> RvResult<PathBuf>;

    /// Set the current working directory
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * relative path will use the current working directory
    ///
    /// ### Errors
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.cwd().unwrap(), PathBuf::from("/"));
    /// vfs.mkdir_p("foo").unwrap();
    /// vfs.set_cwd("foo").unwrap();
    /// assert_eq!(vfs.cwd().unwrap(), PathBuf::from("/foo"));
    /// ```
    fn set_cwd<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>;

    /// Returns an iterator over the given path
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * recursive path traversal
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_stdfs_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("stdfs_func_entries");
    /// let file1 = tmpdir.mash("file1");
    /// assert_stdfs_mkfile!(&file1);
    /// let mut iter = Stdfs::entries(&file1).unwrap().into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    /// assert!(iter.next().is_none());
    /// assert_stdfs_remove_all!(&tmpdir);
    /// ```
    fn entries<T: AsRef<Path>>(&self, path: T) -> RvResult<Entries>;

    /// Returns true if the `path` exists
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.exists("/"), true);
    /// ```
    fn exists<T: AsRef<Path>>(&self, path: T) -> bool;

    /// Returns true if the given path exists and is a directory
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. links even if pointing to a directory return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.is_dir("foo"), false);
    /// let tmpdir = vfs.mkdir_p("foo").unwrap();
    /// assert_eq!(vfs.is_dir(&tmpdir), true);
    /// ```
    fn is_dir<T: AsRef<Path>>(&self, path: T) -> bool;

    /// Returns true if the given path exists and is a file
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. links even if pointing to a file return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.is_file("foo"), false);
    /// let tmpdir = vfs.mkfile("foo").unwrap();
    /// assert_eq!(vfs.is_file(&tmpdir), true);
    /// ```
    fn is_file<T: AsRef<Path>>(&self, path: T) -> bool;

    /// Creates the given directory and any parent directories needed
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    ///
    /// # Errors
    /// * io::Error if its unable to create the directory
    /// * PathError::IsNotDir(PathBuf) when the path already exists and is not a directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.exists("foo"), false);
    /// assert_eq!(vfs.mkdir_p("foo").unwrap(), PathBuf::from("/foo"));
    /// assert_eq!(vfs.exists("foo"), true);
    /// ```
    fn mkdir_p<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>;

    /// Create an empty file similar to the linux touch command
    ///
    /// ### Provides
    /// * handling path expansion and absolute path resolution
    /// * default file creation permissions 0o666 with umask usually ends up being 0o644
    ///
    /// ### Errors
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotDir(PathBuf) when the given path's parent isn't a directory
    /// * PathError::IsNotFile(PathBuf) when the given path exists but isn't a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.exists("file1"), false);
    /// assert_eq!(vfs.mkfile("file1").unwrap(), PathBuf::from("/file1"));
    /// assert_eq!(vfs.exists("file1"), true);
    /// ```
    fn mkfile<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>;

    // fn read(&self, path: &Path) -> RvResult<()>;

    /// Opens a file for writing, creating if it doesn't exist and truncating if it does
    // fn write(&self, path: &Path) -> RvResult<Box<dyn Write>>;

    /// Read all data from the given file and return it as a String
    fn read_all<T: AsRef<Path>>(&self, path: T) -> RvResult<String>;

    /// Removes the given empty directory or file
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. removes the link themselves not what its points to
    ///
    /// ### Errors
    /// * a directory containing files will trigger an error. use `remove_all` instead
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = PathBuf::from("foo");
    /// assert!(vfs.mkfile(&file).is_ok());
    /// assert_eq!(vfs.exists(&file), true);
    /// assert!(vfs.remove(&file).is_ok());
    /// assert_eq!(vfs.exists(&file), false);
    /// ```
    fn remove<T: AsRef<Path>>(&self, path: T) -> RvResult<()>;

    /// Removes the given directory after removing all of its contents
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. removes the link themselves not what its points to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert!(vfs.mkdir_p("foo").is_ok());
    /// assert_eq!(vfs.exists("foo"), true);
    /// assert!(vfs.mkfile("foo/bar").is_ok());
    /// assert_eq!(vfs.is_file("foo/bar"), true);
    /// assert!(vfs.remove_all("foo").is_ok());
    /// assert_eq!(vfs.exists("foo/ar"), false);
    /// assert_eq!(vfs.exists("foo"), false);
    /// ```
    fn remove_all<T: AsRef<Path>>(&self, path: T) -> RvResult<()>;

    /// Write all the given data to to the indicated file creating the file first if it doesn't
    /// exist or truncating it first if it does.
    fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()>;

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
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(vfs.abs("~").unwrap(), PathBuf::from(&home));
    /// ```
    pub fn memfs() -> Vfs
    {
        Vfs::Memfs(Memfs::new())
    }

    /// Create a new instance of Stdfs wrapped in the Vfs enum
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::stdfs();
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(vfs.abs("~").unwrap(), PathBuf::from(&home));
    /// ```
    pub fn stdfs() -> Vfs
    {
        Vfs::Stdfs(Stdfs::new())
    }
}

impl FileSystem for Vfs
{
    /// Return the path in an absolute clean form
    ///
    /// ### Provides:
    /// * environment variable expansion
    /// * relative path resolution for `.` and `..`
    /// * no IO resolution so it will work even with paths that don't exist
    ///
    /// ### Errors
    /// * PathError::ParentNotFound(PathBuf) when parent is not found
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(vfs.abs("~").unwrap(), PathBuf::from(&home));
    /// ```
    fn abs<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.abs(path),
            Vfs::Memfs(x) => x.abs(path),
        }
    }

    /// Returns the current working directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.cwd().unwrap(), PathBuf::from("/"));
    /// assert_eq!(vfs.mkdir_p("foo").unwrap(), PathBuf::from("/foo"));
    /// assert_eq!(vfs.set_cwd("foo").unwrap(), PathBuf::from("/foo"))
    /// assert_eq!(vfs.cwd().unwrap(), PathBuf::from("/foo"));
    /// ```
    fn cwd(&self) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.cwd(),
            Vfs::Memfs(x) => x.cwd(),
        }
    }

    /// Set the current working directory
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * relative path will use the current working directory
    ///
    /// ### Errors
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.cwd().unwrap(), PathBuf::from("/"));
    /// assert_eq!(vfs.mkdir_p("foo").unwrap(), PathBuf::from("/foo"));
    /// assert_eq!(vfs.set_cwd("foo").unwrap(), PathBuf::from("/foo"))
    /// assert_eq!(vfs.cwd().unwrap(), PathBuf::from("/foo"));
    /// ```
    fn set_cwd<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.set_cwd(path),
            Vfs::Memfs(x) => x.set_cwd(path),
        }
    }

    /// Returns an iterator over the given path
    fn entries<T: AsRef<Path>>(&self, path: T) -> RvResult<Entries>
    {
        match self {
            Vfs::Stdfs(x) => x.entries(path),
            Vfs::Memfs(x) => x.entries(path),
        }
    }

    /// Returns true if the `Path` exists. Handles path expansion.
    fn exists<T: AsRef<Path>>(&self, path: T) -> bool
    {
        match self {
            Vfs::Stdfs(x) => x.exists(path),
            Vfs::Memfs(x) => x.exists(path),
        }
    }

    /// Returns true if the given path exists and is a directory
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. links even if pointing to a directory return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.is_dir("foo"), false);
    /// let tmpdir = vfs.mkdir_p("foo").unwrap();
    /// assert_eq!(vfs.is_dir(&tmpdir), true);
    /// ```
    fn is_dir<T: AsRef<Path>>(&self, path: T) -> bool
    {
        match self {
            Vfs::Stdfs(x) => x.is_dir(path),
            Vfs::Memfs(x) => x.is_dir(path),
        }
    }

    /// Returns true if the given path exists and is a file
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. links even if pointing to a file return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.is_file("foo"), false);
    /// let tmpfile = vfs.mkfile("foo").unwrap();
    /// assert_eq!(vfs.is_file(&tmpfile), true);
    /// ```
    fn is_file<T: AsRef<Path>>(&self, path: T) -> bool
    {
        match self {
            Vfs::Stdfs(x) => x.is_file(path),
            Vfs::Memfs(x) => x.is_file(path),
        }
    }

    /// Create an empty file similar to the linux touch command
    ///
    /// ### Provides
    /// * handling path expansion and absolute path resolution
    /// * default file creation permissions 0o666 with umask usually ends up being 0o644
    ///
    /// ### Errors
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotDir(PathBuf) when the given path's parent isn't a directory
    /// * PathError::IsNotFile(PathBuf) when the given path exists but isn't a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.exists("file1"), false);
    /// assert_eq!(vfs.mkfile("file1").unwrap(), PathBuf::from("/file1"));
    /// assert_eq!(vfs.exists("file1"), true);
    /// ```
    fn mkfile<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.mkfile(path),
            Vfs::Memfs(x) => x.mkfile(path),
        }
    }

    /// Creates the given directory and any parent directories needed, handling path expansion and
    /// returning the absolute path of the created directory
    fn mkdir_p<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.mkdir_p(path),
            Vfs::Memfs(x) => x.mkdir_p(path),
        }
    }

    /// Read all data from the given file and return it as a String
    fn read_all<T: AsRef<Path>>(&self, path: T) -> RvResult<String>
    {
        match self {
            Vfs::Stdfs(x) => x.read_all(path),
            Vfs::Memfs(x) => x.read_all(path),
        }
    }

    /// Removes the given empty directory or file
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. removes the link themselves not what its points to
    ///
    /// ### Errors
    /// * a directory containing files will trigger an error. use `remove_all` instead
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = PathBuf::from("foo");
    /// assert!(vfs.mkfile(&file).is_ok());
    /// assert_eq!(vfs.exists(&file), true);
    /// assert!(vfs.remove(&file).is_ok());
    /// assert_eq!(vfs.exists(&file), false);
    /// ```
    fn remove<T: AsRef<Path>>(&self, path: T) -> RvResult<()>
    {
        match self {
            Vfs::Stdfs(x) => x.remove(path),
            Vfs::Memfs(x) => x.remove(path),
        }
    }

    /// Removes the given directory after removing all of its contents
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. removes the link themselves not what its points to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert!(vfs.mkdir_p("foo").is_ok());
    /// assert_eq!(vfs.exists("foo"), true);
    /// assert!(vfs.mkfile("foo/bar").is_ok());
    /// assert_eq!(vfs.is_file("foo/bar"), true);
    /// assert!(vfs.remove_all("foo").is_ok());
    /// assert_eq!(vfs.exists("foo/ar"), false);
    /// assert_eq!(vfs.exists("foo"), false);
    /// ```
    fn remove_all<T: AsRef<Path>>(&self, path: T) -> RvResult<()>
    {
        match self {
            Vfs::Stdfs(x) => x.remove_all(path),
            Vfs::Memfs(x) => x.remove_all(path),
        }
    }

    /// Write the given data to to the indicated file creating the file first if it doesn't exist
    /// or truncating it first if it does.
    fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()>
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
        let vfs = Vfs::stdfs();

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

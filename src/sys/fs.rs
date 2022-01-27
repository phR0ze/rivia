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
    /// * Environment variable expansion
    /// * Relative path resolution for `.` and `..`
    /// * No IO resolution so it will work even with paths that don't exist
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

    /// Returns an iterator over the given path
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Recursive path traversal
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
    /// * Handles path expansion and absolute path resolution
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
    /// * Handles path expansion and absolute path resolution
    /// * Link exclusion i.e. links even if pointing to a directory return false
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
    /// * Handles path expansion and absolute path resolution
    /// * Link exclusion i.e. links even if pointing to a file return false
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
    /// * Handles path expansion and absolute path resolution
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
    /// * Handles path expansion and absolute path resolution
    /// * Default file creation permissions 0o666 with umask usually ends up being 0o644
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

    /// Returns the path the given link points to
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_eq!(vfs.symlink(&link1, &file1).unwrap(), link1);
    /// assert_eq!(vfs.readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// ```
    fn readlink<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>;

    /// Removes the given empty directory or file
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Link exclusion i.e. removes the link themselves not what its points to
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
    /// * Handles path expansion and absolute path resolution
    /// * Link exclusion i.e. removes the link themselves not what its points to
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

    /// Set the current working directory
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Relative path will use the current working directory
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

    /// Creates a new symbolic link
    ///
    /// * Handles path expansion and absolute path resolution
    /// * computes the target path `src` relative to the `dst` link name's absolute path
    /// * returns the link path
    ///
    /// ### Arguments
    /// * `link` - the path of the link being created
    /// * `target` - the path that the link will point to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_eq!(vfs.symlink(&link1, &file1).unwrap(), link1);
    /// assert_eq!(vfs.readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// ```
    fn symlink<T: AsRef<Path>, U: AsRef<Path>>(&self, link: T, target: U) -> RvResult<PathBuf>;

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
    /// * Environment variable expansion
    /// * Relative path resolution for `.` and `..`
    /// * No IO resolution so it will work even with paths that don't exist
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
    /// * Handles path expansion and absolute path resolution
    /// * Link exclusion i.e. links even if pointing to a directory return false
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
    /// * Handles path expansion and absolute path resolution
    /// * Link exclusion i.e. links even if pointing to a file return false
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
    /// * Handles path expansion and absolute path resolution
    /// * Default file creation permissions 0o666 with umask usually ends up being 0o644
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

    /// Returns the path the given link points to
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::memfs(), Some("vfs_method_readlink"));
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_eq!(vfs.symlink(&link1, &file1).unwrap(), link1);
    /// assert_eq!(vfs.readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// ```
    fn readlink<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.readlink(path),
            Vfs::Memfs(x) => x.readlink(path),
        }
    }

    /// Removes the given empty directory or file
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Link exclusion i.e. removes the link themselves not what its points to
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
    /// * Handles path expansion and absolute path resolution
    /// * Link exclusion i.e. removes the link themselves not what its points to
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

    /// Set the current working directory
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Relative path will use the current working directory
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

    /// Creates a new symbolic link
    ///
    /// ### Arguments
    /// * `link` - the path of the link being created
    /// * `target` - the path that the link will point to
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Computes the target path `src` relative to the `dst` link name's absolute path
    /// * Returns the link path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::memfs(), Some("vfs_method_readlink"));
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_eq!(vfs.symlink(&link1, &file1).unwrap(), link1);
    /// assert_eq!(vfs.readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// ```
    fn symlink<T: AsRef<Path>, U: AsRef<Path>>(&self, link: T, target: U) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.symlink(link, target),
            Vfs::Memfs(x) => x.symlink(link, target),
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
    fn test_switching_vfs_backends()
    {
        switching_vfs_backends(assert_vfs_setup!(Vfs::memfs()));
        switching_vfs_backends(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn switching_vfs_backends((vfs, tmpdir): (Vfs, PathBuf))
    {
        // Create a file in a dir
        let dir = tmpdir.mash("dir");
        let file = dir.mash("file");
        assert_vfs_mkdir_p!(&vfs, &dir);
        assert_vfs_mkfile!(&vfs, &file);

        // Remove the file and the dir
        assert_vfs_remove!(&vfs, &file);
        assert_vfs_remove_all!(&vfs, &tmpdir);
    }
}

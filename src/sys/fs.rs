use std::{
    fmt::Debug,
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    errors::*,
    sys::{Entries, Memfs, Stdfs},
};

/// Provides a combination of Read + Seek traits
pub trait ReadSeek: std::io::Read+std::io::Seek
{
}

// Blanket implementation for any type that implements Read + Seek
impl<T> ReadSeek for T where T: std::io::Read+std::io::Seek {}

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

    /// Opens a file in write-only mode
    ///
    /// * Creates a file if it does not exist or truncates it if it does
    ///
    /// ### Errors
    /// * PathError::IsNotDir(PathBuf) when the given path's parent exists but is not a directory
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotFile(PathBuf) when the given path exists but is not a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// let mut f = vfs.create(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar".to_string());
    /// ```
    fn create<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>>;

    /// Returns the current working directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.cwd().unwrap(), vfs.root());
    /// assert_eq!(vfs.mkdir_p("foo").unwrap(), vfs.root().mash("foo"));
    /// assert_eq!(vfs.set_cwd("foo").unwrap(), vfs.root().mash("foo"));
    /// assert_eq!(vfs.cwd().unwrap(), vfs.root().mash("foo"));
    /// ```
    fn cwd(&self) -> RvResult<PathBuf>;

    /// Returns an iterator over the given path
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Handles recursive path traversal
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// let mut iter = vfs.entries(vfs.root()).unwrap().into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), vfs.root());
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &dir);
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &file);
    /// assert!(iter.next().is_none());
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
    /// assert_vfs_no_exists!(vfs, "foo");
    /// assert_vfs_mkdir_p!(vfs, "foo");
    /// assert_vfs_exists!(vfs, "foo");
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
    /// assert_vfs_no_dir!(vfs, "foo");
    /// assert_vfs_mkdir_p!(vfs, "foo");
    /// assert_vfs_is_dir!(vfs, "foo");
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
    /// assert_vfs_no_file!(vfs, "foo");
    /// assert_vfs_mkfile!(vfs, "foo");
    /// assert_vfs_is_file!(vfs, "foo");
    /// ```
    fn is_file<T: AsRef<Path>>(&self, path: T) -> bool;

    /// Returns true if the given path exists and is a symlink
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_vfs_no_symlink!(vfs, "foo");
    /// assert_vfs_symlink!(vfs, "foo", "bar");
    /// assert_vfs_is_symlink!(vfs, "foo");
    /// ```
    fn is_symlink<T: AsRef<Path>>(&self, path: T) -> bool;

    /// Creates the given directory and any parent directories needed
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// # Errors
    /// * PathError::IsNotDir(PathBuf) when the path already exists and is not a directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_vfs_no_dir!(vfs, "foo");
    /// assert_vfs_mkdir_p!(vfs, "foo");
    /// assert_vfs_is_dir!(vfs, "foo");
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
    /// assert_vfs_no_file!(vfs, "foo");
    /// assert_vfs_mkfile!(vfs, "foo");
    /// assert_vfs_is_file!(vfs, "foo");
    /// ```
    fn mkfile<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>;

    /// Attempts to open a file in readonly mode
    ///
    /// * Provides a handle to a Read + Seek implementation
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * PathError::IsNotFile(PathBuf) when the given path isn't a file
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// let mut file = vfs.open(&file).unwrap();
    /// let mut buf = String::new();
    /// file.read_to_string(&mut buf);
    /// assert_eq!(buf, "foobar 1".to_string());
    /// ```
    fn open<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn ReadSeek>>;

    /// Read all data from the given file and return it as a String
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * PathError::IsNotFile(PathBuf) when the given path isn't a file
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// assert_vfs_read_all!(vfs, &file, "foobar 1");
    /// ```
    fn read_all<T: AsRef<Path>>(&self, path: T) -> RvResult<String>;

    /// Returns the path the given link points to
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// assert_vfs_readlink!(vfs, &link, &file);
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
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_exists!(vfs, &file);
    /// assert_vfs_remove!(vfs, &file);
    /// assert_vfs_no_exists!(vfs, &file);
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
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_remove_all!(vfs, &dir);
    /// assert_vfs_no_exists!(vfs, &file);
    /// assert_vfs_no_exists!(vfs, &dir);
    /// ```
    fn remove_all<T: AsRef<Path>>(&self, path: T) -> RvResult<()>;

    /// Returns the current root directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let mut root = PathBuf::new();
    /// root.push(Component::RootDir);
    /// assert_eq!(vfs.root(), root);
    /// ```
    fn root(&self) -> PathBuf;

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
    /// let dir = vfs.root().mash("dir");
    /// assert_eq!(vfs.cwd().unwrap(), vfs.root());
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_eq!(&vfs.set_cwd(&dir).unwrap(), &dir);
    /// assert_eq!(&vfs.cwd().unwrap(), &dir);
    /// ```
    fn set_cwd<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>;

    /// Creates a new symbolic link
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Computes the target path `src` relative to the `dst` link name's absolute path
    /// * Returns the link path
    ///
    /// ### Arguments
    /// * `link` - the path of the link being created
    /// * `target` - the path that the link will point to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// assert_vfs_readlink!(vfs, &link, &file);
    /// ```
    fn symlink<T: AsRef<Path>, U: AsRef<Path>>(&self, link: T, target: U) -> RvResult<PathBuf>;

    /// Write the given data to to the target file
    ///
    /// * Create the file first if it doesn't exist or truncating it first if it does
    /// * `path` - target file to create or overwrite
    /// * `data` - data to write to the target file
    ///
    /// ### Errors
    /// * PathError::IsNotDir(PathBuf) when the given path's parent exists but is not a directory
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotFile(PathBuf) when the given path exists but is not a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());
    /// ```
    fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()>;

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new().upcast();
    /// ```
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
    /// assert_vfs_no_exists!(vfs, "humbug5");
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
    /// assert_vfs_no_exists!(vfs, "humbug5");
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

    /// Opens a file in write-only mode
    ///
    /// * Creates a file if it does not exist or truncates it if it does
    ///
    /// ### Errors
    /// * PathError::IsNotDir(PathBuf) when the given path's parent exists but is not a directory
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotFile(PathBuf) when the given path exists but is not a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// let mut f = vfs.create(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar".to_string());
    /// ```
    fn create<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>>
    {
        match self {
            Vfs::Stdfs(x) => x.create(path),
            Vfs::Memfs(x) => x.create(path),
        }
    }

    /// Returns the current working directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.cwd().unwrap(), vfs.root());
    /// assert_eq!(vfs.mkdir_p("foo").unwrap(), vfs.root().mash("foo"));
    /// assert_eq!(vfs.set_cwd("foo").unwrap(), vfs.root().mash("foo"));
    /// assert_eq!(vfs.cwd().unwrap(), vfs.root().mash("foo"));
    /// ```
    fn cwd(&self) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.cwd(),
            Vfs::Memfs(x) => x.cwd(),
        }
    }

    /// Returns an iterator over the given path
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Handles recursive path traversal
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// let mut iter = vfs.entries(vfs.root()).unwrap().into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), vfs.root());
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &dir);
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &file);
    /// assert!(iter.next().is_none());
    /// ```
    fn entries<T: AsRef<Path>>(&self, path: T) -> RvResult<Entries>
    {
        match self {
            Vfs::Stdfs(x) => x.entries(path),
            Vfs::Memfs(x) => x.entries(path),
        }
    }

    /// Returns true if the `path` exists
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_vfs_no_exists!(vfs, "foo");
    /// assert_vfs_mkdir_p!(vfs, "foo");
    /// assert_vfs_exists!(vfs, "foo");
    /// ```
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
    /// assert_vfs_no_dir!(vfs, "foo");
    /// assert_vfs_mkdir_p!(vfs, "foo");
    /// assert_vfs_is_dir!(vfs, "foo");
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
    /// assert_vfs_no_file!(vfs, "foo");
    /// assert_vfs_mkfile!(vfs, "foo");
    /// assert_vfs_is_file!(vfs, "foo");
    /// ```
    fn is_file<T: AsRef<Path>>(&self, path: T) -> bool
    {
        match self {
            Vfs::Stdfs(x) => x.is_file(path),
            Vfs::Memfs(x) => x.is_file(path),
        }
    }

    /// Returns true if the given path exists and is a symlink
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_vfs_no_symlink!(vfs, "foo");
    /// assert_vfs_symlink!(vfs, "foo", "bar");
    /// assert_vfs_is_symlink!(vfs, "foo");
    /// ```
    fn is_symlink<T: AsRef<Path>>(&self, path: T) -> bool
    {
        match self {
            Vfs::Stdfs(x) => x.is_symlink(path),
            Vfs::Memfs(x) => x.is_symlink(path),
        }
    }

    /// Creates the given directory and any parent directories needed
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// # Errors
    /// * PathError::IsNotDir(PathBuf) when the path already exists and is not a directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_vfs_no_dir!(vfs, "foo");
    /// assert_vfs_mkdir_p!(vfs, "foo");
    /// assert_vfs_is_dir!(vfs, "foo");
    /// ```
    fn mkdir_p<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.mkdir_p(path),
            Vfs::Memfs(x) => x.mkdir_p(path),
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
    /// assert_vfs_no_file!(vfs, "foo");
    /// assert_vfs_mkfile!(vfs, "foo");
    /// assert_vfs_is_file!(vfs, "foo");
    /// ```
    fn mkfile<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.mkfile(path),
            Vfs::Memfs(x) => x.mkfile(path),
        }
    }

    /// Attempts to open a file in readonly mode
    ///
    /// * Provides a handle to a Read + Seek implementation
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * PathError::IsNotFile(PathBuf) when the given path isn't a file
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// let mut file = vfs.open(&file).unwrap();
    /// let mut buf = String::new();
    /// file.read_to_string(&mut buf);
    /// assert_eq!(buf, "foobar 1".to_string());
    /// ```
    fn open<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn ReadSeek>>
    {
        match self {
            Vfs::Stdfs(x) => x.open(path),
            Vfs::Memfs(x) => x.open(path),
        }
    }

    /// Read all data from the given file and return it as a String
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * PathError::IsNotFile(PathBuf) when the given path isn't a file
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// assert_vfs_read_all!(vfs, &file, "foobar 1");
    /// ```
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
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// assert_vfs_readlink!(vfs, &link, &file);
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
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_exists!(vfs, &file);
    /// assert_vfs_remove!(vfs, &file);
    /// assert_vfs_no_exists!(vfs, &file);
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
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_remove_all!(vfs, &dir);
    /// assert_vfs_no_exists!(vfs, &file);
    /// assert_vfs_no_exists!(vfs, &dir);
    /// ```
    fn remove_all<T: AsRef<Path>>(&self, path: T) -> RvResult<()>
    {
        match self {
            Vfs::Stdfs(x) => x.remove_all(path),
            Vfs::Memfs(x) => x.remove_all(path),
        }
    }

    /// Returns the current root directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let mut root = PathBuf::new();
    /// root.push(Component::RootDir);
    /// assert_eq!(vfs.root(), root);
    /// ```
    fn root(&self) -> PathBuf
    {
        match self {
            Vfs::Stdfs(x) => x.root(),
            Vfs::Memfs(x) => x.root(),
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
    /// let dir = vfs.root().mash("dir");
    /// assert_eq!(vfs.cwd().unwrap(), vfs.root());
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_eq!(&vfs.set_cwd(&dir).unwrap(), &dir);
    /// assert_eq!(&vfs.cwd().unwrap(), &dir);
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
    /// * Handles path expansion and absolute path resolution
    /// * Computes the target path `src` relative to the `dst` link name's absolute path
    /// * Returns the link path
    ///
    /// ### Arguments
    /// * `link` - the path of the link being created
    /// * `target` - the path that the link will point to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// assert_vfs_readlink!(vfs, &link, &file);
    /// ```
    fn symlink<T: AsRef<Path>, U: AsRef<Path>>(&self, link: T, target: U) -> RvResult<PathBuf>
    {
        match self {
            Vfs::Stdfs(x) => x.symlink(link, target),
            Vfs::Memfs(x) => x.symlink(link, target),
        }
    }

    /// Write the given data to to the target file
    ///
    /// * Create the file first if it doesn't exist or truncating it first if it does
    /// * `path` - target file to create or overwrite
    /// * `data` - data to write to the target file
    ///
    /// ### Errors
    /// * PathError::IsNotDir(PathBuf) when the given path's parent exists but is not a directory
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotFile(PathBuf) when the given path exists but is not a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());
    /// ```
    fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()>
    {
        match self {
            Vfs::Stdfs(x) => x.write_all(path, data),
            Vfs::Memfs(x) => x.write_all(path, data),
        }
    }

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new().upcast();
    /// ```
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

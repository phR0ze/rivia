use std::{
    fmt::Debug,
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    errors::*,
    sys::{Chmod, Entries, Memfs, Stdfs, VfsEntry},
};

/// Defines a combination of the Read + Seek traits
pub trait ReadSeek: std::io::Read + std::io::Seek {}

// Blanket implementation for any type that implements Read + Seek
impl<T> ReadSeek for T where T: std::io::Read + std::io::Seek {}

/// Defines a virtual file system that can be implemented by various backed providers
pub trait VirtualFileSystem: Debug + Send + Sync + 'static {
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

    /// Opens a file in append mode
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Creates a file if it does not exist or appends to it if it does
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
    /// let mut f = vfs.append(&file).unwrap();
    /// f.write_all(b"123").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar123".to_string());
    /// ```
    fn append<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>>;

    /// Change all file/dir permissions recursivly to `mode`
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Doesn't follow links by default, use the builder `chomd_b` for this option
    ///
    /// ### Errors
    /// * PathError::Empty when the given path is empty
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod(&file, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    /// ```
    fn chmod<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<()>;

    /// Returns a new [`Chmod`] builder for advanced chmod options
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides options for recursion, following links, narrowing in on file types etc...
    ///
    /// ### Errors
    /// * PathError::Empty when the given path is empty
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
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
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40755);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod_b(&dir).unwrap().recurse().all(0o777).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40777);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100777);
    /// ```
    fn chmod_b<T: AsRef<Path>>(&self, path: T) -> RvResult<Chmod>;

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

    /// Returns all directories for the given path, sorted by name
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned as abs paths
    /// * Doesn't include the path itself only its children nor is this recursive
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_iter_eq(vfs.dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
    /// ```
    fn dirs<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>>;

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

    /// Return a virtual filesystem entry for the given path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.entry(&file).unwrap().is_file());
    /// ```
    fn entry<T: AsRef<Path>>(&self, path: T) -> RvResult<VfsEntry>;

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

    /// Returns all files for the given path, sorted by name
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned as abs paths
    /// * Doesn't include the path itself only its children nor is this recursive
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let dir1 = tmpdir.mash("dir1");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file2);
    /// assert_iter_eq(vfs.files(&tmpdir).unwrap(), vec![file1, file2]);
    /// ```
    fn files<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>>;

    /// Returns true if the given path exists and is readonly
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert!(vfs.mkfile_m(&file, 0o644).is_ok());
    /// assert_eq!(vfs.is_exec(&file), false);
    /// assert!(vfs.chmod(&file, 0o777).is_ok());
    /// assert_eq!(vfs.is_exec(&file), true);
    /// ```
    fn is_exec<T: AsRef<Path>>(&self, path: T) -> bool;

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

    /// Returns true if the given path exists and is readonly
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert!(vfs.mkfile_m(&file, 0o644).is_ok());
    /// assert_eq!(vfs.is_readonly(&file), false);
    /// assert!(vfs.chmod_b(&file).unwrap().readonly().exec().is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100444);
    /// assert_eq!(vfs.is_readonly(&file), true);
    /// ```
    fn is_readonly<T: AsRef<Path>>(&self, path: T) -> bool;

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

    /// Creates the given directory and any parent directories needed with the given mode
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let dir = vfs.root().mash("dir");
    /// assert!(vfs.mkdir_m(&dir, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40555);
    /// ```
    fn mkdir_m<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<PathBuf>;

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

    /// Wraps `mkfile` allowing for setting the file's mode.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert!(vfs.mkfile_m(&file, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    /// ```
    fn mkfile_m<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<PathBuf>;

    /// Returns the permissions for a file, directory or link
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * PathError::Empty when the given path is empty
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod(&file, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    /// ```
    fn mode<T: AsRef<Path>>(&self, path: T) -> RvResult<u32>;

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

    /// Returns all paths for the given path, sorted by name
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned as abs paths
    /// * Doesn't include the path itself only its children nor is this recursive
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_iter_eq(vfs.paths(&tmpdir).unwrap(), vec![dir1, dir2, file1]);
    /// ```
    fn paths<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>>;

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

    /// Returns the relative path of the target the link points to
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
    /// assert_eq!(vfs.set_cwd(&dir).unwrap(), dir.clone());
    /// assert_eq!(vfs.cwd().unwrap(), dir);
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
    /// * Handles path expansion and absolute path resolution
    /// * Create the file first if it doesn't exist or truncating it first if it does
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

/// Provides an ergonomic encapsulation of the underlying [`VirtualFileSystem`] backend
/// implementations
#[derive(Debug)]
pub enum Vfs {
    Stdfs(Stdfs),
    Memfs(Memfs),
}

impl Vfs {
    /// Create a new instance of Memfs wrapped in the Vfs enum
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_vfs_no_exists!(vfs, "humbug5");
    /// ```
    pub fn memfs() -> Vfs {
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
    pub fn stdfs() -> Vfs {
        Vfs::Stdfs(Stdfs::new())
    }
}

impl VirtualFileSystem for Vfs {
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
    fn abs<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf> {
        match self {
            Vfs::Stdfs(x) => x.abs(path),
            Vfs::Memfs(x) => x.abs(path),
        }
    }

    /// Opens a file in append mode
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Creates a file if it does not exist or appends to it if it does
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
    /// let mut f = vfs.append(&file).unwrap();
    /// f.write_all(b"123").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar123".to_string());
    /// ```
    fn append<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>> {
        match self {
            Vfs::Stdfs(x) => x.append(path),
            Vfs::Memfs(x) => x.append(path),
        }
    }

    /// Change all file/dir permissions recursivly to `mode`
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Doesn't follow links by default, use the builder `chomd_b` for this option
    ///
    /// ### Errors
    /// * PathError::Empty when the given path is empty
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod(&file, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    /// ```
    fn chmod<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<()> {
        match self {
            Vfs::Stdfs(x) => x.chmod(path, mode),
            Vfs::Memfs(x) => x.chmod(path, mode),
        }
    }

    /// Returns a new [`Chmod`] builder for advanced chmod options
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides options for recursion, following links, narrowing in on file types etc...
    ///
    /// ### Errors
    /// * PathError::Empty when the given path is empty
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
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
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40755);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod_b(&dir).unwrap().recurse().all(0o777).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40777);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100777);
    /// ```
    fn chmod_b<T: AsRef<Path>>(&self, path: T) -> RvResult<Chmod> {
        match self {
            Vfs::Stdfs(x) => x.chmod_b(path),
            Vfs::Memfs(x) => x.chmod_b(path),
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
    fn create<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>> {
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
    fn cwd(&self) -> RvResult<PathBuf> {
        match self {
            Vfs::Stdfs(x) => x.cwd(),
            Vfs::Memfs(x) => x.cwd(),
        }
    }

    /// Returns all directories for the given path, sorted by name
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned as abs paths
    /// * Doesn't include the path itself only its children nor is this recursive
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_iter_eq(vfs.dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
    /// ```
    fn dirs<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>> {
        match self {
            Vfs::Stdfs(x) => x.dirs(path),
            Vfs::Memfs(x) => x.dirs(path),
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
    fn entries<T: AsRef<Path>>(&self, path: T) -> RvResult<Entries> {
        match self {
            Vfs::Stdfs(x) => x.entries(path),
            Vfs::Memfs(x) => x.entries(path),
        }
    }

    /// Return a virtual filesystem entry for the given path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.entry(&file).unwrap().is_file());
    /// ```
    fn entry<T: AsRef<Path>>(&self, path: T) -> RvResult<VfsEntry> {
        match self {
            Vfs::Stdfs(x) => x.entry(path),
            Vfs::Memfs(x) => x.entry(path),
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
    fn exists<T: AsRef<Path>>(&self, path: T) -> bool {
        match self {
            Vfs::Stdfs(x) => x.exists(path),
            Vfs::Memfs(x) => x.exists(path),
        }
    }

    /// Returns all files for the given path, sorted by name
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned as abs paths
    /// * Doesn't include the path itself only its children nor is this recursive
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let dir1 = tmpdir.mash("dir1");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file2);
    /// assert_iter_eq(vfs.files(&tmpdir).unwrap(), vec![file1, file2]);
    /// ```
    fn files<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>> {
        match self {
            Vfs::Stdfs(x) => x.files(path),
            Vfs::Memfs(x) => x.files(path),
        }
    }

    /// Returns true if the given path exists and is readonly
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert!(vfs.mkfile_m(&file, 0o644).is_ok());
    /// assert_eq!(vfs.is_exec(&file), false);
    /// assert!(vfs.chmod(&file, 0o777).is_ok());
    /// assert_eq!(vfs.is_exec(&file), true);
    /// ```
    fn is_exec<T: AsRef<Path>>(&self, path: T) -> bool {
        match self {
            Vfs::Stdfs(x) => x.is_exec(path),
            Vfs::Memfs(x) => x.is_exec(path),
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
    fn is_dir<T: AsRef<Path>>(&self, path: T) -> bool {
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
    fn is_file<T: AsRef<Path>>(&self, path: T) -> bool {
        match self {
            Vfs::Stdfs(x) => x.is_file(path),
            Vfs::Memfs(x) => x.is_file(path),
        }
    }

    /// Returns true if the given path exists and is readonly
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert!(vfs.mkfile_m(&file, 0o644).is_ok());
    /// assert_eq!(vfs.is_readonly(&file), false);
    /// assert!(vfs.chmod_b(&file).unwrap().readonly().exec().is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100444);
    /// assert_eq!(vfs.is_readonly(&file), true);
    /// ```
    fn is_readonly<T: AsRef<Path>>(&self, path: T) -> bool {
        match self {
            Vfs::Stdfs(x) => x.is_readonly(path),
            Vfs::Memfs(x) => x.is_readonly(path),
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
    fn is_symlink<T: AsRef<Path>>(&self, path: T) -> bool {
        match self {
            Vfs::Stdfs(x) => x.is_symlink(path),
            Vfs::Memfs(x) => x.is_symlink(path),
        }
    }

    /// Creates the given directory and any parent directories needed with the given mode
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let dir = vfs.root().mash("dir");
    /// assert!(vfs.mkdir_m(&dir, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40555);
    /// ```
    fn mkdir_m<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<PathBuf> {
        match self {
            Vfs::Stdfs(x) => x.mkdir_m(path, mode),
            Vfs::Memfs(x) => x.mkdir_m(path, mode),
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
    fn mkdir_p<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf> {
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
    fn mkfile<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf> {
        match self {
            Vfs::Stdfs(x) => x.mkfile(path),
            Vfs::Memfs(x) => x.mkfile(path),
        }
    }

    /// Wraps `mkfile` allowing for setting the file's mode.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert!(vfs.mkfile_m(&file, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    /// ```
    fn mkfile_m<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<PathBuf> {
        match self {
            Vfs::Stdfs(x) => x.mkfile_m(path, mode),
            Vfs::Memfs(x) => x.mkfile_m(path, mode),
        }
    }

    /// Returns the permissions for a file, directory or link
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * PathError::Empty when the given path is empty
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod(&file, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    /// ```
    fn mode<T: AsRef<Path>>(&self, path: T) -> RvResult<u32> {
        match self {
            Vfs::Stdfs(x) => x.mode(path),
            Vfs::Memfs(x) => x.mode(path),
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
    fn open<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn ReadSeek>> {
        match self {
            Vfs::Stdfs(x) => x.open(path),
            Vfs::Memfs(x) => x.open(path),
        }
    }

    /// Returns all paths for the given path, sorted by name
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned as abs paths
    /// * Doesn't include the path itself only its children nor is this recursive
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_iter_eq(vfs.paths(&tmpdir).unwrap(), vec![dir1, dir2, file1]);
    /// ```
    fn paths<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>> {
        match self {
            Vfs::Stdfs(x) => x.paths(path),
            Vfs::Memfs(x) => x.paths(path),
        }
    }

    /// Re/// Read all data from the given file and return it as a String
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
    fn read_all<T: AsRef<Path>>(&self, path: T) -> RvResult<String> {
        match self {
            Vfs::Stdfs(x) => x.read_all(path),
            Vfs::Memfs(x) => x.read_all(path),
        }
    }

    /// Returns the relative path of the target the link points to
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
    fn readlink<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf> {
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
    fn remove<T: AsRef<Path>>(&self, path: T) -> RvResult<()> {
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
    fn remove_all<T: AsRef<Path>>(&self, path: T) -> RvResult<()> {
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
    fn root(&self) -> PathBuf {
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
    /// assert_eq!(vfs.set_cwd(&dir).unwrap(), dir.clone());
    /// assert_eq!(vfs.cwd().unwrap(), dir);
    /// ```
    fn set_cwd<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf> {
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
    fn symlink<T: AsRef<Path>, U: AsRef<Path>>(&self, link: T, target: U) -> RvResult<PathBuf> {
        match self {
            Vfs::Stdfs(x) => x.symlink(link, target),
            Vfs::Memfs(x) => x.symlink(link, target),
        }
    }

    /// Write the given data to to the target file
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Create the file first if it doesn't exist or truncating it first if it does
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
    fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()> {
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
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_vfs_cwd() {
        // Stdfs
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        assert_eq!(vfs.cwd().unwrap(), vfs.cwd().unwrap());
        assert_vfs_remove_all!(vfs, &tmpdir);

        // Memfs
        let vfs = Vfs::memfs();
        let mut root = PathBuf::new();
        root.push(Component::RootDir);
        assert_eq!(vfs.cwd().unwrap(), root);
    }

    #[test]
    fn test_vfs_dirs() {
        test_dirs(assert_vfs_setup!(Vfs::memfs()));
        test_dirs(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_dirs((vfs, tmpdir): (Vfs, PathBuf)) {
        let dir1 = tmpdir.mash("dir1");
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_iter_eq(vfs.dirs(&tmpdir).unwrap(), vec![dir1]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_files() {
        test_files(assert_vfs_setup!(Vfs::memfs()));
        test_files(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_files((vfs, tmpdir): (Vfs, PathBuf)) {
        let dir1 = tmpdir.mash("dir1");
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_iter_eq(vfs.files(&tmpdir).unwrap(), vec![file1, file2]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_paths() {
        test_paths(assert_vfs_setup!(Vfs::memfs()));
        test_paths(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_paths((vfs, tmpdir): (Vfs, PathBuf)) {
        let dir1 = tmpdir.mash("dir1");
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_iter_eq(vfs.paths(&tmpdir).unwrap(), vec![dir1, file1, file2]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_root() {
        test_root(assert_vfs_setup!(Vfs::memfs()));
        test_root(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_root((vfs, tmpdir): (Vfs, PathBuf)) {
        let mut root = PathBuf::new();
        root.push(Component::RootDir);
        assert_eq!(vfs.root(), root);
        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_upcast() {
        test_upcast(assert_vfs_setup!(Vfs::memfs()));
        test_upcast(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_upcast((vfs, tmpdir): (Vfs, PathBuf)) {
        let upcast = vfs.upcast();
        assert_vfs_remove_all!(upcast, &tmpdir);
    }
}

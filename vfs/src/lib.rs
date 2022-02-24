//! `rivia-vfs` provides an ergonomic veneer over rivia's VirtualFileSystem implementation
//!
//! This ergonomic approach allows for a clean namespaced usage of the VirtualFileSystem
//! implementation e.g. `vfs::abs()`. Additionally a mechanism is provided to seamlessly switch out
//! the VFS provider dynamically.
//!
//! ## Switching VFS providers
//! By default the VFS backend provider will be set to `Stdfs` which is an implementation wrapping
//! the standard library `std::fs` and related functions to satisfy the `VirtualFileSystem` trait;
//! however you change the backend provider by simply calling the `vfs::set()` function and pass in
//! a different variant of the [`Vfs`] enum.
//!
//! ### Example
//! ```
//! use rivia_vfs::prelude::*;
//!
//! assert!(vfs::set(Vfs::memfs()).is_ok());
//! ```
#[macro_use]
pub mod assert;
use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;
use rivia::prelude::*;

/// All essential symbols in a simple consumable form
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
/// ```
pub mod prelude
{
    pub use rivia::prelude::*;

    pub use crate::assert::*;
    // Export macros by name
    pub use crate::{
        assert_exists, assert_is_dir, assert_is_file, assert_is_symlink, assert_memfs_setup, assert_mkdir_m,
        assert_mkdir_p, assert_mkfile, assert_no_dir, assert_no_exists, assert_no_file, assert_no_symlink,
        assert_read_all, assert_readlink, assert_readlink_abs, assert_remove, assert_remove_all,
        assert_stdfs_setup, assert_symlink, assert_write_all,
    };

    // Nest global vfs functions for ergonomics
    pub mod vfs
    {
        pub use crate::*;
    }
}

lazy_static! {
    /// VFS is a virtual filesystem singleton providing an implementation of Vfs that defaults to
    /// Stdfs but can be changed dynamically to any variant of the Vfs enum.
    ///
    /// Arc is used here to provide the guarantee that the shared VFS instance is thread safe and
    /// is wrapped in a RwLock to provide the ability to change the VFS backend implementation if
    /// desired following the promoting pattern rather than interior mutability i.e. Arc<RwLock>>.
    /// Since changing the backend will be a rare occurance RwLock is used here rather than Mutex
    /// to provide many readers but only one writer which should be as efficient as possible.
    /// https://blog.sentry.io/2018/04/05/you-cant-rust-that
    pub static ref VFS: RwLock<Arc<Vfs>> = RwLock::new(Arc::new(Vfs::stdfs()));
}

/// Set the current vfs backend being used
///
/// Following the promoting pattern we can switch the Vfs backend for the given implementation
/// while allowing current consumers that have a reference to the previous Vfs backend
/// implementation to complete their operations safely.
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set(Vfs::memfs()).is_ok());
/// ```
pub fn set(vfs: Vfs) -> RvResult<()>
{
    // Replace the existing arc with a new one allowing the original to continue to
    // operate as long as there are references to it.
    *VFS.write().unwrap() = Arc::new(vfs);
    Ok(())
}

/// Switch the current vfs provider to Memfs
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// ```
pub fn set_memfs() -> RvResult<()>
{
    {
        // Skip set if not needed
        let guard = VFS.read().unwrap();
        if let Vfs::Memfs(ref _vfs) = &**guard {
            return Ok(());
        }
    }
    set(Vfs::memfs())
}

/// Switch the current vfs provider to Stdfs
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_stdfs().is_ok());
/// ```
pub fn set_stdfs() -> RvResult<()>
{
    {
        // Skip set if not needed
        let guard = VFS.read().unwrap();
        if let Vfs::Stdfs(ref _vfs) = &**guard {
            return Ok(());
        }
    }
    set(Vfs::stdfs())
}

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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let home = sys::home_dir().unwrap();
/// assert_eq!(vfs::abs("~").unwrap(), PathBuf::from(&home));
/// ```
pub fn abs<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
    VFS.read().unwrap().clone().abs(path)
}

/// Returns all dirs for the given path recursively
///
/// * Results are sorted by filename, are distict and don't include the given path
/// * Handles path expansion and absolute path resolution
/// * Paths are returned in absolute form
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let tmpdir = vfs::root().mash("tmpdir");
/// let dir1 = tmpdir.mash("dir1");
/// let dir2 = dir1.mash("dir2");
/// assert_mkdir_p!(&dir2);
/// assert_iter_eq(vfs::all_dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
/// ```
pub fn all_dirs<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>>
{
    VFS.read().unwrap().clone().all_dirs(path)
}

/// Returns all files for the given path recursively
///
/// * Results are sorted by filename, are distict and don't include the given path
/// * Handles path expansion and absolute path resolution
/// * Paths are returned in absolute form
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let tmpdir = vfs::root().mash("tmpdir");
/// let file1 = tmpdir.mash("file1");
/// let dir1 = tmpdir.mash("dir1");
/// let file2 = dir1.mash("file2");
/// assert_mkdir_p!(&dir1);
/// assert_mkfile!(&file1);
/// assert_mkfile!(&file2);
/// assert_iter_eq(vfs::all_files(&tmpdir).unwrap(), vec![file2, file1]);
/// ```
pub fn all_files<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>>
{
    VFS.read().unwrap().clone().all_files(path)
}

/// Returns all paths for the given path recursively
///
/// * Results are sorted by filename, are distict and don't include the given path
/// * Handles path expansion and absolute path resolution
/// * Paths are returned in absolute form
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let tmpdir = vfs::root().mash("tmpdir");
/// let dir1 = tmpdir.mash("dir1");
/// let file1 = tmpdir.mash("file1");
/// let file2 = dir1.mash("file2");
/// let file3 = dir1.mash("file3");
/// assert_mkdir_p!(&dir1);
/// assert_mkfile!(&file1);
/// assert_mkfile!(&file2);
/// assert_mkfile!(&file3);
/// assert_iter_eq(vfs::all_paths(&tmpdir).unwrap(), vec![dir1, file2, file3, file1]);
/// ```
pub fn all_paths<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>>
{
    VFS.read().unwrap().clone().all_paths(path)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// let mut f = vfs::create(&file).unwrap();
/// f.write_all(b"foobar").unwrap();
/// f.flush().unwrap();
/// let mut f = vfs::append(&file).unwrap();
/// f.write_all(b"123").unwrap();
/// f.flush().unwrap();
/// assert_read_all!(&file, "foobar123".to_string());
/// ```
pub fn append<T: AsRef<Path>>(path: T) -> RvResult<Box<dyn Write>>
{
    VFS.read().unwrap().clone().append(path)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert_mkfile!(&file);
/// assert_eq!(vfs::mode(&file).unwrap(), 0o100644);
/// assert!(vfs::chmod(&file, 0o555).is_ok());
/// assert_eq!(vfs::mode(&file).unwrap(), 0o100555);
/// ```
pub fn chmod<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<()>
{
    VFS.read().unwrap().clone().chmod(path, mode)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// let file = dir.mash("file");
/// assert_mkdir_p!(&dir);
/// assert_mkfile!(&file);
/// assert_eq!(vfs::mode(&dir).unwrap(), 0o40755);
/// assert_eq!(vfs::mode(&file).unwrap(), 0o100644);
/// assert!(vfs::chmod_b(&dir).unwrap().recurse().all(0o777).exec().is_ok());
/// assert_eq!(vfs::mode(&dir).unwrap(), 0o40777);
/// assert_eq!(vfs::mode(&file).unwrap(), 0o100777);
/// ```
pub fn chmod_b<T: AsRef<Path>>(path: T) -> RvResult<Chmod>
{
    VFS.read().unwrap().clone().chmod_b(path)
}

/// Change the ownership of the path recursivly
///
/// * Handles path expansion and absolute path resolution
/// * Use `chown_b` for more options
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file1 = vfs::root().mash("file1");
/// assert_mkfile!(&file1);
/// assert!(vfs::chown(&file1, 5, 7).is_ok());
/// assert_eq!(vfs::owner(&file1).unwrap(), (5, 7));
/// ```
pub fn chown<T: AsRef<Path>>(path: T, uid: u32, gid: u32) -> RvResult<()>
{
    VFS.read().unwrap().clone().chown(path, uid, gid)
}

/// Creates new [`Chown`] for use with the builder pattern
///
/// * Handles path expansion and absolute path resolution
/// * Provides options for recursion, following links, narrowing in on file types etc...
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file1 = vfs::root().mash("file1");
/// assert_mkfile!(&file1);
/// assert!(vfs::chown_b(&file1).unwrap().owner(5, 7).exec().is_ok());
/// assert_eq!(vfs::owner(&file1).unwrap(), (5, 7));
/// ```
pub fn chown_b<T: AsRef<Path>>(path: T) -> RvResult<Chown>
{
    VFS.read().unwrap().clone().chown_b(path)
}

/// Copies src to dst recursively
///
/// * `dst` will be copied into if it is an existing directory
/// * `dst` will be a copy of the src if it doesn't exist
/// * Creates destination directories as needed
/// * Handles environment variable expansion
/// * Handles relative path resolution for `.` and `..`
/// * Doesn't follow links
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file1 = vfs::root().mash("file1");
/// let file2 = vfs::root().mash("file2");
/// assert_write_all!(&file1, "this is a test");
/// assert!(vfs::copy(&file1, &file2).is_ok());
/// assert_read_all!(&file2, "this is a test");
/// ```
pub fn copy<T: AsRef<Path>, U: AsRef<Path>>(src: T, dst: U) -> RvResult<()>
{
    VFS.read().unwrap().clone().copy(src, dst)
}

/// Creates a new [`Copier`] for use with the builder pattern
///
/// * `dst` will be copied into if it is an existing directory
/// * `dst` will be a copy of the src if it doesn't exist
/// * Handles environment variable expansion
/// * Handles relative path resolution for `.` and `..`
/// * Options for recursion, mode setting and following links
/// * Execute by calling `exec`
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file1 = vfs::root().mash("file1");
/// let file2 = vfs::root().mash("file2");
/// assert_write_all!(&file1, "this is a test");
/// assert!(vfs::copy_b(&file1, &file2).unwrap().exec().is_ok());
/// assert_read_all!(&file2, "this is a test");
/// ```
pub fn copy_b<T: AsRef<Path>, U: AsRef<Path>>(src: T, dst: U) -> RvResult<Copier>
{
    VFS.read().unwrap().clone().copy_b(src, dst)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// let mut f = vfs::create(&file).unwrap();
/// f.write_all(b"foobar").unwrap();
/// f.flush().unwrap();
/// assert_read_all!(&file, "foobar");
/// ```
pub fn create<T: AsRef<Path>>(path: T) -> RvResult<Box<dyn Write>>
{
    VFS.read().unwrap().clone().create(path)
}

/// Returns the current working directory
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// assert_eq!(vfs::cwd().unwrap(), vfs::root());
/// assert_eq!(&vfs::mkdir_p(&dir).unwrap(), &dir);
/// assert_eq!(&vfs::set_cwd(&dir).unwrap(), &dir);
/// assert_eq!(&vfs::cwd().unwrap(), &dir);
/// ```
pub fn cwd() -> RvResult<PathBuf>
{
    VFS.read().unwrap().clone().cwd()
}

/// Returns all directories for the given path, sorted by name
///
/// * Handles path expansion and absolute path resolution
/// * Paths are returned as abs paths
/// * Doesn't include the path itself only its children nor is this recursive
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let tmpdir = vfs::root().mash("tmpdir");
/// let dir1 = tmpdir.mash("dir1");
/// let dir2 = tmpdir.mash("dir2");
/// let file1 = tmpdir.mash("file1");
/// assert_mkdir_p!(&dir1);
/// assert_mkdir_p!(&dir2);
/// assert_mkfile!(&file1);
/// assert_iter_eq(vfs::dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
/// ```
pub fn dirs<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>>
{
    VFS.read().unwrap().clone().dirs(path)
}

/// Returns an iterator over the given path
///
/// * Handles path expansion and absolute path resolution
/// * Handles recursive path traversal
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// let file = dir.mash("file");
/// assert_mkdir_p!(&dir);
/// assert_mkfile!(&file);
/// let mut iter = vfs::entries(vfs::root()).unwrap().into_iter();
/// assert_iter_eq(iter.map(|x| x.unwrap().path_buf()), vec![vfs::root(), dir, file]);
/// ```
pub fn entries<T: AsRef<Path>>(path: T) -> RvResult<Entries>
{
    VFS.read().unwrap().clone().entries(path)
}

/// Return a virtual filesystem entry for the given path
///
/// * Handles converting path to absolute form
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert_mkfile!(&file);
/// assert!(vfs::entry(&file).unwrap().is_file());
/// ```
pub fn entry<T: AsRef<Path>>(path: T) -> RvResult<VfsEntry>
{
    VFS.read().unwrap().clone().entry(path)
}

/// Returns true if the `path` exists
///
/// * Handles path expansion and absolute path resolution
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("foo");
/// assert_eq!(vfs::exists(&dir), false);
/// assert_mkdir_p!(&dir);
/// assert_eq!(vfs::exists(&dir), true);
/// ```
pub fn exists<T: AsRef<Path>>(path: T) -> bool
{
    VFS.read().unwrap().clone().exists(path)
}

/// Returns all files for the given path, sorted by name
///
/// * Handles path expansion and absolute path resolution
/// * Paths are returned as abs paths
/// * Doesn't include the path itself only its children nor is this recursive
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let tmpdir = vfs::root().mash("tmpdir");
/// let dir1 = tmpdir.mash("dir1");
/// let file1 = tmpdir.mash("file1");
/// let file2 = tmpdir.mash("file2");
/// assert_mkdir_p!(&dir1);
/// assert_mkfile!(&file1);
/// assert_mkfile!(&file2);
/// assert_iter_eq(vfs::files(&tmpdir).unwrap(), vec![file1, file2]);
/// ```
pub fn files<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>>
{
    VFS.read().unwrap().clone().files(path)
}

/// Returns the group ID of the owner of this file
///
/// * Handles path expansion and absolute path resolution
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_eq!(vfs::gid(vfs::root()).unwrap(), 1000);
/// ```
pub fn gid<T: AsRef<Path>>(path: T) -> RvResult<u32>
{
    VFS.read().unwrap().clone().gid(path)
}

/// Returns true if the given path exists and is readonly
///
/// * Handles path expansion and absolute path resolution
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert!(vfs::mkfile_m(&file, 0o644).is_ok());
/// assert_eq!(vfs::is_exec(&file), false);
/// assert!(vfs::chmod(&file, 0o777).is_ok());
/// assert_eq!(vfs::is_exec(&file), true);
/// ```
pub fn is_exec<T: AsRef<Path>>(path: T) -> bool
{
    VFS.read().unwrap().clone().is_exec(path)
}

/// Returns true if the given path exists and is a directory
///
/// * Handles path expansion and absolute path resolution
/// * Link exclusion i.e. links even if pointing to a directory return false
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// assert_eq!(vfs::is_dir(&dir), false);
/// assert_mkdir_p!(&dir);
/// assert_eq!(vfs::is_dir(&dir), true);
/// ```
pub fn is_dir<T: AsRef<Path>>(path: T) -> bool
{
    VFS.read().unwrap().clone().is_dir(path)
}

/// Returns true if the given path exists and is a file
///
/// * Handles path expansion and absolute path resolution
/// * Link exclusion i.e. links even if pointing to a file return false
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert_eq!(vfs::is_file(&file), false);
/// assert_mkfile!(&file);
/// assert_eq!(vfs::is_file(&file), true);
/// ```
pub fn is_file<T: AsRef<Path>>(path: T) -> bool
{
    VFS.read().unwrap().clone().is_file(path)
}

/// Returns true if the given path exists and is readonly
///
/// * Handles path expansion and absolute path resolution
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert!(vfs::mkfile_m(&file, 0o644).is_ok());
/// assert_eq!(vfs::is_readonly(&file), false);
/// assert!(vfs::chmod_b(&file).unwrap().readonly().exec().is_ok());
/// assert_eq!(vfs::mode(&file).unwrap(), 0o100444);
/// assert_eq!(vfs::is_readonly(&file), true);
/// ```
pub fn is_readonly<T: AsRef<Path>>(path: T) -> bool
{
    VFS.read().unwrap().clone().is_readonly(path)
}

/// Returns true if the given path exists and is a symlink
///
/// * Handles path expansion and absolute path resolution
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// let link = vfs::root().mash("link");
/// assert_eq!(vfs::is_symlink(&link), false);
/// assert_symlink!(&link, &file);
/// assert_eq!(vfs::is_symlink(&link), true);
/// ```
pub fn is_symlink<T: AsRef<Path>>(path: T) -> bool
{
    VFS.read().unwrap().clone().is_symlink(path)
}

/// Returns true if the given path exists and is a symlink pointing to a directory
///
/// * Handles path expansion and absolute path resolution
/// * Checks the path itself and what it points to
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// let file = vfs::root().mash("file");
/// let link1 = vfs::root().mash("link1");
/// let link2 = vfs::root().mash("link2");
/// assert_mkdir_p!(&dir);
/// assert_mkfile!(&file);
/// assert_symlink!(&link1, &dir);
/// assert_symlink!(&link2, &file);
/// assert_eq!(vfs::is_symlink_dir(&link1), true);
/// assert_eq!(vfs::is_symlink_dir(&link2), false);
/// ```
pub fn is_symlink_dir<T: AsRef<Path>>(path: T) -> bool
{
    VFS.read().unwrap().clone().is_symlink_dir(path)
}

/// Returns true if the given path exists and is a symlink pointing to a file
///
/// * Handles path expansion and absolute path resolution
/// * Checks the path itself and what it points to
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// let file = vfs::root().mash("file");
/// let link1 = vfs::root().mash("link1");
/// let link2 = vfs::root().mash("link2");
/// assert_mkdir_p!(&dir);
/// assert_mkfile!(&file);
/// assert_symlink!(&link1, &dir);
/// assert_symlink!(&link2, &file);
/// assert_eq!(vfs::is_symlink_file(&link1), false);
/// assert_eq!(vfs::is_symlink_file(&link2), true);
/// ```
pub fn is_symlink_file<T: AsRef<Path>>(path: T) -> bool
{
    VFS.read().unwrap().clone().is_symlink_file(path)
}

/// Creates the given directory and any parent directories needed with the given mode
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// assert!(vfs::mkdir_m(&dir, 0o555).is_ok());
/// assert_eq!(vfs::mode(&dir).unwrap(), 0o40555);
/// ```
pub fn mkdir_m<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<PathBuf>
{
    VFS.read().unwrap().clone().mkdir_m(path, mode)
}

/// Creates the given directory and any parent directories needed
///
/// * Handles path expansion and absolute path resolution
///
/// ### Errors
/// * PathError::IsNotDir(PathBuf) when the path already exists and is not a directory
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// assert_no_dir!(&dir);
/// assert_eq!(&vfs::mkdir_p(&dir).unwrap(), &dir);
/// assert_is_dir!(&dir);
/// ```
pub fn mkdir_p<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
    VFS.read().unwrap().clone().mkdir_p(path)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert_no_file!(&file);
/// assert_eq!(&vfs::mkfile(&file).unwrap(), &file);
/// assert_is_file!(&file);
/// ```
pub fn mkfile<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
    VFS.read().unwrap().clone().mkfile(path)
}

/// Wraps `mkfile` allowing for setting the file's mode.
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert!(vfs::mkfile_m(&file, 0o555).is_ok());
/// assert_eq!(vfs::mode(&file).unwrap(), 0o100555);
/// ```
pub fn mkfile_m<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<PathBuf>
{
    VFS.read().unwrap().clone().mkfile_m(path, mode)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert_mkfile!(&file);
/// assert_eq!(vfs::mode(&file).unwrap(), 0o100644);
/// assert!(vfs::chmod(&file, 0o555).is_ok());
/// assert_eq!(vfs::mode(&file).unwrap(), 0o100555);
/// ```
pub fn mode<T: AsRef<Path>>(path: T) -> RvResult<u32>
{
    VFS.read().unwrap().clone().mode(path)
}

/// Move a file or directory
///
/// * Handles path expansion and absolute path resolution
/// * Always moves `src` into `dst` if `dst` is an existing directory
/// * Replaces destination files if they exist
///
/// ### Errors
/// * PathError::DoesNotExist when the source doesn't exist
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// let file = vfs::root().mash("file");
/// let dirfile = dir.mash("file");
/// assert_mkdir_p!(&dir);
/// assert_mkfile!(&file);
/// assert!(vfs::move_p(&file, &dir).is_ok());
/// assert_no_file!(&file);
/// assert_is_file!(&dirfile);
/// ```
pub fn move_p<T: AsRef<Path>, U: AsRef<Path>>(src: T, dst: U) -> RvResult<()>
{
    VFS.read().unwrap().clone().move_p(src, dst)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert_write_all!(&file, b"foobar 1");
/// let mut file = vfs::open(&file).unwrap();
/// let mut buf = String::new();
/// file.read_to_string(&mut buf);
/// assert_eq!(buf, "foobar 1".to_string());
/// ```
pub fn open<T: AsRef<Path>>(path: T) -> RvResult<Box<dyn ReadSeek>>
{
    VFS.read().unwrap().clone().open(path)
}

/// Returns the (user ID, group ID) of the owner of this file
///
/// * Handles path expansion and absolute path resolution
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_eq!(vfs::owner(vfs::root()).unwrap(), (1000, 1000));
/// ```
pub fn owner<T: AsRef<Path>>(path: T) -> RvResult<(u32, u32)>
{
    VFS.read().unwrap().clone().owner(path)
}

/// Returns all paths for the given path, sorted by name
///
/// * Handles path expansion and absolute path resolution
/// * Paths are returned as abs paths
/// * Doesn't include the path itself only its children nor is this recursive
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let tmpdir = vfs::root().mash("tmpdir");
/// let dir1 = tmpdir.mash("dir1");
/// let dir2 = tmpdir.mash("dir2");
/// let file1 = tmpdir.mash("file1");
/// assert_mkdir_p!(&dir1);
/// assert_mkdir_p!(&dir2);
/// assert_mkfile!(&file1);
/// assert_iter_eq(vfs::paths(&tmpdir).unwrap(), vec![dir1, dir2, file1]);
/// ```
pub fn paths<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>>
{
    VFS.read().unwrap().clone().paths(path)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert_write_all!(&file, b"foobar 1");
/// assert_read_all!(&file, "foobar 1");
/// ```
pub fn read_all<T: AsRef<Path>>(path: T) -> RvResult<String>
{
    VFS.read().unwrap().clone().read_all(path)
}

/// Returns the relative path of the target the link points to
///
/// * Handles path expansion and absolute path resolution
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// let link = dir.mash("link");
/// let file = vfs::root().mash("file");
/// assert_mkdir_p!(&dir);
/// assert_mkfile!(&file);
/// assert_symlink!(&link, &file);
/// assert_readlink!(&link, PathBuf::from("..").mash("file"));
/// ```
pub fn readlink<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
    VFS.read().unwrap().clone().readlink(path)
}

/// Returns the absolute path of the target the link points to
///
/// * Handles path expansion and absolute path resolution
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// let link = vfs::root().mash("link");
/// assert_mkfile!(&file);
/// assert_symlink!(&link, &file);
/// assert_readlink_abs!(&link, &file);
/// ```
pub fn readlink_abs<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
    VFS.read().unwrap().clone().readlink_abs(path)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert_mkfile!(&file);
/// assert_exists!(&file);
/// assert_remove!(&file);
/// assert_no_exists!(&file);
/// ```
pub fn remove<T: AsRef<Path>>(path: T) -> RvResult<()>
{
    VFS.read().unwrap().clone().remove(path)
}

/// Removes the given directory after removing all of its contents
///
/// * Handles path expansion and absolute path resolution
/// * Link exclusion i.e. removes the link themselves not what its points to
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// let file = dir.mash("file");
/// assert_mkdir_p!(&dir);
/// assert_mkfile!(&file);
/// assert_is_file!(&file);
/// assert_remove_all!(&dir);
/// assert_no_exists!(&file);
/// assert_no_exists!(&dir);
/// ```
pub fn remove_all<T: AsRef<Path>>(path: T) -> RvResult<()>
{
    VFS.read().unwrap().clone().remove_all(path)
}

/// Returns the current root directory
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let mut root = PathBuf::new();
/// root.push(Component::RootDir);
/// assert_eq!(vfs::root(), root);
/// ```
pub fn root() -> PathBuf
{
    VFS.read().unwrap().clone().root()
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let dir = vfs::root().mash("dir");
/// assert_eq!(vfs::cwd().unwrap(), vfs::root());
/// assert_mkdir_p!(&dir);
/// assert_eq!(vfs::set_cwd(&dir).unwrap(), dir.clone());
/// assert_eq!(vfs::cwd().unwrap(), dir);
/// ```
pub fn set_cwd<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
    VFS.read().unwrap().clone().set_cwd(path)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// let link = vfs::root().mash("link");
/// assert_mkfile!(&file);
/// assert_symlink!(&link, &file);
/// assert_readlink_abs!(&link, &file);
/// ```
pub fn symlink<T: AsRef<Path>, U: AsRef<Path>>(link: T, target: U) -> RvResult<PathBuf>
{
    VFS.read().unwrap().clone().symlink(link, target)
}

/// Returns the user ID of the owner of this file
///
/// * Handles path expansion and absolute path resolution
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_eq!(vfs::uid(vfs::root()).unwrap(), 1000);
/// ```
pub fn uid<T: AsRef<Path>>(path: T) -> RvResult<u32>
{
    VFS.read().unwrap().clone().uid(path)
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
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file = vfs::root().mash("file");
/// assert_no_file!(&file);
/// assert_write_all!(&file, "foobar 1");
/// assert_is_file!(&file);
/// assert_read_all!(&file, "foobar 1");
/// ```
pub fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(path: T, data: U) -> RvResult<()>
{
    VFS.read().unwrap().clone().write_all(path, data)
}

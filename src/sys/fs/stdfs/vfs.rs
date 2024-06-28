use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    os::unix::{self, fs::MetadataExt, fs::PermissionsExt},
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use nix::sys::{
    stat::{self, UtimensatFlags},
    time::TimeSpec,
};

use super::{StdfsEntry, StdfsEntryIter};
use crate::{
    core::*,
    errors::*,
    sys::{
        self, Chmod, ChmodOpts, Chown, ChownOpts, Copier, CopyOpts, Entries, Entry, EntryIter, PathExt, ReadSeek,
        Vfs, VfsEntry, VirtualFileSystem,
    },
};

use super::Stdfs;

impl VirtualFileSystem for Stdfs {
    /// Return the path in an absolute clean form
    ///
    /// * Handles environment variable expansion
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
    /// let stdfs = Stdfs::new();
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(stdfs.abs("~").unwrap(), PathBuf::from(&home));
    /// ```
    fn abs<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf> {
        Stdfs::abs(path)
    }

    /// Returns all dirs for the given path recursively
    ///
    /// * Results are sorted by filename, are distict and don't include the given path
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned in absolute form
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_all_dirs");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = dir1.mash("dir2");
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_iter_eq(vfs.all_dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn all_dirs<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>> {
        Stdfs::all_dirs(path)
    }

    /// Returns all files for the given path recursively
    ///
    /// * Results are sorted by filename, are distict and don't include the given path
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned in absolute form
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_all_files");
    /// let file1 = tmpdir.mash("file1");
    /// let dir1 = tmpdir.mash("dir1");
    /// let file2 = dir1.mash("file2");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file2);
    /// assert_iter_eq(vfs.all_files(&tmpdir).unwrap(), vec![file2, file1]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn all_files<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>> {
        Stdfs::all_files(path)
    }

    /// Returns all paths for the given path recursively
    ///
    /// * Results are sorted by filename, are distict and don't include the given path
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned in absolute form
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_all_paths");
    /// let file1 = tmpdir.mash("file1");
    /// let dir1 = tmpdir.mash("dir1");
    /// let file2 = dir1.mash("file2");
    /// let file3 = dir1.mash("file3");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file2);
    /// assert_vfs_mkfile!(vfs, &file3);
    /// assert_iter_eq(vfs.all_paths(&tmpdir).unwrap(), vec![dir1, file2, file3, file1]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn all_paths<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>> {
        Stdfs::all_paths(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_append");
    /// let file = tmpdir.mash("file");
    /// let mut f = vfs.write(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// let mut f = vfs.append(&file).unwrap();
    /// f.write_all(b"123").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar123");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn append<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>> {
        Stdfs::append(path)
    }

    /// Append the given data to to the target file
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_append_all");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, "foobar 1");
    /// assert!(vfs.append_all(&file, "foobar 2").is_ok());
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1foobar 2");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn append_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()> {
        Stdfs::append_all(path, data)
    }

    /// Append the given line to to the target file including a newline
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_append_line");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, "foobar 1");
    /// assert!(vfs.append_line(&file, "foobar 2").is_ok());
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1foobar 2\n");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn append_line<T: AsRef<Path>, U: AsRef<str>>(&self, path: T, line: U) -> RvResult<()> {
        Stdfs::append_line(path, line)
    }

    /// Append the given lines to to the target file including newlines
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_append_lines");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert!(vfs.append_lines(&file, &["1", "2"]).is_ok());
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "1\n2\n");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn append_lines<T: AsRef<Path>, U: AsRef<str>>(&self, path: T, lines: &[U]) -> RvResult<()> {
        Stdfs::append_lines(path, lines)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_chmod");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod(&file, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn chmod<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<()> {
        Stdfs::chmod(path, mode)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_chmod_b");
    /// let dir = tmpdir.mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40755);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod_b(&dir).unwrap().recurse().all(0o777).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40777);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100777);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn chmod_b<T: AsRef<Path>>(&self, path: T) -> RvResult<Chmod> {
        Stdfs::chmod_b(path)
    }

    /// Change the ownership of the path recursivly
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Use `chown_b` for more options
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_chown");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// let (uid, gid) = Stdfs::owner(&file1).unwrap();
    /// assert!(Stdfs::chown(&file1, uid, gid).is_ok());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn chown<T: AsRef<Path>>(&self, path: T, uid: u32, gid: u32) -> RvResult<()> {
        Stdfs::chown(path, uid, gid)
    }

    /// Creates new [`Chown`] for use with the builder pattern
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides options for recursion, following links, narrowing in on file types etc...
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_chown_b");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// let (uid, gid) = Stdfs::owner(&file1).unwrap();
    /// assert!(Stdfs::chown_b(&file1).unwrap().owner(uid, gid).exec().is_ok());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn chown_b<T: AsRef<Path>>(&self, path: T) -> RvResult<Chown> {
        Stdfs::chown_b(path)
    }

    /// Returns the highest priority active configuration directory.
    ///
    /// * Searches first the $XDG_CONFIG_HOME directory, then the $XDG_CONFIG_DIRS directories.
    /// * Returns the first directory that contains the given configuration file.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// ```
    fn config_dir<T: AsRef<str>>(&self, config: T) -> Option<PathBuf> {
        Stdfs::config_dir(config)
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
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_copy");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// assert_vfs_write_all!(vfs, &file1, "this is a test");
    /// assert!(vfs.copy(&file1, &file2).is_ok());
    /// assert_vfs_read_all!(vfs, &file2, "this is a test");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn copy<T: AsRef<Path>, U: AsRef<Path>>(&self, src: T, dst: U) -> RvResult<()> {
        Stdfs::copy(src, dst)
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
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_copy_b");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// assert_vfs_write_all!(vfs, &file1, "this is a test");
    /// assert!(vfs.copy_b(&file1, &file2).unwrap().exec().is_ok());
    /// assert_vfs_read_all!(vfs, &file2, "this is a test");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn copy_b<T: AsRef<Path>, U: AsRef<Path>>(&self, src: T, dst: U) -> RvResult<Copier> {
        Stdfs::copy_b(src, dst)
    }

    /// Returns the current working directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let stdfs = Stdfs::new();
    /// stdfs.set_cwd(stdfs.cwd().unwrap().mash("tests"));
    /// assert_eq!(stdfs.cwd().unwrap().base().unwrap(), "tests".to_string());
    /// ```
    fn cwd(&self) -> RvResult<PathBuf> {
        Stdfs::cwd()
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_dirs");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_iter_eq(vfs.dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn dirs<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>> {
        Stdfs::dirs(path)
    }

    /// Returns an iterator over the given path
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides recursive path traversal
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_entries");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// let mut iter = vfs.entries(&file1).unwrap().into_iter();
    /// assert_iter_eq(iter.map(|x| x.unwrap().path_buf()), vec![file1]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn entries<T: AsRef<Path>>(&self, path: T) -> RvResult<Entries> {
        Stdfs::entries(path)
    }

    /// Return a virtual filesystem entry for the given path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_entry");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.entry(&file).unwrap().is_file());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn entry<T: AsRef<Path>>(&self, path: T) -> RvResult<VfsEntry> {
        Stdfs::entry(path)
    }

    /// Returns true if the `path` exists
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_exists");
    /// assert_vfs_exists!(vfs, &tmpdir);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// assert_vfs_no_exists!(vfs, &tmpdir);
    /// ```
    fn exists<T: AsRef<Path>>(&self, path: T) -> bool {
        Stdfs::exists(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_files");
    /// let dir = tmpdir.mash("dir");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file2);
    /// assert_iter_eq(vfs.files(&tmpdir).unwrap(), vec![file1, file2]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn files<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>> {
        Stdfs::files(path)
    }

    /// Returns the group ID of the owner of this file
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::stdfs();
    /// assert_eq!(Stdfs::gid(vfs.root()).unwrap(), 0);
    /// ```
    fn gid<T: AsRef<Path>>(&self, path: T) -> RvResult<u32> {
        Stdfs::gid(path)
    }

    /// Returns true if the given path exists and is readonly
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_is_exec");
    /// let file = tmpdir.mash("file");
    /// assert!(Stdfs::mkfile_m(&file, 0o644).is_ok());
    /// assert_eq!(Stdfs::is_exec(&file), false);
    /// assert!(Stdfs::chmod(&file, 0o777).is_ok());
    /// assert_eq!(Stdfs::is_exec(&file), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn is_exec<T: AsRef<Path>>(&self, path: T) -> bool {
        Stdfs::is_exec(path)
    }

    /// Returns true if the given path exists and is a directory
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides link exclusion i.e. links even if pointing to a directory return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_is_dir");
    /// assert_vfs_is_dir!(vfs, &tmpdir);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// assert_vfs_no_dir!(vfs, &tmpdir);
    /// ```
    fn is_dir<T: AsRef<Path>>(&self, path: T) -> bool {
        Stdfs::is_dir(path)
    }

    /// Returns true if the given path exists and is a file
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides link exclusion i.e. links even if pointing to a file return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_is_file");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_no_file!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_is_file!(vfs, &file1);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn is_file<T: AsRef<Path>>(&self, path: T) -> bool {
        Stdfs::is_file(path)
    }

    /// Returns true if the given path exists and is readonly
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Example
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_is_readonly");
    /// let file = tmpdir.mash("file1");
    /// assert!(vfs.mkfile_m(&file, 0o644).is_ok());
    /// assert_eq!(vfs.is_readonly(&file), false);
    /// assert!(vfs.chmod_b(&file).unwrap().readonly().exec().is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100444);
    /// assert_eq!(vfs.is_readonly(&file), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn is_readonly<T: AsRef<Path>>(&self, path: T) -> bool {
        Stdfs::is_readonly(path)
    }

    /// Returns true if the given path exists and is a symlink
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Checks the path itself and not what is potentially pointed to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_is_symlink");
    /// let file = tmpdir.mash("file");
    /// let link = tmpdir.mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.is_symlink(&link), false);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// assert_eq!(vfs.is_symlink(&link), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn is_symlink<T: AsRef<Path>>(&self, path: T) -> bool {
        Stdfs::is_symlink(path)
    }

    /// Returns true if the given path exists and is a symlink pointing to a directory
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Checks the path itself and what it points to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_is_symlink_dir");
    /// let dir = tmpdir.mash("dir");
    /// let file = tmpdir.mash("file");
    /// let link1 = tmpdir.mash("link1");
    /// let link2 = tmpdir.mash("link2");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link1, &dir);
    /// assert_vfs_symlink!(vfs, &link2, &file);
    /// assert_eq!(vfs.is_symlink_dir(&link1), true);
    /// assert_eq!(vfs.is_symlink_dir(&link2), false);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn is_symlink_dir<T: AsRef<Path>>(&self, path: T) -> bool {
        Stdfs::is_symlink_dir(path)
    }

    /// Returns true if the given path exists and is a symlink pointing to a file
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Checks the path itself and what it points to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_is_symlink_file");
    /// let dir = tmpdir.mash("dir");
    /// let file = tmpdir.mash("file");
    /// let link1 = tmpdir.mash("link1");
    /// let link2 = tmpdir.mash("link2");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link1, &dir);
    /// assert_vfs_symlink!(vfs, &link2, &file);
    /// assert_eq!(vfs.is_symlink_file(&link1), false);
    /// assert_eq!(vfs.is_symlink_file(&link2), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn is_symlink_file<T: AsRef<Path>>(&self, path: T) -> bool {
        Stdfs::is_symlink_file(path)
    }

    /// Creates the given directory and any parent directories needed with the given mode
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_mkdir_m");
    /// let dir1 = tmpdir.mash("dir1");
    /// assert!(vfs.mkdir_m(&dir1, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&dir1).unwrap(), 0o40555);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn mkdir_m<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<PathBuf> {
        Stdfs::mkdir_m(path, mode)
    }

    /// Creates the given directory and any parent directories needed
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * io::Error if its unable to create the directory
    /// * PathError::IsNotDir(PathBuf) when the path already exists and is not a directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_mkdir_p");
    /// let dir1 = tmpdir.mash("dir1");
    /// assert_vfs_no_dir!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_is_dir!(vfs, &dir1);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn mkdir_p<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf> {
        Stdfs::mkdir_p(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_mkfile");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_no_file!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_is_file!(vfs, &file1);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn mkfile<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf> {
        Stdfs::mkfile(path)
    }

    /// Wraps `mkfile` allowing for setting the file's mode
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_mkfile_m");
    /// let file1 = tmpdir.mash("file1");
    /// assert!(vfs.mkfile_m(&file1, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file1).unwrap(), 0o100555);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn mkfile_m<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<PathBuf> {
        Stdfs::mkfile_m(path, mode)
    }

    /// Returns the permissions for a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_mode");
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::mkfile_m(&file1, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file1).unwrap(), 0o100555);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn mode<T: AsRef<Path>>(&self, path: T) -> RvResult<u32> {
        Stdfs::mode(path)
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
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_move_p");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert!(Stdfs::move_p(&file1, &file2).is_ok());
    /// assert_vfs_no_exists!(vfs, &file1);
    /// assert_vfs_exists!(vfs, &file2);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn move_p<T: AsRef<Path>, U: AsRef<Path>>(&self, src: T, dst: U) -> RvResult<()> {
        Stdfs::move_p(src, dst)
    }

    /// Returns the (user ID, group ID) of the owner of this file
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::stdfs();
    /// assert_eq!(vfs.owner(vfs.root()).unwrap(), (0, 0));
    /// ```
    fn owner<T: AsRef<Path>>(&self, path: T) -> RvResult<(u32, u32)> {
        Stdfs::owner(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_paths");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_iter_eq(vfs.paths(&tmpdir).unwrap(), vec![dir1, dir2, file1]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn paths<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>> {
        Stdfs::paths(path)
    }

    /// Open a file in readonly mode
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_read");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// let mut file = Stdfs::read(&file).unwrap();
    /// let mut buf = String::new();
    /// file.read_to_string(&mut buf);
    /// assert_eq!(buf, "foobar 1".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn read<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn ReadSeek>> {
        Stdfs::read(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_read_all");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_write_all!(vfs, &file1, "this is a test");
    /// assert_eq!(vfs.read_all(&file1).unwrap(), "this is a test");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn read_all<T: AsRef<Path>>(&self, path: T) -> RvResult<String> {
        Stdfs::read_all(path)
    }

    /// Read the given file and returns it as lines in a vector
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_read_lines");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_write_all!(vfs, &file, "1\n2");
    /// assert_eq!(vfs.read_lines(&file).unwrap(), vec!["1".to_string(), "2".to_string()]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn read_lines<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<String>> {
        Stdfs::read_lines(path)
    }

    /// Returns the relative path of the target the link points to
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_readlink");
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_symlink!(vfs, &link1, &file1);
    /// assert_eq!(vfs.readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn readlink<T: AsRef<Path>>(&self, link: T) -> RvResult<PathBuf> {
        Stdfs::readlink(link)
    }

    /// Returns the absolute path of the target the link points to
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_readlink_abs");
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_symlink!(vfs, &link1, &file1);
    /// assert_eq!(vfs.readlink_abs(&link1).unwrap(), file1);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn readlink_abs<T: AsRef<Path>>(&self, link: T) -> RvResult<PathBuf> {
        Stdfs::readlink_abs(link)
    }

    /// Removes the given empty directory or file
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides link exclusion i.e. removes the link themselves not what its points to
    ///
    /// ### Errors
    /// * a directory containing files will trigger an error. use `remove_all` instead
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_remove");
    /// assert_vfs_is_dir!(vfs, &tmpdir);
    /// assert_vfs_remove!(vfs, &tmpdir);
    /// assert_vfs_no_dir!(vfs, &tmpdir);
    /// ```
    fn remove<T: AsRef<Path>>(&self, path: T) -> RvResult<()> {
        Stdfs::remove(path)
    }

    /// Removes the given directory after removing all of its contents
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides link exclusion i.e. removes the link themselves not what its points to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_remove_all");
    /// assert_vfs_is_dir!(vfs, &tmpdir);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// assert_vfs_no_dir!(vfs, &tmpdir);
    /// ```
    fn remove_all<T: AsRef<Path>>(&self, path: T) -> RvResult<()> {
        Stdfs::remove_all(path)
    }

    /// Returns the current root directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn root(&self) -> PathBuf {
        Stdfs::root()
    }

    /// Set the current working directory
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Relative path will use the current working directory
    ///
    /// ### Errors
    /// * io::Error, kind: NotFound when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let stdfs = Stdfs::new();
    /// stdfs.set_cwd(stdfs.cwd().unwrap().mash("tests"));
    /// assert_eq!(stdfs.cwd().unwrap().base().unwrap(), "tests".to_string());
    /// ```
    fn set_cwd<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf> {
        Stdfs::set_cwd(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_symlink");
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_eq!(&vfs.symlink(&link1, &file1).unwrap(), &link1);
    /// assert_vfs_readlink!(vfs, &link1, PathBuf::from("file1"));
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn symlink<T: AsRef<Path>, U: AsRef<Path>>(&self, link: T, target: U) -> RvResult<PathBuf> {
        Stdfs::symlink(link, target)
    }

    /// Returns the user ID of the owner of this file
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::stdfs();
    /// assert_eq!(Stdfs::uid(vfs.root()).unwrap(), 0);
    /// ```
    fn uid<T: AsRef<Path>>(&self, path: T) -> RvResult<u32> {
        Stdfs::uid(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_create");
    /// let file = tmpdir.mash("file");
    /// let mut f = vfs.write(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn write<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>> {
        Stdfs::write(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_write_all");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, "foobar 1");
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()> {
        Stdfs::write_all(path, data)
    }

    /// Write the given lines to to the target file including final newline
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_write_lines");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert!(vfs.write_lines(&file, &["1", "2"]).is_ok());
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "1\n2\n".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn write_lines<T: AsRef<Path>, U: AsRef<str>>(&self, path: T, lines: &[U]) -> RvResult<()> {
        Stdfs::write_lines(path, lines)
    }

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Stdfs::new().upcast();
    /// ```
    fn upcast(self) -> Vfs {
        Vfs::Stdfs(self)
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_stdfs_abs() {
        let cwd = Stdfs::cwd().unwrap();
        let prev = cwd.dir().unwrap();

        // expand relative directory
        assert_eq!(Stdfs::abs("foo").unwrap(), cwd.mash("foo"));

        // expand previous directory and drop trailing slashes
        assert_eq!(Stdfs::abs("..//").unwrap(), prev);
        assert_eq!(Stdfs::abs("../").unwrap(), prev);
        assert_eq!(Stdfs::abs("..").unwrap(), prev);

        // expand current directory and drop trailing slashes
        assert_eq!(Stdfs::abs(".//").unwrap(), cwd);
        assert_eq!(Stdfs::abs("./").unwrap(), cwd);
        assert_eq!(Stdfs::abs(".").unwrap(), cwd);

        // home dir
        let home = PathBuf::from(sys::home_dir().unwrap());
        assert_eq!(Stdfs::abs("~").unwrap(), home);
        assert_eq!(Stdfs::abs("~/").unwrap(), home);

        // expand home path
        assert_eq!(Stdfs::abs("~/foo").unwrap(), home.mash("foo"));

        // More complicated
        assert_eq!(Stdfs::abs("~/foo/bar/../.").unwrap(), home.mash("foo"));
        assert_eq!(Stdfs::abs("~/foo/bar/../").unwrap(), home.mash("foo"));
        assert_eq!(Stdfs::abs("~/foo/bar/../blah").unwrap(), home.mash("foo/blah"));

        // Move up the path multiple levels
        assert_eq!(Stdfs::abs("/foo/bar/blah/../../foo1").unwrap(), PathBuf::from("/foo/foo1"));
        assert_eq!(Stdfs::abs("/../../foo").unwrap(), PathBuf::from("/foo"));

        // Move up until invalid
        assert_eq!(
            Stdfs::abs("../../../../../../../foo").unwrap_err().to_string(),
            PathError::ParentNotFound(PathBuf::from("/")).to_string()
        );

        // absolute path doesn't exist
        assert_eq!(Stdfs::abs("").unwrap_err().to_string(), PathError::Empty.to_string());
    }

    #[test]
    fn test_stdfs_all_dirs() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir1 = tmpdir.mash("dir1");
        let dir2 = dir1.mash("dir2");
        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_iter_eq(vfs.all_dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_all_files() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file1 = tmpdir.mash("file1");
        let dir1 = tmpdir.mash("dir1");
        let file2 = dir1.mash("file2");

        // abs error
        assert_eq!(vfs.paths("").unwrap_err().to_string(), PathError::is_not_dir("").to_string());

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_iter_eq(vfs.all_files(&tmpdir).unwrap(), vec![file2, file1]);
        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_all_paths() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir1 = tmpdir.mash("dir1");
        let dir2 = dir1.mash("dir2");

        // abs error
        assert_eq!(vfs.paths("").unwrap_err().to_string(), PathError::is_not_dir("").to_string());

        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_iter_eq(vfs.all_dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_append() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        if let Err(e) = vfs.append("") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        // Append to a new file and check the data wrote to it
        let mut f = vfs.append(&file).unwrap();
        f.write_all(b"foobar").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "foobar".to_string());
        f.write_all(b"123").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "foobar123".to_string());

        // Append to the file in another trasaction
        let mut f = vfs.append(&file).unwrap();
        f.write_all(b" this is a test").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "foobar123 this is a test".to_string());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_append_all() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        if let Err(e) = vfs.append_all("", "") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        // Append to a new file
        assert!(vfs.append_all(&file, "foobar 1").is_ok());
        assert_vfs_read_all!(vfs, &file, "foobar 1");

        // Append again
        assert!(vfs.append_all(&file, "foobar 2").is_ok());
        assert_vfs_read_all!(vfs, &file, "foobar 1foobar 2");

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_append_line() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        if let Err(e) = vfs.append_line("", "") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        // Append to a new file
        assert!(vfs.append_line(&file, "foobar 1").is_ok());
        assert_vfs_read_all!(vfs, &file, "foobar 1\n");

        // Append again
        assert!(vfs.append_line(&file, "foobar 2").is_ok());
        assert_vfs_read_all!(vfs, &file, "foobar 1\nfoobar 2\n");

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_append_lines() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        if let Err(e) = vfs.append_lines("", &[""]) {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        // Append to a new file
        assert!(vfs.append_lines(&file, &["1", "2"]).is_ok());
        assert_vfs_read_all!(vfs, &file, "1\n2\n");

        // Append again
        assert!(vfs.append_lines(&file, &["3"]).is_ok());
        assert_vfs_read_all!(vfs, &file, "1\n2\n3\n");

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_chmod() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        if let Err(e) = vfs.chmod("", 0) {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        assert_vfs_mkfile!(vfs, &file);
        assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
        assert!(vfs.chmod(&file, 0o555).is_ok());
        assert_eq!(vfs.mode(&file).unwrap(), 0o100555);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_chmod_b() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir = tmpdir.mash("dir");
        let file = dir.mash("file");

        // abs fails
        if let Err(e) = vfs.chmod_b("") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        assert_vfs_mkdir_p!(vfs, &dir);
        assert_vfs_mkfile!(vfs, &file);
        assert_eq!(vfs.mode(&dir).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
        assert!(vfs.chmod_b(&dir).unwrap().recurse().all(0o777).exec().is_ok());
        assert_eq!(vfs.mode(&dir).unwrap(), 0o40777);
        assert_eq!(vfs.mode(&file).unwrap(), 0o100777);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_copy() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file1 = tmpdir.mash("file1");
        let dir1 = tmpdir.mash("dir1");
        let dir1file2 = dir1.mash("file2");

        // Copy file to a dir that doesn't exist with new name
        assert_vfs_mkfile!(vfs, &file1);
        assert!(vfs.copy(&file1, &dir1file2).is_ok());
        assert_vfs_exists!(vfs, &dir1);
        assert_vfs_exists!(vfs, &dir1file2);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_dirs() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir1 = tmpdir.mash("dir1");
        let dir2 = tmpdir.mash("dir2");
        let file1 = tmpdir.mash("file1");

        // abs error
        assert_eq!(Stdfs::dirs("").unwrap_err().to_string(), PathError::is_not_dir("").to_string());

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_vfs_mkfile!(vfs, &file1);
        assert_iter_eq(Stdfs::dirs(&tmpdir).unwrap(), vec![dir1, dir2]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_entries() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file1 = tmpdir.mash("file1");

        // abs error
        assert_eq!(vfs.entries("").unwrap_err().to_string(), PathError::Empty.to_string());

        assert_eq!(&vfs.mkfile(&file1).unwrap(), &file1);
        let mut iter = vfs.entries(&file1).unwrap().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert!(iter.next().is_none());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_entry() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs error
        assert_eq!(vfs.entry("").unwrap_err().to_string(), PathError::Empty.to_string());

        assert_vfs_mkfile!(vfs, &file);
        assert!(vfs.entry(&file).unwrap().is_file());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_entry_iter() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file1 = tmpdir.mash("file1");
        assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
        let mut iter = Stdfs::entry_iter(&tmpdir, false).unwrap();
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert!(iter.next().is_none());
        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_exists() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        assert_eq!(vfs.exists(""), false);

        // Doesn't exist
        assert_eq!(vfs.exists(&file), false);

        assert_vfs_no_exists!(vfs, &file);
        assert_vfs_mkfile!(vfs, &file);
        assert_vfs_exists!(vfs, &file);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_files() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir1 = tmpdir.mash("dir1");
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");

        // abs error
        assert_eq!(Stdfs::files("").unwrap_err().to_string(), PathError::is_not_dir("").to_string());

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_iter_eq(Stdfs::files(&tmpdir).unwrap(), vec![file1, file2]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_is_exec() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        assert_eq!(vfs.is_exec(""), false);

        assert!(vfs.mkfile_m(&file, 0o644).is_ok());
        assert_eq!(vfs.is_exec(&file), false);
        assert!(vfs.chmod(&file, 0o777).is_ok());
        assert_eq!(vfs.is_exec(&file), true);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_is_dir() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir = tmpdir.mash("dir");

        // abs fails
        assert_eq!(vfs.is_dir(""), false);

        // Doesn't exist
        assert_eq!(vfs.is_dir(&dir), false);

        assert_vfs_no_dir!(vfs, &dir);
        assert_vfs_mkdir_p!(vfs, &dir);
        assert_vfs_is_dir!(vfs, &dir);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_is_file() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        assert_eq!(vfs.is_file(""), false);

        // Doesn't exist
        assert_eq!(vfs.is_file(&file), false);

        assert_vfs_no_file!(vfs, &file);
        assert_vfs_mkfile!(vfs, &file);
        assert_vfs_is_file!(vfs, &file);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_is_readonly() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        assert_eq!(vfs.is_readonly(""), false);

        assert!(vfs.mkfile_m(&file, 0o644).is_ok());
        assert_eq!(vfs.is_readonly(&file), false);
        assert!(vfs.chmod_b(&file).unwrap().readonly().exec().is_ok());
        assert_eq!(vfs.mode(&file).unwrap(), 0o100444);
        assert_eq!(vfs.is_readonly(&file), true);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_is_symlink() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");
        let link = tmpdir.mash("link");

        // abs fails
        assert_eq!(vfs.is_symlink(""), false);

        // Doesn't exist
        assert_eq!(vfs.is_symlink(&file), false);

        // Exists
        assert_vfs_mkfile!(vfs, &file);
        assert_vfs_symlink!(vfs, &link, &file);
        assert_vfs_is_symlink!(vfs, &link);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_mkdir_m() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir = tmpdir.mash("dir");

        // abs error
        assert_eq!(vfs.mkdir_m("", 0).unwrap_err().to_string(), PathError::Empty.to_string());

        assert!(vfs.mkdir_m(&dir, 0o555).is_ok());
        assert_eq!(vfs.mode(&dir).unwrap(), 0o40555);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_mkdir_p() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir = tmpdir.mash("dir");

        // Check single top level
        assert_vfs_no_exists!(vfs, &dir);
        assert_vfs_mkdir_p!(vfs, &dir);
        assert_vfs_exists!(vfs, &dir);
        assert_vfs_exists!(vfs, &dir);

        // Check nested
        let dir1 = tmpdir.mash("dir1");
        let dir2 = dir1.mash("dir2");
        let dir3 = dir2.mash("dir3");
        assert_vfs_mkdir_p!(vfs, &dir3);
        assert_vfs_exists!(vfs, &dir3);
        assert_vfs_exists!(vfs, &dir2);
        assert_vfs_exists!(vfs, &dir1);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_mkfile() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");

        // abs error
        assert_eq!(vfs.mkfile("").unwrap_err().to_string(), PathError::Empty.to_string());

        // parent directory doesn't exist
        assert_eq!(vfs.mkfile(&file1).unwrap_err().to_string(), PathError::does_not_exist(&dir1).to_string());

        // Error: target exists and is not a file
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_eq!(vfs.mkfile(&dir1).unwrap_err().to_string(), PathError::is_not_file(&dir1).to_string());

        // Make a file in the root
        let file2 = tmpdir.mash("file2");
        assert_vfs_no_exists!(vfs, &file2);
        assert_vfs_mkfile!(vfs, &file2);
        assert_vfs_exists!(vfs, &file2);

        // Make a file in a directory
        assert_vfs_no_exists!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_exists!(vfs, &file1);

        // Error: parent exists and is not a directory
        let file3 = file1.mash("file3");
        assert_eq!(vfs.mkfile(&file3).unwrap_err().to_string(), PathError::is_not_dir(&file1).to_string());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_mkfile_m() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs error
        assert_eq!(vfs.mkfile_m("", 0).unwrap_err().to_string(), PathError::Empty.to_string());

        assert!(vfs.mkfile_m(&file, 0o555).is_ok());
        assert_eq!(vfs.mode(&file).unwrap(), 0o100555);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_mode() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs error
        assert_eq!(vfs.mode("").unwrap_err().to_string(), PathError::Empty.to_string());

        assert_vfs_mkfile!(vfs, &file);
        assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
        assert!(vfs.chmod(&file, 0o555).is_ok());
        assert_eq!(vfs.mode(&file).unwrap(), 0o100555);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_paths() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir1 = tmpdir.mash("dir1");
        let dir2 = tmpdir.mash("dir2");
        let file1 = tmpdir.mash("file1");

        // abs error
        assert_eq!(vfs.paths("").unwrap_err().to_string(), PathError::is_not_dir("").to_string());

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_vfs_mkfile!(vfs, &file1);
        assert_iter_eq(Stdfs::paths(&tmpdir).unwrap(), vec![dir1, dir2, file1]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_read() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        if let Err(e) = vfs.read("") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        assert_vfs_write_all!(vfs, &file, b"foobar 1");
        let mut file = vfs.read(&file).unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "foobar 1".to_string());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_read_all() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // Doesn't exist error
        assert_eq!(vfs.read_all(&file).unwrap_err().to_string(), PathError::does_not_exist(&file).to_string());

        // Create the file with the given data
        assert_vfs_write_all!(vfs, &file, b"foobar 1");
        assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());

        // Read a second time
        assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_read_lines() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // Doesn't exist error
        assert_eq!(vfs.read_lines(&file).unwrap_err().to_string(), PathError::does_not_exist(&file).to_string());

        // Create the file with the given data
        assert_vfs_write_all!(vfs, &file, "1\n2");
        assert_eq!(vfs.read_lines(&file).unwrap(), vec!["1".to_string(), "2".to_string()]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_readlink() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");
        let link = tmpdir.mash("link");

        // Doesn't exist error
        assert_eq!(vfs.readlink("").unwrap_err().to_string(), PathError::Empty.to_string());

        assert_vfs_mkfile!(vfs, &file);
        assert_vfs_symlink!(vfs, &link, &file);
        assert_vfs_readlink!(vfs, &link, PathBuf::from("file"));

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_readlink_abs() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");
        let link = tmpdir.mash("link");

        // Doesn't exist error
        assert_eq!(vfs.readlink_abs("").unwrap_err().to_string(), PathError::Empty.to_string());

        assert_vfs_mkfile!(vfs, &file);
        assert_vfs_symlink!(vfs, &link, &file);
        assert_vfs_readlink_abs!(vfs, &link, &file);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_remove() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let file2 = tmpdir.mash("file2");

        // abs error
        assert_eq!(vfs.remove("").unwrap_err().to_string(), PathError::Empty.to_string());

        // Single file
        assert_vfs_mkfile!(vfs, &file2);
        assert_vfs_is_file!(vfs, &file2);
        assert_vfs_remove!(vfs, &file2);
        assert_vfs_no_file!(vfs, &file2);

        // Directory with files
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_eq!(vfs.remove(&dir1).unwrap_err().to_string(), PathError::dir_contains_files(&dir1).to_string());
        assert_vfs_remove!(vfs, &file1);
        assert_vfs_remove!(vfs, &dir1);
        assert_vfs_no_exists!(vfs, &dir1);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_remove_all() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir = tmpdir.mash("dir");
        let file = dir.mash("file");

        assert_vfs_mkdir_p!(vfs, &dir);
        assert_vfs_mkfile!(vfs, &file);
        assert_vfs_is_file!(vfs, &file);
        assert_vfs_remove_all!(vfs, &dir);
        assert_vfs_no_exists!(vfs, &file);
        assert_vfs_no_exists!(vfs, &dir);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_symlink() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let link1 = tmpdir.mash("link1");
        let link2 = tmpdir.mash("link2");

        // Create link to nothing
        assert_eq!(&vfs.symlink(&link2, &file1).unwrap(), &link2);

        // Link to dir
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_symlink!(vfs, &link1, &dir1);
        assert_eq!(vfs.is_symlink_dir(&link1), true);
        assert_eq!(vfs.is_symlink_file(&link1), false);

        // Link to file
        assert_vfs_mkfile!(vfs, &file1);
        assert_eq!(vfs.is_symlink_dir(&link2), false);
        assert_eq!(vfs.is_symlink_file(&link2), true);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_write() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        if let Err(e) = vfs.write("") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        // Create a new file and check the data wrote to it
        let mut f = vfs.write(&file).unwrap();
        f.write_all(b"foobar").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "foobar".to_string());

        // Overwrite the file
        let mut f = vfs.write(&file).unwrap();
        f.write_all(b"this is a test").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "this is a test".to_string());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_write_all() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir = tmpdir.mash("dir");
        let file = dir.mash("file");

        // fail abs
        assert_eq!(vfs.write_all("", "").unwrap_err().to_string(), PathError::Empty.to_string());

        // parent doesn't exist
        assert_eq!(vfs.write_all(&file, "").unwrap_err().to_string(), PathError::does_not_exist(&dir).to_string());

        // exists but not a file
        assert_vfs_mkdir_p!(vfs, &dir);
        assert_eq!(vfs.write_all(&dir, "").unwrap_err().to_string(), PathError::is_not_file(&dir).to_string());

        // happy path
        assert!(vfs.write_all(&file, b"foobar 1").is_ok());
        assert_vfs_is_file!(vfs, &file);
        assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_write_lines() {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir = tmpdir.mash("dir");
        let file = dir.mash("file");

        // fail abs
        assert_eq!(vfs.write_lines("", &["foo"]).unwrap_err().to_string(), PathError::Empty.to_string());

        // parent doesn't exist
        assert_eq!(
            vfs.write_lines(&file, &["foo"]).unwrap_err().to_string(),
            PathError::does_not_exist(&dir).to_string()
        );

        // exists but not a file
        assert_vfs_mkdir_p!(vfs, &dir);
        assert_eq!(
            vfs.write_lines(&dir, &["foo"]).unwrap_err().to_string(),
            PathError::is_not_file(&dir).to_string()
        );

        // happy path
        assert!(vfs.write_lines(&file, &["1", "2"]).is_ok());
        assert_vfs_is_file!(vfs, &file);
        assert_vfs_read_all!(vfs, &file, "1\n2\n".to_string());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }
}

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::{self, fs::PermissionsExt},
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use nix::sys::{
    stat::{self, UtimensatFlags},
    time::TimeSpec,
};

use super::StdfsEntryIter;
use crate::{
    errors::*,
    exts::*,
    sys::{self, Entries, Entry, EntryIter, FileSystem, PathExt, ReadSeek, StdfsEntry, Vfs},
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
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(Stdfs::abs("~").unwrap(), PathBuf::from(&home));
    /// ```
    pub fn abs<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
    {
        let path = path.as_ref();

        // Check for empty string
        if sys::is_empty(path) {
            return Err(PathError::Empty.into());
        }

        // Expand home directory
        let mut path_buf = sys::expand(path)?;

        // Trim protocol prefix if needed
        path_buf = sys::trim_protocol(path_buf);

        // Clean the resulting path
        path_buf = sys::clean(path_buf);

        // Expand relative directories if needed
        if !path_buf.is_absolute() {
            let mut curr = Stdfs::cwd()?;
            while let Ok(path) = path_buf.components().first_result() {
                match path {
                    Component::CurDir => {
                        path_buf = sys::trim_first(path_buf);
                    },
                    Component::ParentDir => {
                        if curr.to_string()? == "/" {
                            return Err(PathError::ParentNotFound(curr).into());
                        }
                        curr = sys::dir(curr)?;
                        path_buf = sys::trim_first(path_buf);
                    },
                    _ => return Ok(sys::mash(curr, path_buf)),
                };
            }
            return Ok(curr);
        }

        Ok(path_buf)
    }

    /// Opens a file in append mode
    ///
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_append");
    /// let file = tmpdir.mash("file");
    /// let mut f = Stdfs::create(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// let mut f = Stdfs::append(&file).unwrap();
    /// f.write_all(b"123").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar123".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn append<T: AsRef<Path>>(path: T) -> RvResult<Box<dyn Write>>
    {
        // Ensure the file exists as the std functions don't do that
        Stdfs::mkfile(&path)?;

        Ok(Box::new(OpenOptions::new().write(true).append(true).open(Stdfs::abs(path)?)?))
    }

    /// Set the given mode for the `Path`
    ///
    /// ### Examples
    /// ```ignore
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_chmod");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert!(file1.chmod(0o644).is_ok());
    /// assert_eq!(file1.mode().unwrap(), 0o100644);
    /// assert!(file1.chmod(0o555).is_ok());
    /// assert_eq!(file1.mode().unwrap(), 0o100555);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn chmod<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<()>
    {
        fs::set_permissions(path.as_ref(), fs::Permissions::from_mode(mode))?;
        Ok(())
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_create");
    /// let file = tmpdir.mash("file");
    /// let mut f = Stdfs::create(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn create<T: AsRef<Path>>(path: T) -> RvResult<Box<dyn Write>>
    {
        Ok(Box::new(File::create(Stdfs::abs(path)?)?))
    }

    /// Returns the current working directory
    ///
    /// ### Errors
    /// * Current directory does not exist.
    /// * There are insufficient permissions to access the current directory.
    ///
    /// ### Examples
    /// ```ignore
    /// use rivia::prelude::*;
    ///
    /// Stdfs::set_cwd(Stdfs::cwd().unwrap().mash("tests"));
    /// assert_eq!(Stdfs::cwd().unwrap().base().unwrap(), "tests".to_string());
    /// ```
    pub fn cwd() -> RvResult<PathBuf>
    {
        let path = std::env::current_dir()?;
        Ok(path)
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
    /// ```ignore
    /// use rivia::prelude::*;
    ///
    /// Stdfs::set_cwd(Stdfs::cwd().unwrap().mash("tests"));
    /// assert_eq!(Stdfs::cwd().unwrap().base().unwrap(), "tests".to_string());
    /// ```
    pub fn set_cwd<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
    {
        let path = Stdfs::abs(path)?;
        std::env::set_current_dir(&path)?;
        Ok(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_entries");
    /// let file1 = tmpdir.mash("file1");
    /// assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
    /// let mut iter = Stdfs::entries(&file1).unwrap().into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    /// assert!(iter.next().is_none());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn entries<T: AsRef<Path>>(path: T) -> RvResult<Entries>
    {
        let iter_func = |path: &Path, follow: bool| -> RvResult<EntryIter> {
            Ok(EntryIter {
                path: path.to_path_buf(),
                cached: false,
                following: follow,
                iter: Box::new(StdfsEntryIter {
                    dir: fs::read_dir(path)?,
                }),
            })
        };

        Ok(Entries {
            root: StdfsEntry::from(path)?.upcast(),
            dirs: Default::default(),
            files: Default::default(),
            follow: false,
            min_depth: 0,
            max_depth: std::usize::MAX,
            max_descriptors: sys::DEFAULT_MAX_DESCRIPTORS,
            dirs_first: false,
            files_first: false,
            contents_first: false,
            sort_by_name: false,
            pre_op: None,
            sort: None,
            iter_from: Box::new(iter_func),
        })
    }

    /// Returns true if the `path` exists
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Stdfs::exists("/etc"), true);
    /// ```
    pub fn exists<T: AsRef<Path>>(path: T) -> bool
    {
        match Stdfs::abs(path) {
            Ok(abs) => fs::metadata(abs).is_ok(),
            Err(_) => false,
        }
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_is_dir");
    /// assert_eq!(Stdfs::is_dir(&tmpdir), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn is_dir<T: AsRef<Path>>(path: T) -> bool
    {
        match fs::symlink_metadata(path.as_ref()) {
            Ok(x) => !x.file_type().is_symlink() && x.is_dir(),
            _ => false,
        }
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_is_file");
    /// let file1 = tmpdir.mash("file1");
    /// assert_eq!(Stdfs::is_file(&file1), false);
    /// assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
    /// assert_eq!(Stdfs::is_file(&file1), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn is_file<T: AsRef<Path>>(path: T) -> bool
    {
        match fs::symlink_metadata(path.as_ref()) {
            Ok(x) => !x.file_type().is_symlink() && x.is_file(),
            _ => false,
        }
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
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.is_symlink("foo"), false);
    /// let tmpfile = vfs.symlink("foo", "bar").unwrap();
    /// assert_eq!(vfs.is_symlink(&tmpfile), true);
    /// ```
    pub fn is_symlink<T: AsRef<Path>>(path: T) -> bool
    {
        match fs::symlink_metadata(path.as_ref()) {
            Ok(x) => x.file_type().is_symlink(),
            _ => false,
        }
    }

    /// Wraps `mkdir` allowing for setting the directory's mode.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_mkdir_m");
    /// let dir1 = tmpdir.mash("dir1");
    /// assert!(Stdfs::mkdir_m(&dir1, 0o555).is_ok());
    /// assert_eq!(Stdfs::mode(&dir1).unwrap(), 0o40555);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn mkdir_m<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<PathBuf>
    {
        let path = Stdfs::abs(path)?;

        // For each directory created apply the same permission given
        let path_str = path.to_string()?;
        let mut dir = PathBuf::from("/");
        let mut components = path_str.split('/').rev().collect::<Vec<&str>>();
        while !components.is_empty() {
            dir = dir.mash(components.pop().unwrap());
            if !dir.exists() {
                fs::create_dir(&dir)?;
                fs::set_permissions(&dir, fs::Permissions::from_mode(mode))?;
            }
        }
        Ok(path)
    }

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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_mkdir_p");
    /// let dir1 = tmpdir.mash("dir1");
    /// assert_eq!(Stdfs::exists(&dir1), false);
    /// assert!(Stdfs::mkdir_p(&dir1).is_ok());
    /// assert_eq!(Stdfs::exists(&dir1), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn mkdir_p<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
    {
        let path = Stdfs::abs(path)?;

        // Doesn't error out if it exists
        if !Stdfs::exists(&path) {
            fs::create_dir_all(&path)?;
        } else if !Stdfs::is_dir(&path) {
            return Err(PathError::IsNotDir(path).into());
        }

        Ok(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_mkfile");
    /// let file1 = tmpdir.mash("file1");
    /// assert_eq!(Stdfs::is_file(&file1), false);
    /// assert_eq!(Stdfs::mkfile(&file1).unwrap(), file1);
    /// assert_eq!(Stdfs::is_file(&file1), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn mkfile<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
    {
        let path = Stdfs::abs(path)?;

        // Validate path components
        let dir = path.dir()?;
        if let Ok(meta) = fs::symlink_metadata(&dir) {
            if !meta.is_dir() {
                return Err(PathError::is_not_dir(dir).into());
            }
        } else {
            return Err(PathError::does_not_exist(dir).into());
        }

        // Validate the path itself
        if let Ok(meta) = fs::symlink_metadata(&path) {
            if !meta.is_file() {
                return Err(PathError::is_not_file(path).into());
            }
        } else {
            File::create(&path)?;
        }

        Ok(path)
    }

    /// Returns the permissions for a file
    ///
    /// ### Examples
    /// ```ignore
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_mode");
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::mkfile_m(&file1, 0o555).is_ok());
    /// assert_eq!(Stdfs::mode(&file1).unwrap(), 0o100555);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn mode<T: AsRef<Path>>(path: T) -> RvResult<u32>
    {
        let path = Stdfs::abs(path)?;
        let meta = fs::symlink_metadata(path)?;
        Ok(meta.permissions().mode())
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_open");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// let mut file = Stdfs::open(&file).unwrap();
    /// let mut buf = String::new();
    /// file.read_to_string(&mut buf);
    /// assert_eq!(buf, "foobar 1".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn open<T: AsRef<Path>>(path: T) -> RvResult<Box<dyn ReadSeek>>
    {
        let path = Stdfs::abs(path)?;

        // Validate target exists and is a file
        if Stdfs::exists(&path) {
            if !Stdfs::is_file(&path) {
                return Err(PathError::is_not_file(&path).into());
            }
        } else {
            return Err(PathError::does_not_exist(&path).into());
        }

        // Return the file handle
        Ok(Box::new(File::open(&path)?))
    }

    /// Returns the contents of the `path` as a `String`.
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * PathError::IsNotFile(PathBuf) when the given path isn't a file
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    //
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_read");
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::write_all(&file1, "this is a test").is_ok());
    /// assert_eq!(Stdfs::read_all(&file1).unwrap(), "this is a test");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn read_all<T: AsRef<Path>>(path: T) -> RvResult<String>
    {
        let path = Stdfs::abs(path.as_ref())?;

        // Validate the target file
        if let Ok(meta) = fs::symlink_metadata(&path) {
            if !meta.is_file() {
                return Err(PathError::is_not_file(&path).into());
            }
        } else {
            return Err(PathError::does_not_exist(&path).into());
        }

        match std::fs::read_to_string(path) {
            Ok(data) => Ok(data),
            Err(err) => Err(err.into()),
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_readlink");
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
    /// assert_eq!(&Stdfs::symlink(&link1, &file1).unwrap(), &link1);
    /// assert_eq!(Stdfs::readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn readlink<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
    {
        Ok(fs::read_link(Stdfs::abs(path)?)?)
    }

    /// Returns the absolute path for the given link target. Handles path expansion for
    /// the given link. Useful for determining the absolute path of source relative to the
    /// link rather than cwd.
    //
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_readlink_abs");
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
    /// assert_eq!(&Stdfs::symlink(&link1, &file1).unwrap(), &link1);
    /// assert_eq!(Stdfs::readlink_abs(&link1).unwrap(), file1);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn readlink_abs<T: AsRef<Path>>(link: T) -> RvResult<PathBuf>
    {
        Ok(StdfsEntry::from(link.as_ref())?.alt_buf())
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_remove");
    /// assert!(Stdfs::remove(&tmpdir).is_ok());
    /// assert_eq!(Stdfs::exists(&tmpdir), false);
    /// ```
    pub fn remove<T: AsRef<Path>>(path: T) -> RvResult<()>
    {
        let path = Stdfs::abs(path)?;
        if let Ok(meta) = fs::metadata(&path) {
            if meta.is_file() {
                fs::remove_file(&path)?;
            } else if meta.is_dir() {
                let result = fs::remove_dir(&path);

                // Normalize IO errors
                if result.is_err() {
                    let err = result.unwrap_err();
                    if err.to_string().contains("Directory not empty") {
                        return Err(PathError::dir_contains_files(&path).into());
                    }
                    return Err(err.into());
                }
            }
        }
        Ok(())
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_remove_all");
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
    /// assert_eq!(Stdfs::exists(&tmpdir), false);
    /// ```
    pub fn remove_all<T: AsRef<Path>>(path: T) -> RvResult<()>
    {
        let path = Stdfs::abs(path)?;
        if Stdfs::exists(&path) {
            fs::remove_dir_all(path)?;
        }
        Ok(())
    }

    /// Returns the current root directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn root() -> PathBuf
    {
        let mut root = PathBuf::new();
        root.push(Component::RootDir);
        root
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_symlink");
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
    /// assert_eq!(&Stdfs::symlink(&link1, &file1).unwrap(), &link1);
    /// assert_eq!(Stdfs::readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn symlink<T: AsRef<Path>, U: AsRef<Path>>(link: T, target: U) -> RvResult<PathBuf>
    {
        let target = target.as_ref().to_owned();

        // Ensure link is rooted properly
        let link = Stdfs::abs(link.as_ref())?;

        // If target is not rooted then it is already relative to the link thus mashing the link's directory
        // to the target and cleaning it will given an absolute path.
        let target = Stdfs::abs(if !target.is_absolute() { link.dir()?.mash(target) } else { target })?;

        // Keep the source path relative if possible,
        let target = target.relative(link.dir()?)?;

        unix::fs::symlink(target, &link)?;
        Ok(link)
    }

    /// Set the access and modification times for the given file to the given times
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn set_file_time<T: AsRef<Path>>(path: T, atime: SystemTime, mtime: SystemTime) -> RvResult<()>
    {
        let atime_spec = TimeSpec::from(atime.duration_since(std::time::UNIX_EPOCH)?);
        let mtime_spec = TimeSpec::from(mtime.duration_since(std::time::UNIX_EPOCH)?);
        stat::utimensat(None, path.as_ref(), &atime_spec, &mtime_spec, UtimensatFlags::NoFollowSymlink)?;
        Ok(())
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_read_all");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(path: T, data: U) -> RvResult<()>
    {
        let path = Stdfs::abs(path)?;
        let dir = path.dir()?;

        // Validate the parent directory
        if Stdfs::exists(&dir) {
            if !Stdfs::is_dir(&dir) {
                return Err(PathError::is_not_dir(&dir).into());
            }
        } else {
            return Err(PathError::does_not_exist(&dir).into());
        }

        // Validate the file
        if Stdfs::exists(&path) && !Stdfs::is_file(&path) {
            return Err(PathError::is_not_file(&path).into());
        }

        // Create or truncate the target file
        let mut f = File::create(&path)?;
        f.write_all(data.as_ref())?;

        // f.sync_all() works better than f.flush()?
        f.sync_all()?;
        Ok(())
    }
}

impl FileSystem for Stdfs
{
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
    fn abs<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        Stdfs::abs(path)
    }

    /// Opens a file in append mode
    ///
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
    /// let mut f = vfs.create(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// let mut f = vfs.append(&file).unwrap();
    /// f.write_all(b"123").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar123".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn append<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>>
    {
        Stdfs::append(path)
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
    /// let mut f = vfs.create(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn create<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>>
    {
        Stdfs::create(path)
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
    fn cwd(&self) -> RvResult<PathBuf>
    {
        Stdfs::cwd()
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
    /// assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    /// assert!(iter.next().is_none());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn entries<T: AsRef<Path>>(&self, path: T) -> RvResult<Entries>
    {
        Stdfs::entries(path)
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
    fn exists<T: AsRef<Path>>(&self, path: T) -> bool
    {
        Stdfs::exists(path)
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
    fn is_dir<T: AsRef<Path>>(&self, path: T) -> bool
    {
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
    fn is_file<T: AsRef<Path>>(&self, path: T) -> bool
    {
        Stdfs::is_file(path)
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
    /// let vfs = Vfs::memfs();
    /// assert_eq!(vfs.is_symlink("foo"), false);
    /// let tmpfile = vfs.symlink("foo", "bar").unwrap();
    /// assert_eq!(vfs.is_symlink(&tmpfile), true);
    /// ```
    fn is_symlink<T: AsRef<Path>>(&self, path: T) -> bool
    {
        Stdfs::is_symlink(path)
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
    fn mkfile<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        Stdfs::mkfile(path)
    }

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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_mkdir_p");
    /// let dir1 = tmpdir.mash("dir1");
    /// assert_vfs_no_dir!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_is_dir!(vfs, &dir1);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn mkdir_p<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        Stdfs::mkdir_p(path)
    }

    /// Open a Read + Seek handle to the indicated file
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_open");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// let mut file = Stdfs::open(&file).unwrap();
    /// let mut buf = String::new();
    /// file.read_to_string(&mut buf);
    /// assert_eq!(buf, "foobar 1".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn open<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn ReadSeek>>
    {
        Stdfs::open(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_read");
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::write_all(&file1, "this is a test").is_ok());
    /// assert_eq!(Stdfs::read_all(&file1).unwrap(), "this is a test");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn read_all<T: AsRef<Path>>(&self, path: T) -> RvResult<String>
    {
        Stdfs::read_all(path)
    }

    /// Returns the path the given link points to
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
    /// assert_eq!(&vfs.symlink(&link1, &file1).unwrap(), &link1);
    /// assert_eq!(vfs.readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn readlink<T: AsRef<Path>>(&self, link: T) -> RvResult<PathBuf>
    {
        Stdfs::readlink(link)
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
    fn remove<T: AsRef<Path>>(&self, path: T) -> RvResult<()>
    {
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
    fn remove_all<T: AsRef<Path>>(&self, path: T) -> RvResult<()>
    {
        Stdfs::remove_all(path)
    }

    /// Returns the current root directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn root(&self) -> PathBuf
    {
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
    fn set_cwd<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
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
    /// assert_eq!(vfs.readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn symlink<T: AsRef<Path>, U: AsRef<Path>>(&self, link: T, target: U) -> RvResult<PathBuf>
    {
        Stdfs::symlink(link, target)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_method_read_all");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()>
    {
        Stdfs::write_all(path, data)
    }

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Stdfs::new().upcast();
    /// ```
    fn upcast(self) -> Vfs
    {
        Vfs::Stdfs(self)
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[test]
    fn test_stdfs_abs()
    {
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
    fn test_stdfs_append()
    {
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
    fn test_stdfs_create()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let file = tmpdir.mash("file");

        // abs fails
        if let Err(e) = vfs.create("") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        // Create a new file and check the data wrote to it
        let mut f = vfs.create(&file).unwrap();
        f.write_all(b"foobar").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "foobar".to_string());

        // Overwrite the file
        let mut f = vfs.create(&file).unwrap();
        f.write_all(b"this is a test").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "this is a test".to_string());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_stdfs_exists()
    {
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
    fn test_stdfs_is_dir()
    {
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
    fn test_stdfs_is_file()
    {
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
    fn test_stdfs_is_symlink()
    {
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
    fn test_stdfs_mkdir_p()
    {
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
    fn test_stdfs_mkfile()
    {
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
    fn test_stdfs_read_all()
    {
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
    fn test_stdfs_remove()
    {
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
    fn test_memfs_symlink()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let link1 = tmpdir.mash("link1");
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_symlink!(vfs, &link1, &dir1);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_write_all()
    {
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
}

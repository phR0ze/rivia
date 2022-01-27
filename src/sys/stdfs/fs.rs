use std::{
    fs::{self, File},
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
    sys::{self, Entries, Entry, EntryIter, FileSystem, PathExt, StdfsEntry, Vfs},
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
    /// ### Detail:
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

    /// Set the given mode for the `Path`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_chmod"));
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert!(file1.chmod(0o644).is_ok());
    /// assert_eq!(file1.mode().unwrap(), 0o100644);
    /// assert!(file1.chmod(0o555).is_ok());
    /// assert_eq!(file1.mode().unwrap(), 0o100555);
    /// //assert!(Stdfs::remove_all(&tmpdir).is_ok());
    /// ```
    pub fn chmod<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<()>
    {
        fs::set_permissions(path.as_ref(), fs::Permissions::from_mode(mode))?;
        Ok(())
    }

    // /// Returns the `Path` with the given string concatenated on without injecting
    // /// path separators.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_eq!(Path::new("/foo/bar").concat(".rs").unwrap(), PathBuf::from("/foo/bar.rs"));
    // /// ```
    // pub fn concat<T: AsRef<str>>(&self, val: T) -> RvResult<PathBuf> {
    //     Ok(PathBuf::from(format!("{}{}", self.to_string()?, val.as_ref())))
    // }

    /// Returns the current working directory
    ///
    /// ### Errors
    /// * Current directory does not exist.
    /// * There are insufficient permissions to access the current directory.
    ///
    /// ### Examples
    /// ```
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
    /// ### Detail
    /// * path expansion and absolute path resolution
    /// * relative path will use the current working directory
    ///
    /// ### Errors
    /// * io::Error, kind: NotFound when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
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
    /// ### Detail
    /// * path expansion and absolute path resolution
    /// * recursive path traversal
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_entries"));
    /// let file1 = tmpdir.mash("file1");
    /// assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
    /// let mut iter = Stdfs::entries(&file1).unwrap().into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    /// assert!(iter.next().is_none());
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
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
    /// ### Detail
    /// * path expansion and absolute path resolution
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

    // /// Returns the extension of the path or an error.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_eq!(Path::new("foo.bar").ext().unwrap(), "bar");
    // /// ```
    // pub fn ext(&self) -> RvResult<String> {
    //     match self.extension() {
    //         Some(val) => val.to_string(),
    //         None => Err(PathError::extension_not_found(self).into()),
    //     }
    // }

    // /// Returns the group ID of the owner of this file.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_eq!(Path::new("/etc").gid().unwrap(), 0);
    // /// ```
    // pub fn gid(&self) -> RvResult<u32> {
    //     Stdfs::gid(&self)
    // }

    /// Returns true if the given path exists and is a directory
    ///
    /// ### Detail
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. links even if pointing to a directory return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_is_dir"));
    /// assert_eq!(Stdfs::is_dir(&tmpdir), true);
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
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
    /// ### Detail
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. links even if pointing to a file return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_is_file"));
    /// let file1 = tmpdir.mash("file1");
    /// assert_eq!(Stdfs::is_file(&file1), false);
    /// assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
    /// assert_eq!(Stdfs::is_file(&file1), true);
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
    /// ```
    pub fn is_file<T: AsRef<Path>>(path: T) -> bool
    {
        match fs::symlink_metadata(path.as_ref()) {
            Ok(x) => !x.file_type().is_symlink() && x.is_file(),
            _ => false,
        }
    }

    // /// Returns true if the `Path` exists and is an executable. Handles path expansion.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_stdfs_setup_func!();
    // /// let tmpdir = assert_stdfs_setup!("stdfs_func_is_exec");
    // /// let file1 = tmpdir.mash("file1");
    // /// assert!(Stdfs::mkfile_m(&file1, 0o644).is_ok());
    // /// assert_eq!(file1.is_exec(), false);
    // /// assert!(Stdfs::chmod_b(&file1).unwrap().sym("a:a+x").exec().is_ok());
    // /// assert_eq!(file1.mode().unwrap(), 0o100755);
    // /// assert_eq!(file1.is_exec(), true);
    // /// assert_remove_all!(&tmpdir);
    // /// ```
    // pub fn is_exec(&self) -> bool {
    //     Stdfs::is_exec(self)
    // }

    // /// Returns true if the `Path` exists and is a file. Handles path expansion
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_eq!(Path::new("/etc/hosts").is_file(), true);
    // /// ```
    // pub fn is_file(&self) -> bool {
    //     Stdfs::is_file(self)
    // }

    // /// Returns true if the `Path` exists and is readonly. Handles path expansion.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_stdfs_setup_func!();
    // /// let tmpdir = assert_stdfs_setup!("stdfs_func_is_readonly");
    // /// let file1 = tmpdir.mash("file1");
    // /// assert!(Stdfs::mkfile_m(&file1, 0o644).is_ok());
    // /// assert_eq!(file1.is_readonly(), false);
    // /// assert!(Stdfs::chmod_b(&file1).unwrap().readonly().exec().is_ok());
    // /// assert_eq!(file1.mode().unwrap(), 0o100444);
    // /// assert_eq!(file1.is_readonly(), true);
    // /// assert_remove_all!(&tmpdir);
    // /// ```
    // pub fn is_readonly(&self) -> bool {
    //     Stdfs::is_readonly(self)
    // }

    // /// Returns true if the `Path` exists and is a symlink. Handles path expansion
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_stdfs_setup_func!();
    // /// let tmpdir = assert_stdfs_setup!("stdfs_func_is_symlink");
    // /// let file1 = tmpdir.mash("file1");
    // /// let link1 = tmpdir.mash("link1");
    // /// assert_mkfile!(&file1);
    // /// assert!(Stdfs::symlink(&file1, &link1).is_ok());
    // /// assert_eq!(link1.is_symlink(), true);
    // /// assert_remove_all!(&tmpdir);
    // /// ```
    // pub fn is_symlink(&self) -> bool {
    //     Stdfs::is_symlink(self)
    // }

    // /// Returns true if the `Path` exists and is a symlinked directory. Handles path expansion
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_stdfs_setup_func!();
    // /// let tmpdir = assert_stdfs_setup!("stdfs_func_is_symlink_dir");
    // /// let dir1 = tmpdir.mash("dir1");
    // /// let link1 = tmpdir.mash("link1");
    // /// assert_mkdir!(&dir1);
    // /// assert!(Stdfs::symlink(&dir1, &link1).is_ok());
    // /// assert_eq!(link1.is_symlink_dir(), true);
    // /// assert_remove_all!(&tmpdir);
    // /// ```
    // pub fn is_symlink_dir(&self) -> bool {
    //     Stdfs::is_symlink_dir(self)
    // }

    // /// Returns true if the given `Path` exists and is a symlinked file. Handles path
    // /// expansion
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_stdfs_setup_func!();
    // /// let tmpdir = assert_stdfs_setup!("stdfs_func_is_symlink_file");
    // /// let file1 = tmpdir.mash("file1");
    // /// let link1 = tmpdir.mash("link1");
    // /// assert_mkfile!(&file1);
    // /// assert!(Stdfs::symlink(&file1, &link1).is_ok());
    // /// assert_eq!(link1.is_symlink_file(), true);
    // /// assert_remove_all!(&tmpdir);
    // /// ```
    // pub fn is_symlink_file(&self) -> bool {
    //     Stdfs::is_symlink_file(self)
    // }

    /// Create an empty file similar to the linux touch command
    ///
    /// ### Detail
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
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_mkfile"));
    /// let file1 = tmpdir.mash("file1");
    /// assert_eq!(Stdfs::is_file(&file1), false);
    /// assert_eq!(Stdfs::mkfile(&file1).unwrap(), file1);
    /// assert_eq!(Stdfs::is_file(&file1), true);
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
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

    /// Wraps `mkdir` allowing for setting the directory's mode.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_mkdir_m"));
    /// let dir1 = tmpdir.mash("dir1");
    /// assert!(Stdfs::mkdir_m(&dir1, 0o555).is_ok());
    /// assert_eq!(Stdfs::mode(&dir1).unwrap(), 0o40555);
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
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
    /// ### Detail
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
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_mkdir_p"));
    /// let dir1 = tmpdir.mash("dir1");
    /// assert_eq!(Stdfs::exists(&dir1), false);
    /// assert!(Stdfs::mkdir_p(&dir1).is_ok());
    /// assert_eq!(Stdfs::exists(&dir1), true);
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
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

    /// Returns the permissions for a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_mode"));
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::mkfile_m(&file1, 0o555).is_ok());
    /// assert_eq!(Stdfs::mode(&file1).unwrap(), 0o100555);
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
    /// ```
    pub fn mode<T: AsRef<Path>>(path: T) -> RvResult<u32>
    {
        let path = Stdfs::abs(path)?;
        let meta = fs::symlink_metadata(path)?;
        Ok(meta.permissions().mode())
    }

    /// Returns the contents of the `path` as a `String`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_read"));
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::write(&file1, "this is a test").is_ok());
    /// assert_eq!(Stdfs::read(&file1).unwrap(), "this is a test");
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
    /// ```
    pub fn read_all<T: AsRef<Path>>(path: T) -> RvResult<String>
    {
        let path = Stdfs::abs(path.as_ref())?;
        match std::fs::read_to_string(path) {
            Ok(data) => Ok(data),
            Err(err) => Err(err.into()),
        }
    }

    /// Returns the path the given link points to
    ///
    /// ### Detail
    /// * path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_readlink"));
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
    /// assert_eq!(&Stdfs::symlink(&link1, &file1).unwrap(), &link1);
    /// assert_eq!(Stdfs::readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
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
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_readlink_abs"));
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
    /// assert_eq!(&Stdfs::symlink(&link1, &file1).unwrap(), &link1);
    /// assert_eq!(Stdfs::readlink_abs(&link1).unwrap(), file1);
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
    /// ```
    pub fn readlink_abs<T: AsRef<Path>>(link: T) -> RvResult<PathBuf>
    {
        Ok(StdfsEntry::from(link.as_ref())?.alt_buf())
    }

    /// Returns the `Path` relative to the given `base` path. Think what is the path navigation
    /// required to get from `base` to self. Every path used should represent a directory not a file
    /// or link. For files or links trim off the last segement of the path before calling this
    /// method. No attempt is made by this method to trim off the file segment.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(
    ///     PathBuf::from("foo/bar1").relative("foo/bar2").unwrap(),
    ///     PathBuf::from("../bar1")
    /// );
    /// ```
    pub fn relative<T: AsRef<Path>, U: AsRef<Path>>(path: T, base: U) -> RvResult<PathBuf>
    {
        let path = Stdfs::abs(path)?;
        let base = Stdfs::abs(base)?;
        if path != base {
            let mut x = path.components();
            let mut y = base.components();
            let mut comps: Vec<Component> = vec![];
            loop {
                match (x.next(), y.next()) {
                    // nothing were done
                    (None, None) => break,

                    // base is ahead one
                    (None, _) => comps.push(Component::ParentDir),

                    // self is ahead the remaining
                    (Some(a), None) => {
                        comps.push(a);
                        comps.extend(x.by_ref());
                        break;
                    },

                    // both components are the same and we haven't processed anything yet skip it
                    (Some(a), Some(b)) if comps.is_empty() && a == b => continue,

                    // any additional components in the base need to be backed tracked from self
                    (Some(a), Some(_)) => {
                        // backtrack the current component and all remaining ones
                        comps.push(Component::ParentDir);
                        for _ in y {
                            comps.push(Component::ParentDir);
                        }

                        // now include the current self and all remaining components
                        comps.push(a);
                        comps.extend(x.by_ref());
                        break;
                    },
                }
            }
            return Ok(comps.iter().collect::<PathBuf>());
        }
        Ok(path)
    }

    /// Removes the given empty directory or file
    ///
    /// ### Detail
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
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_remove"));
    /// assert!(Stdfs::remove(&tmpdir).is_ok());
    /// assert_eq!(Stdfs::exists(&tmpdir), false);
    /// ```
    pub fn remove<T: AsRef<Path>>(path: T) -> RvResult<()>
    {
        let path = Stdfs::abs(path)?;
        if let Ok(meta) = fs::metadata(&path) {
            if meta.is_file() {
                fs::remove_file(path)?;
            } else if meta.is_dir() {
                fs::remove_dir(path)?;
            }
        }
        Ok(())
    }

    /// Removes the given directory after removing all of its contents
    ///
    /// ### Detail
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. removes the link themselves not what its points to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_remove_all"));
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

    // /// Set the given [`Mode`] on the `Path` and return the `Path`
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_stdfs_setup_func!();
    // /// let tmpdir = assert_stdfs_setup!("stdfs_func_set_mode");
    // /// let file1 = tmpdir.mash("file1");
    // /// assert_mkfile!(&file1);
    // /// assert!(file1.chmod(0o644).is_ok());
    // /// assert_eq!(file1.mode().unwrap(), 0o100644);
    // /// assert!(file1.set_mode(0o555).is_ok());
    // /// assert_eq!(file1.mode().unwrap(), 0o100555);
    // /// assert_remove_all!(&tmpdir);
    // /// ```
    // pub fn set_mode(&self, mode: u32) -> RvResult<PathBuf> {
    //     Stdfs::set_mode(self, mode)?;
    //     Ok(self.to_path_buf())
    // }

    // /// Returns the shared path prefix between `self` and `Path`. All paths will share root `/`
    // /// so this case is being dropped to simplify detection of shared components.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_eq!(PathBuf::from("bar1").shared_prefix("bar2").unwrap(), Stdfs::cwd().unwrap());
    // /// ```
    // pub fn shared_prefix<T: AsRef<Path>>(&self, base: T) -> RvResult<PathBuf> {
    //     let path = self.abs()?;
    //     let base = base.as_ref().abs()?;
    //     if path != base {
    //         let mut x = path.components();
    //         let mut y = base.components();
    //         let mut comps: Vec<Component> = vec![];
    //         loop {
    //             match (x.next(), y.next()) {
    //                 (Some(a), Some(b)) if a == b => comps.push(a),
    //                 (..) => break,
    //             }
    //         }

    //         // If all that is shared is the root then drop it to help detect this case better
    //         if comps.len() == 1 {
    //             if let Some(x) = comps.first() {
    //                 if x == &Component::RootDir {
    //                     comps.remove(0);
    //                 }
    //             }
    //         }

    //         return Ok(comps.iter().collect::<PathBuf>());
    //     }
    //     Ok(path)
    // }

    /// Creates a new symbolic link
    ///
    /// ### Arguments
    /// * `link` - the path of the link being created
    /// * `target` - the path that the link will point to
    ///
    /// ### Detail:
    /// * path expansion and absolute path resolution
    /// * computes the target path `src` relative to the `dst` link name's absolute path
    /// * returns the link path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_func_symlink"));
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_eq!(&Stdfs::mkfile(&file1).unwrap(), &file1);
    /// assert_eq!(&Stdfs::symlink(&link1, &file1).unwrap(), &link1);
    /// assert_eq!(Stdfs::readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// assert!(Stdfs::remove_all(&tmpdir).is_ok());
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
        let target = Stdfs::relative(target, link.dir()?)?;

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

    // /// Returns the user ID of the owner of this file.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_eq!(Path::new("/etc").uid().unwrap(), 0);
    // /// ```
    // pub fn uid(&self) -> RvResult<u32> {
    //     Stdfs::uid(&self)
    // }

    /// Write the given data to to the indicated file creating the file first if it doesn't exist
    /// or truncating it first if it does. If the path exists an isn't a file an error will be
    /// returned.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(path: T, data: U) -> RvResult<()>
    {
        let path = Stdfs::abs(path)?;
        if Stdfs::exists(&path) && !Stdfs::is_file(&path) {
            return Err(PathError::IsNotFile(path).into());
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
    /// ### Detail:
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
    /// let stdfs = Stdfs::new();
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(stdfs.abs("~").unwrap(), PathBuf::from(&home));
    /// ```
    fn abs<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        Stdfs::abs(path)
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
    /// ### Detail
    /// * path expansion and absolute path resolution
    /// * recursive path traversal
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_method_entries"));
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
    /// ### Detail
    /// * path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_method_exists"));
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
    /// ### Detail
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. links even if pointing to a directory return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_method_is_dir"));
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
    /// ### Detail
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. links even if pointing to a file return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_method_is_file"));
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

    /// Create an empty file similar to the linux touch command
    ///
    /// ### Detail
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
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_method_mkfile"));
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
    /// ### Detail
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
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_method_mkdir_p"));
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

    /// Read all data from the given file and return it as a String
    fn read_all<T: AsRef<Path>>(&self, path: T) -> RvResult<String>
    {
        Stdfs::read_all(path)
    }

    /// Returns the path the given link points to
    ///
    /// ### Detail
    /// * path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_method_readlink"));
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
    /// ### Detail
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
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_method_remove"));
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
    /// ### Detail
    /// * path expansion and absolute path resolution
    /// * link exclusion i.e. removes the link themselves not what its points to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_method_remove_all"));
    /// assert_vfs_is_dir!(vfs, &tmpdir);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// assert_vfs_no_dir!(vfs, &tmpdir);
    /// ```
    fn remove_all<T: AsRef<Path>>(&self, path: T) -> RvResult<()>
    {
        Stdfs::remove_all(path)
    }

    /// Set the current working directory
    ///
    /// ### Detail
    /// * path expansion and absolute path resolution
    /// * relative path will use the current working directory
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
    /// ### Arguments
    /// * `link` - the path of the link being created
    /// * `target` - the path that the link will point to
    ///
    /// ### Detail:
    /// * path expansion and absolute path resolution
    /// * computes the target path `src` relative to the `dst` link name's absolute path
    /// * returns the link path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = testing::vfs_setup_p(Vfs::stdfs(), Some("stdfs_method_symlink"));
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

    /// Write the given data to to the indicated file creating the file first if it doesn't exist
    /// or truncating it first if it does.
    fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()>
    {
        Stdfs::write_all(path, data)
    }

    /// Up cast the trait type to the enum wrapper
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
    fn test_stdfs_abs() -> RvResult<()>
    {
        let cwd = Stdfs::cwd()?;
        let prev = cwd.dir()?;

        // expand relative directory
        assert_eq!(Stdfs::abs("foo")?, cwd.mash("foo"));

        // expand previous directory and drop trailing slashes
        assert_eq!(Stdfs::abs("..//")?, prev);
        assert_eq!(Stdfs::abs("../")?, prev);
        assert_eq!(Stdfs::abs("..")?, prev);

        // expand current directory and drop trailing slashes
        assert_eq!(Stdfs::abs(".//")?, cwd);
        assert_eq!(Stdfs::abs("./")?, cwd);
        assert_eq!(Stdfs::abs(".")?, cwd);

        // home dir
        let home = PathBuf::from(sys::home_dir()?);
        assert_eq!(Stdfs::abs("~")?, home);
        assert_eq!(Stdfs::abs("~/")?, home);

        // expand home path
        assert_eq!(Stdfs::abs("~/foo")?, home.mash("foo"));

        // More complicated
        assert_eq!(Stdfs::abs("~/foo/bar/../.")?, home.mash("foo"));
        assert_eq!(Stdfs::abs("~/foo/bar/../")?, home.mash("foo"));
        assert_eq!(Stdfs::abs("~/foo/bar/../blah")?, home.mash("foo/blah"));

        // Move up the path multiple levels
        assert_eq!(Stdfs::abs("/foo/bar/blah/../../foo1")?, PathBuf::from("/foo/foo1"));
        assert_eq!(Stdfs::abs("/../../foo")?, PathBuf::from("/foo"));

        // Move up until invalid
        assert_eq!(
            Stdfs::abs("../../../../../../../foo").unwrap_err().to_string(),
            PathError::ParentNotFound(PathBuf::from("/")).to_string()
        );
        Ok(())
    }

    // #[test]
    // fn test_stdfs_exists()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let tmpfile = tmpdir.mash("file");
    //     assert_stdfs_remove_all!(&tmpdir);
    //     assert!(Stdfs::mkdir_p(&tmpdir).is_ok());
    //     assert_eq!(Stdfs::exists(&tmpfile), false);
    //     assert!(Stdfs::mkfile(&tmpfile).is_ok());
    //     assert_eq!(Stdfs::exists(&tmpfile), true);
    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_mkfile()
    // {
    //     // Error: directory doesn't exist
    //     let err = memfs.mkfile("dir1/file1").unwrap_err();
    //     assert_eq!(err.downcast_ref::<PathError>().unwrap(),
    // &PathError::does_not_exist("/dir1"));

    //     // Error: target exists and is not a file
    //     memfs.mkdir_p("dir1").unwrap();
    //     let err = memfs.mkfile("dir1").unwrap_err();
    //     assert_eq!(err.downcast_ref::<PathError>().unwrap(), &PathError::is_not_file("/dir1"));

    //     // Make a file in the root
    //     assert_eq!(memfs.exists("file2"), false);
    //     assert!(memfs.mkfile("file2").is_ok());
    //     assert_eq!(memfs.exists("file2"), true);

    //     // Error: parent exists and is not a directory
    //     let err = memfs.mkfile("file2/file1").unwrap_err();
    //     assert_eq!(err.downcast_ref::<PathError>().unwrap(), &PathError::is_not_dir("/file2"));
    // }
    // use crate::prelude::*;
    // assert_stdfs_setup_func!();

    // fn assert_iter_eq(iter: EntriesIter, paths: Vec<&PathBuf>)
    // {
    //     // Using a vector here as there can be duplicates
    //     let mut entries = Vec::new();
    //     for entry in iter {
    //         entries.push(entry.unwrap().path().to_path_buf());
    //     }

    //     assert_eq!(entries.len(), paths.len());
    //     for path in paths.iter() {
    //         assert!(entries.contains(path));
    //     }
    // }

    // #[test]
    // fn test_stdfs_contents_first()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = tmpdir.mash("dir1");
    //     let file1 = dir1.mash("file1");
    //     let dir2 = dir1.mash("dir2");
    //     let file2 = dir2.mash("file2");
    //     let file3 = tmpdir.mash("file3");
    //     let link1 = tmpdir.mash("link1");

    //     assert_stdfs_mkdir_p!(&dir2);
    //     assert_stdfs_mkfile!(&file1);
    //     assert_stdfs_mkfile!(&file2);
    //     assert_stdfs_mkfile!(&file3);
    //     assert_eq!(Stdfs::symlink(&file3, &link1).unwrap(), link1);

    //     // contents first un-sorted
    //     let iter = Stdfs::entries(&tmpdir).unwrap().contents_first().into_iter();
    //     assert_iter_eq(iter, vec![&link1, &file3, &file2, &dir2, &file1, &dir1, &tmpdir]);

    //     // contents first sorted
    //     let mut iter =
    // Stdfs::entries(&tmpdir).unwrap().contents_first().dirs_first().into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file3);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), link1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
    //     assert!(iter.next().is_none());

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_sort()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let zdir1 = tmpdir.mash("zdir1");
    //     let dir1file1 = zdir1.mash("file1");
    //     let dir1file2 = zdir1.mash("file2");
    //     let zdir2 = tmpdir.mash("zdir2");
    //     let dir2file1 = zdir2.mash("file1");
    //     let dir2file2 = zdir2.mash("file2");
    //     let file1 = tmpdir.mash("file1");
    //     let file2 = tmpdir.mash("file2");

    //     assert_stdfs_mkdir_p!(&zdir1);
    //     assert_stdfs_mkdir_p!(&zdir2);
    //     assert_stdfs_mkfile!(&dir1file1);
    //     assert_stdfs_mkfile!(&dir1file2);
    //     assert_stdfs_mkfile!(&dir2file1);
    //     assert_stdfs_mkfile!(&dir2file2);
    //     assert_stdfs_mkfile!(&file1);
    //     assert_stdfs_mkfile!(&file2);

    //     // Without sorting
    //     let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
    //     assert_iter_eq(iter, vec![
    //         &tmpdir, &file2, &zdir1, &dir1file2, &dir1file1, &file1, &zdir2, &dir2file2,
    // &dir2file1,     ]);

    //     // with sorting on name
    //     let mut iter = Stdfs::entries(&tmpdir).unwrap().sort(|x, y|
    // x.file_name().cmp(&y.file_name())).into_iter();     assert_eq!(iter.next().unwrap().
    // unwrap().path(), tmpdir);     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2file2);
    //     assert!(iter.next().is_none());

    //     // with sort default set
    //     let mut iter = Stdfs::entries(&tmpdir).unwrap().sort_by_name().into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2file2);
    //     assert!(iter.next().is_none());

    //     // sort dirs first
    //     let zdir3 = zdir1.mash("zdir3");
    //     let dir3file1 = zdir3.mash("file1");
    //     let dir3file2 = zdir3.mash("file2");
    //     assert_stdfs_mkdir_p!(&zdir3);
    //     assert_stdfs_mkfile!(&dir3file1);
    //     assert_stdfs_mkfile!(&dir3file2);

    //     let mut iter = Stdfs::entries(&tmpdir).unwrap().dirs_first().into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir3);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir3file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir3file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert!(iter.next().is_none());

    //     // sort files first
    //     let mut iter = Stdfs::entries(&tmpdir).unwrap().files_first().into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir3);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir3file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir3file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2file2);
    //     assert!(iter.next().is_none());

    //     // sort files first but in reverse aphabetic order
    //     let mut iter = Stdfs::entries(&tmpdir)
    //         .unwrap()
    //         .files_first()
    //         .sort(|x, y| y.file_name().cmp(&x.file_name()))
    //         .into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), zdir3);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir3file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir3file1);
    //     assert!(iter.next().is_none());

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_max_descriptors()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = tmpdir.mash("dir1");
    //     let file1 = dir1.mash("file1");
    //     let dir2 = dir1.mash("dir2");
    //     let file2 = dir2.mash("file2");
    //     let dir3 = dir2.mash("dir3");
    //     let file3 = dir3.mash("file3");

    //     assert_stdfs_mkdir_p!(&dir3);
    //     assert_stdfs_mkfile!(&file1);
    //     assert_stdfs_mkfile!(&file2);
    //     assert_stdfs_mkfile!(&file3);

    //     // Without descriptor cap
    //     let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
    //     assert_iter_eq(iter, vec![&tmpdir, &dir1, &dir2, &file2, &dir3, &file3, &file1]);

    //     // with descritor cap - should have the same pattern
    //     let mut paths = Stdfs::entries(&tmpdir).unwrap();
    //     paths.max_descriptors = 1;
    //     let iter = paths.into_iter();
    //     assert_iter_eq(iter, vec![&tmpdir, &dir1, &dir2, &file2, &dir3, &file3, &file1]);

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_loop_detection()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = tmpdir.mash("dir1");
    //     let dir2 = dir1.mash("dir2");
    //     let link1 = dir2.mash("link1");

    //     assert_stdfs_mkdir_p!(&dir2);
    //     assert_eq!(Stdfs::symlink(&dir1, &link1).unwrap(), link1);

    //     // Non follow should be fine
    //     let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
    //     assert_iter_eq(iter, vec![&tmpdir, &dir1, &dir2, &link1]);

    //     // Follow link will loop
    //     let mut iter = Stdfs::entries(&tmpdir).unwrap().follow(true).into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2);
    //     assert_eq!(iter.next().unwrap().unwrap_err().to_string(),
    // PathError::link_looping(dir1).to_string());     assert!(iter.next().is_none());

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_filter()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = tmpdir.mash("dir1");
    //     let file1 = dir1.mash("file1");
    //     let dir2 = dir1.mash("dir2");
    //     let file2 = dir2.mash("file2");
    //     let file3 = tmpdir.mash("file3");
    //     let link1 = tmpdir.mash("link1");
    //     let link2 = tmpdir.mash("link2");

    //     assert_stdfs_mkdir_p!(&dir2);
    //     assert_stdfs_mkfile!(&file1);
    //     assert_stdfs_mkfile!(&file2);
    //     assert_stdfs_mkfile!(&file3);
    //     assert_eq!(Stdfs::symlink(&dir2, &link2).unwrap(), link2);
    //     assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);

    //     // Files only
    //     let iter = Stdfs::entries(&tmpdir).unwrap().files().into_iter();
    //     assert_iter_eq(iter, vec![&link1, &file3, &file2, &file1]);

    //     // Dirs only
    //     let iter = Stdfs::entries(&tmpdir).unwrap().dirs().into_iter();
    //     assert_iter_eq(iter, vec![&tmpdir, &link2, &dir1, &dir2]);

    //     // Custom links only
    //     let mut iter = Stdfs::entries(&tmpdir).unwrap().into_iter().filter_p(|x| x.is_symlink());
    //     assert_iter_eq(iter, vec![&link1, &link2]);

    //     // Custom name
    //     let iter = Stdfs::entries(&tmpdir).unwrap().into_iter().filter_p(|x|
    // x.path().has_suffix("1"));     assert_iter_eq(iter, vec![&link1, &dir1, &file1]);

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_follow()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = tmpdir.mash("dir1");
    //     let file1 = dir1.mash("file1");
    //     let dir2 = dir1.mash("dir2");
    //     let file2 = dir2.mash("file2");
    //     let file3 = tmpdir.mash("file3");
    //     let link1 = tmpdir.mash("link1");

    //     assert_stdfs_mkdir_p!(&dir2);
    //     assert_stdfs_mkfile!(&file1);
    //     assert_stdfs_mkfile!(&file2);
    //     assert_stdfs_mkfile!(&file3);
    //     assert_eq!(Stdfs::symlink(&dir2, &link1).unwrap(), link1);

    //     // Follow off
    //     let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
    //     assert_iter_eq(iter, vec![&tmpdir, &link1, &file3, &dir1, &dir2, &file2, &file1]);

    //     // Follow on
    //     let iter = Stdfs::entries(&tmpdir).unwrap().follow(true).into_iter();
    //     assert_iter_eq(iter, vec![&tmpdir, &dir2, &file2, &file3, &dir1, &dir2, &file2, &file1]);

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_depth()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = tmpdir.mash("dir1");
    //     let dir1file1 = dir1.mash("file1");
    //     let file1 = tmpdir.mash("file1");
    //     let dir2 = dir1.mash("dir2");
    //     let dir2file1 = dir2.mash("file1");

    //     assert_stdfs_mkdir_p!(&dir2);
    //     assert_stdfs_mkfile!(&dir1file1);
    //     assert_stdfs_mkfile!(&dir2file1);
    //     assert_stdfs_mkfile!(&file1);

    //     // Min: 0, Max: 0 = only root
    //     let mut iter = Stdfs::entries(&tmpdir).unwrap().max_depth(0).into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
    //     assert!(iter.next().is_none());

    //     // Min: 0, Max: 1 = root and immediate children
    //     let iter = Stdfs::entries(&tmpdir).unwrap().max_depth(1).into_iter();
    //     assert_iter_eq(iter, vec![&tmpdir, &file1, &dir1]);

    //     // Min: 0, Max: 2 = root, its immediate children and their immediate children
    //     let iter = Stdfs::entries(&tmpdir).unwrap().max_depth(2).into_iter();
    //     assert_iter_eq(iter, vec![&tmpdir, &file1, &dir1, &dir2, &dir1file1]);

    //     // Min: 1, Max: max = skip root, all rest
    //     let iter = Stdfs::entries(&tmpdir).unwrap().min_depth(1).into_iter();
    //     assert_iter_eq(iter, vec![&file1, &dir1, &dir2, &dir1file1, &dir2file1]);

    //     // Min: 1, Max: 1 = skip root, hit root's children only
    //     let iter = Stdfs::entries(&tmpdir).unwrap().min_depth(1).max_depth(1).into_iter();
    //     assert_iter_eq(iter, vec![&file1, &dir1]);

    //     // Min: 1, Max: 2 = skip root, hit root's chilren and theirs only
    //     let iter = Stdfs::entries(&tmpdir).unwrap().min_depth(1).max_depth(2).into_iter();
    //     assert_iter_eq(iter, vec![&file1, &dir1, &dir2, &dir1file1]);

    //     // Min: 2, Max: 1 - max should get corrected to 2 because of ordering
    //     let iter = Stdfs::entries(&tmpdir).unwrap().min_depth(2).max_depth(1).into_iter();
    //     assert_eq!(iter.opts.min_depth, 2);
    //     assert_eq!(iter.opts.max_depth, 2);
    //     assert_iter_eq(iter, vec![&dir2, &dir1file1]);

    //     // Min: 2, Max: 1 - min should get corrected to 1 because of ordering
    //     let iter = Stdfs::entries(&tmpdir).unwrap().max_depth(1).min_depth(2).into_iter();
    //     assert_eq!(iter.opts.min_depth, 1);
    //     assert_eq!(iter.opts.max_depth, 1);
    //     assert_iter_eq(iter, vec![&file1, &dir1]);

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // // #[test]
    // // fn test_memfs_multiple()
    // // {
    // //     let memfs = Memfs::new();
    // //     let tmpdir = PathBuf::from(testing::TEST_TEMP_DIR);
    // //     let dir1 = tmpdir.mash("dir1");
    // //     let file1 = dir1.mash("file1");
    // //     let dir2 = dir1.mash("dir2");
    // //     let file2 = dir2.mash("file2");
    // //     let file3 = tmpdir.mash("file3");
    // //     let link1 = tmpdir.mash("link1");

    // //     assert_stdfs_mkdir_p!(&dir2);
    // //     assert_stdfs_mkfile!(&file1);
    // //     assert_stdfs_mkfile!(&file2);
    // //     assert_stdfs_mkfile!(&file3);
    // //     assert_eq!(Stdfs::symlink(&file3, &link1).unwrap(), link1);

    // //     let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
    // //     assert_iter_eq(iter, vec![&tmpdir, &file3, &dir1, &dir2, &file2, &file1, &link1]);
    // //     assert_stdfs_remove_all!(&tmpdir);
    // // }

    // #[test]
    // fn test_stdfs_multiple()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = tmpdir.mash("dir1");
    //     let file1 = dir1.mash("file1");
    //     let dir2 = dir1.mash("dir2");
    //     let file2 = dir2.mash("file2");
    //     let file3 = tmpdir.mash("file3");
    //     let link1 = tmpdir.mash("link1");

    //     assert_stdfs_mkdir_p!(&dir2);
    //     assert_stdfs_mkfile!(&file1);
    //     assert_stdfs_mkfile!(&file2);
    //     assert_stdfs_mkfile!(&file3);
    //     assert_eq!(Stdfs::symlink(&file3, &link1).unwrap(), link1);

    //     let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
    //     assert_iter_eq(iter, vec![&tmpdir, &file3, &dir1, &dir2, &file2, &file1, &link1]);
    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_memfs_single()
    // {
    //     // Single directory
    //     let memfs = Memfs::new();
    //     assert_eq!(memfs.mkdir_p("dir1").unwrap(), PathBuf::from("/dir1"));
    //     let mut iter = memfs.entries("dir1").unwrap().into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/dir1"));
    //     assert!(iter.next().is_none());

    //     // Single file
    //     assert_eq!(memfs.mkfile("dir1/file1").unwrap(), PathBuf::from("/dir1/file1"));
    //     let mut iter = memfs.entries("/dir1/file1").unwrap().into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/dir1/file1"));
    //     assert!(iter.next().is_none());
    // }

    // #[test]
    // fn test_stdfs_single()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let file1 = tmpdir.mash("file1");

    //     // Single directory
    //     let mut iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
    //     assert!(iter.next().is_none());

    //     // Single file
    //     assert!(Stdfs::mkfile(&file1).is_ok());
    //     let mut iter = Stdfs::entries(&file1).unwrap().into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert!(iter.next().is_none());

    //     assert_stdfs_remove_all!(&tmpdir);
    // }
}

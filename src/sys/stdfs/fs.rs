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

use crate::{
    errors::*,
    exts::*,
    sys::{self, Entries, Entry, FileSystem, StdfsEntry, Vfs},
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
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let home = Stdfs::home_dir().unwrap();
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
        path_buf = sys::clean(path_buf)?;

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
    /// assert_setup_func!();
    /// let tmpdir = assert_setup!("pathext_trait_chmod");
    /// let file1 = tmpdir.mash("file1");
    /// assert_mkfile!(&file1);
    /// assert!(file1.chmod(0o644).is_ok());
    /// assert_eq!(file1.mode().unwrap(), 0o100644);
    /// assert!(file1.chmod(0o555).is_ok());
    /// assert_eq!(file1.mode().unwrap(), 0o100555);
    /// assert_remove_all!(&tmpdir);
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

    /// Returns the current working directory as a [`PathBuf`].
    /// Wraps std::env::current_dir
    ///
    /// # Errors
    ///
    /// Returns an [`Err`] if the current working directory value is invalid.
    /// Possible cases:
    ///
    /// * Current directory does not exist.
    /// * There are insufficient permissions to access the current directory.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// println!("current working directory: {:?}", Stdfs::cwd().unwrap());
    /// ```
    pub fn cwd() -> RvResult<PathBuf>
    {
        let path = std::env::current_dir()?;
        Ok(path)
    }

    /// Returns true if the `Path` exists. Handles path expansion.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/etc").exists(), true);
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

    /// Returns true if the given path exists and is a directory. Handles path expansion.
    /// Only looks at the given path thus a link will not be considered a directory even
    /// if it points to a directory.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("vfs_stdfs_func_is_dir");
    /// assert_eq!(Stdfs::is_dir(&tmpdir), true);
    /// assert_stdfs_remove_all!(&tmpdir);
    /// ```
    pub fn is_dir<T: AsRef<Path>>(path: T) -> bool
    {
        match fs::symlink_metadata(path.as_ref()) {
            Ok(x) => !x.file_type().is_symlink() && x.is_dir(),
            _ => false,
        }
    }

    /// Returns true if the given path exists and is a file. Handles path expansion.
    /// Only looks at the given path thus a link will not be considered a file even
    /// if it points to a file.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_stdfs_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("vfs_stdfs_func_is_file");
    /// let file1 = tmpdir.mash("file1");
    /// assert_eq!(Stdfs::is_file(&file1), false);
    /// assert_stdfs_touch!(&file1);
    /// assert_eq!(Stdfs::is_file(&file1), true);
    /// assert_stdfs_remove_all!(&tmpdir);
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
    // /// assert_setup_func!();
    // /// let tmpdir = assert_setup!("pathext_trait_is_exec");
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
    // /// assert_setup_func!();
    // /// let tmpdir = assert_setup!("pathext_trait_is_readonly");
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
    // /// assert_setup_func!();
    // /// let tmpdir = assert_setup!("pathext_trait_is_symlink");
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
    // /// assert_setup_func!();
    // /// let tmpdir = assert_setup!("pathext_trait_is_symlink_dir");
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
    // /// assert_setup_func!();
    // /// let tmpdir = assert_setup!("pathext_trait_is_symlink_file");
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

    /// Wraps `mkdir` allowing for setting the directory's mode.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_stdfs_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("vfs_stdfs_func_mkdir_m");
    /// let dir1 = tmpdir.mash("dir1");
    /// assert!(Stdfs::mkdir_m(&dir1, 0o555).is_ok());
    /// assert_eq!(Stdfs::mode(&dir1).unwrap(), 0o40555);
    /// assert_stdfs_remove_all!(&tmpdir);
    /// ```
    pub fn mkdir_m<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<PathBuf>
    {
        let path = Stdfs::abs(path)?;

        // For each directory created apply the same permission given
        let path_str = path.to_string()?;
        let mut dir = PathBuf::from("/");
        let mut components = path_str.split('/').rev().collect::<Vec<&str>>();
        while !components.is_empty() {
            dir = sys::mash(dir, components.pop().unwrap());
            if !dir.exists() {
                fs::create_dir(&dir)?;
                fs::set_permissions(&dir, fs::Permissions::from_mode(mode))?;
            }
        }
        Ok(path)
    }

    /// Creates the given directory and any parent directories needed, handling path expansion and
    /// returning the absolute path of the created directory
    ///
    /// # Arguments
    /// * `path` - the target directory to create
    ///
    /// # Errors
    /// * PathError::IsNotDir when the path already exists
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_setup_func!();
    /// let tmpdir = assert_setup!("vfs_stdfs_func_mkdir");
    /// let dir1 = tmpdir.mash("dir1");
    /// assert_eq!(Stdfs::exists(&dir1), false);
    /// assert!(Stdfs::mkdir(&dir1).is_ok());
    /// assert_eq!(Stdfs::exists(&dir1), true);
    /// assert_stdfs_remove_all!(&tmpdir);
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
    /// assert_stdfs_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("vfs_stdfs_func_mode");
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::mkfile_m(&file1, 0o555).is_ok());
    /// assert_eq!(Stdfs::mode(&file1).unwrap(), 0o100555);
    /// assert_stdfs_remove_all!(&tmpdir);
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
    /// assert_stdfs_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("vfs_stdfs_func_read");
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::write(&file1, "this is a test").is_ok());
    /// assert_eq!(Stdfs::read(&file1).unwrap(), "this is a test");
    /// assert_stdfs_remove_all!(&tmpdir);
    /// ```
    pub fn read_all<T: AsRef<Path>>(path: T) -> RvResult<String>
    {
        let path = Stdfs::abs(path.as_ref())?;
        match std::fs::read_to_string(path) {
            Ok(data) => Ok(data),
            Err(err) => Err(err.into()),
        }
    }

    // /// Returns the absolute path for the link target. Handles path expansion
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_setup_func!();
    // /// let tmpdir = assert_setup!("pathext_trait_readlink");
    // /// let file1 = tmpdir.mash("file1");
    // /// let link1 = tmpdir.mash("link1");
    // /// assert_mkfile!(&file1);
    // /// assert!(Stdfs::symlink(&file1, &link1).is_ok());
    // /// assert_eq!(link1.readlink().unwrap(), PathBuf::from("file1"));
    // /// assert_remove_all!(&tmpdir);
    // /// ```
    // pub fn readlink(&self) -> RvResult<PathBuf> {
    //     Stdfs::readlink(self)
    // }

    /// Returns the absolute path for the given link target. Handles path expansion for
    /// the given link. Useful for determining the absolute path of source relative to the
    /// link rather than cwd.
    //
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_stdfs_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("vfs_stdfs_func_readlink_abs");
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_stdfs_touch!(&file1);
    /// assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);
    /// assert_eq!(Stdfs::readlink_abs(link1).unwrap(), file1);
    /// assert_stdfs_remove_all!(&tmpdir);
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

    /// Removes the given empty directory or file. Handles path expansion. Does not follow
    /// symbolic links but rather removes the links themselves. A directory that contains
    /// files will trigger an error use `remove_all` if this is undesired.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_stdfs_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("vfs_stdfs_func_remove");
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

    /// Removes the given directory after removing all of its contents. Handles path expansion.
    /// Does not follow symbolic links but rather removes the links themselves.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("remove_all");
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
    // /// assert_setup_func!();
    // /// let tmpdir = assert_setup!("pathext_trait_set_mode");
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

    /// Creates a new symbolic link. Handles path expansion and returns an absolute path to the
    /// link. Always computes the target `src` path relative to the `dst` link name's absolute path.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_stdfs_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("vfs_stdfs_func_symlink");
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// assert_stdfs_touch!(&file1);
    /// assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);
    /// assert_eq!(Stdfs::readlink(&link1).unwrap(), PathBuf::from("file1"));
    /// assert_stdfs_remove_all!(&tmpdir);
    /// ```
    pub fn symlink<T: AsRef<Path>, U: AsRef<Path>>(src: T, dst: U) -> RvResult<PathBuf>
    {
        let src = src.as_ref().to_owned();

        // Ensure dst is rooted properly standard lookup
        let dst = Stdfs::abs(dst.as_ref())?;

        // If source is not rooted then it is already relative to the dst thus mashing the dst's
        // directory
        // to the src and cleaning it will given an absolute path.
        let src = Stdfs::abs(if !src.is_absolute() { sys::mash(sys::dir(&dst)?, src) } else { src })?;

        // Keep the source path relative if possible,
        let src = Stdfs::relative(src, sys::dir(&dst)?)?;

        unix::fs::symlink(src, &dst)?;
        Ok(dst)
    }

    /// Create an empty file similar to the linux touch command. Handles path expansion.
    /// Uses default file creation permissions 0o666 - umask usually ends up being 0o644.
    /// If the path already exists and is a file only the access and modified times are changed.
    /// If the path already exists and isn't a file an error is returned.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_stdfs_setup_func!();
    /// let tmpdir = assert_stdfs_setup!("vfs_stdfs_func_mkfile");
    /// let file1 = tmpdir.mash("file1");
    /// assert_eq!(Stdfs::is_file(&file1), false);
    /// assert_eq!(Stdfs::mkfile(&file1).unwrap(), file1);
    /// assert_eq!(Stdfs::is_file(&file1), true);
    /// assert_stdfs_remove_all!(&tmpdir);
    /// ```
    pub fn touch<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
    {
        let path = Stdfs::abs(path)?;
        let meta = fs::symlink_metadata(&path);
        if let Err(_) = meta {
            File::create(&path)?;
        } else {
            let meta = meta.unwrap();
            if !meta.is_file() {
                return Err(PathError::IsNotFile(path).into());
            }
            let now = SystemTime::now();
            Stdfs::set_file_time(&path, now, now)?;
        }
        Ok(path)
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
    pub fn write_all<T: AsRef<Path>>(path: T, data: &[u8]) -> RvResult<()>
    {
        let path = Stdfs::abs(path)?;
        if Stdfs::exists(&path) && !Stdfs::is_file(&path) {
            return Err(PathError::IsNotFile(path).into());
        }

        // Create or truncate the target file
        let mut f = File::create(&path)?;
        f.write_all(data)?;

        // f.sync_all() works better than f.flush()?
        f.sync_all()?;
        Ok(())
    }
}

impl FileSystem for Stdfs
{
    /// Return the path in an absolute clean form
    fn abs(&self, path: &Path) -> RvResult<PathBuf>
    {
        Stdfs::abs(path)
    }

    /// Returns the current working directory
    fn cwd(&self) -> RvResult<PathBuf>
    {
        Stdfs::cwd()
    }

    // /// Returns an iterator over the given path
    // fn entries(&self, path: &Path) -> RvResult<Entries>
    // {
    //     Ok(Entries {
    //         root: StdfsEntry::from(path)?.upcast(),
    //         dirs: Default::default(),
    //         files: Default::default(),
    //         follow: false,
    //         min_depth: 0,
    //         max_depth: std::usize::MAX,
    //         max_descriptors: sys::DEFAULT_MAX_DESCRIPTORS,
    //         dirs_first: false,
    //         files_first: false,
    //         contents_first: false,
    //         sort_by_name: false,
    //         pre_op: None,
    //         sort: None,
    //         iter_from: Box::new(StdfsEntry::iter),
    //     })
    // }

    /// Returns true if the `Path` exists. Handles path expansion.
    fn exists(&self, path: &Path) -> bool
    {
        Stdfs::exists(path)
    }

    /// Creates the given directory and any parent directories needed, handling path expansion and
    /// returning the absolute path of the created directory
    fn mkdir_p(&self, path: &Path) -> RvResult<PathBuf>
    {
        Stdfs::mkdir_p(path)
    }

    /// Read all data from the given file and return it as a String
    fn read_all(&self, path: &Path) -> RvResult<String>
    {
        Stdfs::read_all(path)
    }

    /// Write the given data to to the indicated file creating the file first if it doesn't exist
    /// or truncating it first if it does.
    fn write_all(&self, path: &Path, data: &[u8]) -> RvResult<()>
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
        let prev = sys::dir(&cwd)?;

        // expand relative directory
        assert_eq!(Stdfs::abs("foo")?, sys::mash(&cwd, "foo"));

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
        assert_eq!(Stdfs::abs("~/foo")?, sys::mash(&home, "foo"));

        // More complicated
        assert_eq!(Stdfs::abs("~/foo/bar/../.")?, sys::mash(&home, "foo"));
        assert_eq!(Stdfs::abs("~/foo/bar/../")?, sys::mash(&home, "foo"));
        assert_eq!(Stdfs::abs("~/foo/bar/../blah")?, sys::mash(&home, "foo/blah"));

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
    // fn test_vfs_stdfs_exists() {
    //     let tmpdir = assert_stdfs_setup!();
    //     let tmpfile = tmpdir.mash("file");
    //     assert_stdfs_remove_all!(&tmpdir);
    //     assert!(Stdfs::mkdir(&tmpdir).is_ok());
    //     assert_eq!(Stdfs::exists(&tmpfile), false);
    //     assert!(Stdfs::mkfile(&tmpfile).is_ok());
    //     assert_eq!(Stdfs::exists(&tmpfile), true);
    //     assert_stdfs_remove_all!(&tmpdir);
    // }
}

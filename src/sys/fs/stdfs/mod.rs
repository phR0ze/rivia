mod entry;
mod vfs;

pub use entry::*;

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

use crate::{
    core::*,
    errors::*,
    sys::{
        self, Chmod, ChmodOpts, Chown, ChownOpts, Copier, CopyOpts, Entries, Entry, EntryIter, PathExt, ReadSeek,
        VfsEntry,
    },
};

/// Provides a wrapper around the `std::fs` module as a [`VirtualFileSystem`] backend implementation
#[derive(Debug, Default)]
pub struct Stdfs;
impl Stdfs {
    /// Create a new instance of the Stdfs Vfs backend implementation
    pub fn new() -> Self {
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
    pub fn abs<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_all_dirs");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = dir1.mash("dir2");
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_iter_eq(Stdfs::all_dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn all_dirs<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>> {
        let mut paths: Vec<PathBuf> = vec![];
        let src = StdfsEntry::from(path)?;
        if !src.is_dir() {
            return Err(PathError::is_not_dir(src.path_buf()).into());
        }
        for entry in Stdfs::entries(src.path())?.min_depth(1).sort_by_name().dirs() {
            let entry = entry?;
            paths.push(entry.path_buf());
        }
        Ok(paths)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_all_files");
    /// let file1 = tmpdir.mash("file1");
    /// let dir1 = tmpdir.mash("dir1");
    /// let file2 = dir1.mash("file2");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file2);
    /// assert_iter_eq(Stdfs::all_files(&tmpdir).unwrap(), vec![file2, file1]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn all_files<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>> {
        let mut paths: Vec<PathBuf> = vec![];
        let src = StdfsEntry::from(path)?;
        if !src.is_dir() {
            return Err(PathError::is_not_dir(src.path_buf()).into());
        }
        for entry in Stdfs::entries(src.path())?.min_depth(1).sort_by_name().files() {
            let entry = entry?;
            paths.push(entry.path_buf());
        }
        Ok(paths)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_all_paths");
    /// let file1 = tmpdir.mash("file1");
    /// let dir1 = tmpdir.mash("dir1");
    /// let file2 = dir1.mash("file2");
    /// let file3 = dir1.mash("file3");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file2);
    /// assert_vfs_mkfile!(vfs, &file3);
    /// assert_iter_eq(Stdfs::all_paths(&tmpdir).unwrap(), vec![dir1, file2, file3, file1]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn all_paths<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>> {
        let mut paths: Vec<PathBuf> = vec![];
        let src = StdfsEntry::from(path)?;
        if !src.is_dir() {
            return Err(PathError::is_not_dir(src.path_buf()).into());
        }
        for entry in Stdfs::entries(src.path())?.min_depth(1).sort_by_name() {
            let entry = entry?;
            paths.push(entry.path_buf());
        }
        Ok(paths)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_append");
    /// let file = tmpdir.mash("file");
    /// let mut f = Stdfs::write(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// let mut f = Stdfs::append(&file).unwrap();
    /// f.write_all(b"123").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar123");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn append<T: AsRef<Path>>(path: T) -> RvResult<Box<dyn Write>> {
        // Ensure the file exists as the std functions don't do that
        Stdfs::mkfile(&path)?;

        Ok(Box::new(File::options().append(true).open(Stdfs::abs(path)?)?))
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_append_all");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, "foobar 1");
    /// assert!(Stdfs::append_all(&file, "foobar 2").is_ok());
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1foobar 2");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn append_all<T: AsRef<Path>, U: AsRef<[u8]>>(path: T, data: U) -> RvResult<()> {
        let mut f = Stdfs::append(path)?;
        f.write_all(data.as_ref())?;
        f.flush()?;
        Ok(())
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_append_line");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, "foobar 1");
    /// assert!(Stdfs::append_line(&file, "foobar 2").is_ok());
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1foobar 2\n");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn append_line<T: AsRef<Path>, U: AsRef<str>>(path: T, line: U) -> RvResult<()> {
        let line = line.as_ref().to_string();
        if !line.is_empty() {
            Stdfs::append_all(path, line + "\n")?;
        }
        Ok(())
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_append_lines");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert!(Stdfs::append_lines(&file, &["1", "2"]).is_ok());
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "1\n2\n");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn append_lines<T: AsRef<Path>, U: AsRef<str>>(path: T, lines: &[U]) -> RvResult<()> {
        let lines = lines.iter().map(|x| x.as_ref()).collect::<Vec<&str>>().join("\n");
        if !lines.is_empty() {
            Stdfs::append_all(path, lines + "\n")?;
        }
        Ok(())
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_chmod");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(Stdfs::mode(&file).unwrap(), 0o100644);
    /// assert!(Stdfs::chmod(&file, 0o555).is_ok());
    /// assert_eq!(Stdfs::mode(&file).unwrap(), 0o100555);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn chmod<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<()> {
        Stdfs::chmod_b(path)?.all(mode).exec()
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_chmod_b");
    /// let dir = tmpdir.mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(Stdfs::mode(&dir).unwrap(), 0o40755);
    /// assert_eq!(Stdfs::mode(&file).unwrap(), 0o100644);
    /// assert!(Stdfs::chmod_b(&dir).unwrap().recurse().all(0o777).exec().is_ok());
    /// assert_eq!(Stdfs::mode(&dir).unwrap(), 0o40777);
    /// assert_eq!(Stdfs::mode(&file).unwrap(), 0o100777);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn chmod_b<T: AsRef<Path>>(path: T) -> RvResult<Chmod> {
        Ok(Chmod {
            opts: ChmodOpts {
                path: Stdfs::abs(path)?,
                dirs: 0,
                files: 0,
                follow: false,
                recursive: true,
                sym: "".to_string(),
            },
            exec: Box::new(Stdfs::_chmod),
        })
    }

    // Execute chmod with the given [`Mode`] options
    fn _chmod(opts: ChmodOpts) -> RvResult<()> {
        // Using `contents_first` to yield directories last so that revoking permissions happen to
        // directories as the last thing when completing the traversal, else we'll lock
        // ourselves out.
        let mut entries = Stdfs::entries(&opts.path)?.contents_first();

        // Set the `max_depth` based on recursion
        entries = entries.max_depth(match opts.recursive {
            true => usize::MAX,
            false => 0,
        });

        // Using `dirs_first` and `pre_op` options here to grant addative permissions as a
        // pre-traversal operation to allow for the possible addition of permissions that would allow
        // directory traversal that otherwise wouldn't be allowed.
        let m = opts.clone();
        entries = entries.follow(opts.follow).dirs_first().pre_op(move |x| {
            let m1 = sys::mode(x, m.dirs, &m.sym)?;
            if (!x.is_symlink() || m.follow) && x.is_dir() && !sys::revoking_mode(x.mode(), m1) && x.mode() != m1 {
                fs::set_permissions(x.path(), fs::Permissions::from_mode(m1))?;
            }
            Ok(())
        });

        // Set permissions on the way out for everything specified
        for entry in entries {
            let src = entry?;

            // Compute mode based on octal and symbolic values
            let m2 = if src.is_dir() {
                sys::mode(&src, opts.dirs, &opts.sym)?
            } else if src.is_file() {
                sys::mode(&src, opts.files, &opts.sym)?
            } else {
                0
            };

            // Apply permission to entry if set
            if (!src.is_symlink() || opts.follow) && m2 != src.mode() && m2 != 0 {
                fs::set_permissions(src.path(), fs::Permissions::from_mode(m2))?;
            }
        }

        Ok(())
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_chown");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// let (uid, gid) = Stdfs::owner(&file1).unwrap();
    /// assert!(Stdfs::chown(&file1, uid, gid).is_ok());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn chown<T: AsRef<Path>>(path: T, uid: u32, gid: u32) -> RvResult<()> {
        Stdfs::chown_b(path)?.owner(uid, gid).exec()
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_chown_b");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// let (uid, gid) = Stdfs::owner(&file1).unwrap();
    /// assert!(Stdfs::chown_b(&file1).unwrap().owner(uid, gid).exec().is_ok());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn chown_b<T: AsRef<Path>>(path: T) -> RvResult<Chown> {
        Ok(Chown {
            opts: ChownOpts {
                path: Stdfs::abs(path)?,
                uid: None,
                gid: None,
                follow: false,
                recursive: true,
            },
            exec: Box::new(Stdfs::_chown),
        })
    }

    // Execute chown with the given [`Chown`] options
    fn _chown(opts: ChownOpts) -> RvResult<()> {
        let max_depth = if opts.recursive { usize::MAX } else { 0 };
        for entry in Stdfs::entries(&opts.path)?.max_depth(max_depth).follow(opts.follow) {
            let src = entry?;
            let uid = opts.uid.map(nix::unistd::Uid::from_raw);
            let gid = opts.gid.map(nix::unistd::Gid::from_raw);
            nix::unistd::chown(src.path(), uid, gid)?;
        }
        Ok(())
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
    /// let vfs = Vfs::memfs(); // replace this with Vfs::stdfs() for the real filesystem
    /// let dir = PathBuf::from("/etc/xdg");
    /// vfs.mkdir_p(&dir).unwrap();
    /// let filepath = dir.mash("rivia.toml");
    /// vfs.write_all(&filepath, "this is a test").unwrap();
    /// assert_eq!(vfs.config_dir("rivia.toml").unwrap().to_str().unwrap(), "/etc/xdg");
    ///
    /// if let Some(config_dir) = vfs.config_dir("rivia.toml") {
    ///    let path = config_dir.mash("rivia.toml");
    ///    let config = vfs.read_all(&path).unwrap();
    ///    assert_eq!(config, "this is a test");
    /// }
    /// ```
    fn config_dir<T: AsRef<str>>(config: T) -> Option<PathBuf> {
        if let Ok(config_dir) = crate::sys::user::config_dir() {
            if let Ok(mut config_dirs) = crate::sys::user::sys_config_dirs() {
                config_dirs.insert(0, config_dir);
                for config_dir in config_dirs {
                    let path = config_dir.mash(config.as_ref());
                    if Stdfs::exists(path) {
                        return Some(config_dir);
                    }
                }
            }
        }
        None
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_copy");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// assert_vfs_write_all!(vfs, &file1, "this is a test");
    /// assert!(Stdfs::copy(&file1, &file2).is_ok());
    /// assert_vfs_read_all!(vfs, &file2, "this is a test");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn copy<T: AsRef<Path>, U: AsRef<Path>>(src: T, dst: U) -> RvResult<()> {
        Stdfs::copy_b(src, dst)?.exec()
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_copy_b");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// assert_vfs_write_all!(vfs, &file1, "this is a test");
    /// assert!(Stdfs::copy_b(&file1, &file2).unwrap().exec().is_ok());
    /// assert_vfs_read_all!(vfs, &file2, "this is a test");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn copy_b<T: AsRef<Path>, U: AsRef<Path>>(src: T, dst: U) -> RvResult<Copier> {
        Ok(Copier {
            opts: CopyOpts {
                src: src.as_ref().to_owned(),
                dst: dst.as_ref().to_owned(),
                mode: Default::default(),
                cdirs: Default::default(),
                cfiles: Default::default(),
                follow: Default::default(),
            },
            exec: Box::new(Stdfs::_copy),
        })
    }

    // Execute copy with the given [`CopyOpts`] option
    fn _copy(cp: sys::CopyOpts) -> RvResult<()> {
        // Resolve abs paths
        let src_root = Stdfs::abs(&cp.src)?;
        let dst_root = Stdfs::abs(&cp.dst)?;

        // Detect source is destination
        if src_root == dst_root {
            return Ok(());
        }

        // Determine the given modes
        let dir_mode = match cp.mode {
            Some(x) if cp.cdirs || !cp.cfiles => Some(x),
            _ => None,
        };
        let file_mode = match cp.mode {
            Some(x) if cp.cfiles || !cp.cdirs => Some(x),
            _ => None,
        };

        // Copy into requires a pre-existing destination directory
        let copy_into = Stdfs::is_dir(&dst_root);

        // Iterate over source taking into account link following
        let src_root = StdfsEntry::from(&src_root)?.follow(cp.follow);
        for entry in Stdfs::entries(src_root.path())?.follow(cp.follow) {
            let src = entry?;

            // Set destination path based on source path
            let dst_path = if copy_into {
                dst_root.mash(src.path().trim_prefix(src_root.path().dir()?))
            } else {
                dst_root.mash(src.path().trim_prefix(src_root.path()))
            };

            // Recreate links if were not following them
            if !cp.follow && src.is_symlink() {
                Stdfs::symlink(dst_path, src.alt())?;
            } else if src.is_dir() {
                Stdfs::mkdir_m(&dst_path, dir_mode.unwrap_or(src.mode()))?;
            } else {
                // Copying into a directory might require creating it first
                if !Stdfs::exists(&dst_path.dir()?) {
                    Stdfs::mkdir_m(
                        &dst_path.dir()?,
                        match dir_mode {
                            Some(x) => x,
                            None => StdfsEntry::from(src.path().dir()?)?.mode(),
                        },
                    )?;
                }

                // Copy over the file/link
                fs::copy(src.path(), &dst_path)?;

                // Optionally set new mode
                if let Some(mode) = file_mode {
                    fs::set_permissions(&dst_path, fs::Permissions::from_mode(mode))?;
                }
            }
        }

        Ok(())
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
    pub fn cwd() -> RvResult<PathBuf> {
        let path = std::env::current_dir()?;
        Ok(path)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_dirs");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_iter_eq(Stdfs::dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn dirs<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>> {
        let mut paths: Vec<PathBuf> = vec![];
        if !Stdfs::is_dir(&path) {
            return Err(PathError::is_not_dir(&path).into());
        }
        for entry in Stdfs::entries(path)?.min_depth(1).max_depth(1).sort_by_name().dirs() {
            let entry = entry?;
            paths.push(entry.path_buf());
        }
        Ok(paths)
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
    /// assert_iter_eq(iter.map(|x| x.unwrap().path_buf()), vec![file1]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn entries<T: AsRef<Path>>(path: T) -> RvResult<Entries> {
        Ok(Entries {
            root: StdfsEntry::from(path)?.upcast(),
            dirs: Default::default(),
            files: Default::default(),
            follow: false,
            min_depth: 0,
            max_depth: usize::MAX,
            max_descriptors: sys::DEFAULT_MAX_DESCRIPTORS,
            dirs_first: false,
            files_first: false,
            contents_first: false,
            sort_by_name: false,
            pre_op: None,
            sort: None,
            iter_from: Box::new(Stdfs::entry_iter),
        })
    }

    /// Return a virtual filesystem entry for the given path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_entry");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(Stdfs::entry(&file).unwrap().is_file());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn entry<T: AsRef<Path>>(path: T) -> RvResult<VfsEntry> {
        Ok(StdfsEntry::from(path)?.upcast())
    }

    /// Return a EntryIter function
    pub(crate) fn entry_iter(path: &Path, follow: bool) -> RvResult<EntryIter> {
        Ok(EntryIter {
            path: path.to_path_buf(),
            cached: false,
            following: follow,
            iter: Box::new(StdfsEntryIter {
                dir: fs::read_dir(path)?,
            }),
        })
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_files");
    /// let dir = tmpdir.mash("dir");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file2);
    /// assert_iter_eq(Stdfs::files(&tmpdir).unwrap(), vec![file1, file2]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn files<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>> {
        let mut paths: Vec<PathBuf> = vec![];
        if !Stdfs::is_dir(&path) {
            return Err(PathError::is_not_dir(&path).into());
        }
        for entry in Stdfs::entries(path)?.min_depth(1).max_depth(1).sort_by_name().files() {
            let entry = entry?;
            paths.push(entry.path_buf());
        }
        Ok(paths)
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
    pub fn gid<T: AsRef<Path>>(path: T) -> RvResult<u32> {
        Ok(fs::metadata(Stdfs::abs(path)?)?.gid())
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
    pub fn exists<T: AsRef<Path>>(path: T) -> bool {
        match Stdfs::abs(path) {
            Ok(abs) => fs::metadata(abs).is_ok(),
            Err(_) => false,
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_is_exec");
    /// let file = tmpdir.mash("file");
    /// assert!(Stdfs::mkfile_m(&file, 0o644).is_ok());
    /// assert_eq!(Stdfs::is_exec(&file), false);
    /// assert!(Stdfs::chmod(&file, 0o777).is_ok());
    /// assert_eq!(Stdfs::is_exec(&file), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn is_exec<T: AsRef<Path>>(path: T) -> bool {
        match Stdfs::abs(path) {
            Ok(x) => match fs::metadata(x) {
                Ok(y) => y.permissions().mode() & 0o111 != 0,
                Err(_) => false,
            },
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
    pub fn is_dir<T: AsRef<Path>>(path: T) -> bool {
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
    pub fn is_file<T: AsRef<Path>>(path: T) -> bool {
        match fs::symlink_metadata(path.as_ref()) {
            Ok(x) => !x.file_type().is_symlink() && x.is_file(),
            _ => false,
        }
    }

    /// Returns true if the given path exists and is readonly
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Example
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_is_readonly");
    /// let file = tmpdir.mash("file1");
    /// assert!(Stdfs::mkfile_m(&file, 0o644).is_ok());
    /// assert_eq!(Stdfs::is_readonly(&file), false);
    /// assert!(Stdfs::chmod_b(&file).unwrap().readonly().exec().is_ok());
    /// assert_eq!(Stdfs::mode(&file).unwrap(), 0o100444);
    /// assert_eq!(Stdfs::is_readonly(&file), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn is_readonly<T: AsRef<Path>>(path: T) -> bool {
        match Stdfs::abs(path) {
            Ok(x) => match fs::metadata(x) {
                Ok(y) => y.permissions().readonly(),
                Err(_) => false,
            },
            Err(_) => false,
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_is_symlink");
    /// let file = tmpdir.mash("file");
    /// let link = tmpdir.mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_no_symlink!(vfs, &link);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// assert_vfs_is_symlink!(vfs, &link);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn is_symlink<T: AsRef<Path>>(path: T) -> bool {
        match StdfsEntry::from(path) {
            Ok(x) => x.is_symlink(),
            _ => false,
        }
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
    /// assert_eq!(Stdfs::is_symlink_dir(&link1), true);
    /// assert_eq!(Stdfs::is_symlink_dir(&link2), false);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn is_symlink_dir<T: AsRef<Path>>(path: T) -> bool {
        match StdfsEntry::from(path) {
            Ok(x) => x.is_symlink_dir(),
            _ => false,
        }
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
    /// assert_eq!(Stdfs::is_symlink_file(&link1), false);
    /// assert_eq!(Stdfs::is_symlink_file(&link2), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn is_symlink_file<T: AsRef<Path>>(path: T) -> bool {
        match StdfsEntry::from(path) {
            Ok(x) => x.is_symlink_file(),
            _ => false,
        }
    }

    /// Creates the given directory and any parent directories needed with the given mode
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
    pub fn mkdir_m<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<PathBuf> {
        let abs = Stdfs::abs(path)?;

        let mut path = PathBuf::new();
        for component in abs.components() {
            path.push(component);
            if !path.exists() {
                fs::create_dir(&path)?;
                fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
            }
        }
        Ok(abs)
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_mkdir_p");
    /// let dir1 = tmpdir.mash("dir1");
    /// assert_eq!(Stdfs::exists(&dir1), false);
    /// assert!(Stdfs::mkdir_p(&dir1).is_ok());
    /// assert_eq!(Stdfs::exists(&dir1), true);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn mkdir_p<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
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
    pub fn mkfile<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
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

    /// Wraps `mkfile` allowing for setting the file's mode
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_mkfile_m");
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::mkfile_m(&file1, 0o555).is_ok());
    /// assert_eq!(Stdfs::mode(&file1).unwrap(), 0o100555);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn mkfile_m<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<PathBuf> {
        let path = Stdfs::mkfile(path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
        Ok(path)
    }

    /// Returns the permissions for a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_mode");
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::mkfile_m(&file1, 0o555).is_ok());
    /// assert_eq!(Stdfs::mode(&file1).unwrap(), 0o100555);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn mode<T: AsRef<Path>>(path: T) -> RvResult<u32> {
        let path = Stdfs::abs(path)?;
        let meta = fs::symlink_metadata(path)?;
        Ok(meta.permissions().mode())
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
    pub fn move_p<T: AsRef<Path>, U: AsRef<Path>>(src: T, dst: U) -> RvResult<()> {
        let src_path = Stdfs::abs(src)?;
        let dst_root = Stdfs::abs(dst)?;
        let copy_into = Stdfs::is_dir(&dst_root);

        let dst_path = if copy_into { dst_root.mash(src_path.base()?) } else { dst_root.clone() };
        fs::rename(src_path, dst_path)?;
        Ok(())
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
    /// assert_eq!(Stdfs::owner(vfs.root()).unwrap(), (0, 0));
    /// ```
    pub fn owner<T: AsRef<Path>>(path: T) -> RvResult<(u32, u32)> {
        let meta = fs::metadata(Stdfs::abs(path)?)?;
        Ok((meta.uid(), meta.gid()))
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_paths");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_iter_eq(Stdfs::paths(&tmpdir).unwrap(), vec![dir1, dir2, file1]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn paths<T: AsRef<Path>>(path: T) -> RvResult<Vec<PathBuf>> {
        let mut paths: Vec<PathBuf> = vec![];
        if !Stdfs::is_dir(&path) {
            return Err(PathError::is_not_dir(&path).into());
        }
        for entry in Stdfs::entries(path)?.min_depth(1).max_depth(1).sort_by_name() {
            let entry = entry?;
            paths.push(entry.path_buf());
        }
        Ok(paths)
    }

    /// Open a file in readonly mode
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_read");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// let mut file = Stdfs::read(&file).unwrap();
    /// let mut buf = String::new();
    /// file.read_to_string(&mut buf);
    /// assert_eq!(buf, "foobar 1".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn read<T: AsRef<Path>>(path: T) -> RvResult<Box<dyn ReadSeek>> {
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_read_all");
    /// let file1 = tmpdir.mash("file1");
    /// assert!(Stdfs::write_all(&file1, "this is a test").is_ok());
    /// assert_eq!(Stdfs::read_all(&file1).unwrap(), "this is a test");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn read_all<T: AsRef<Path>>(path: T) -> RvResult<String> {
        let path = Stdfs::abs(path)?;

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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_read_lines");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_write_all!(vfs, &file, "1\n2");
    /// assert_eq!(vfs.read_lines(&file).unwrap(), vec!["1".to_string(), "2".to_string()]);
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn read_lines<T: AsRef<Path>>(path: T) -> RvResult<Vec<String>> {
        let mut lines = vec![];
        for line in BufReader::new(Stdfs::read(path)?).lines() {
            lines.push(line?);
        }
        Ok(lines)
    }

    /// Returns the relative path of the target the link points to
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
    pub fn readlink<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
        Ok(fs::read_link(Stdfs::abs(path)?)?)
    }

    /// Returns the absolute path of the target the link points to
    ///
    /// * Handles path expansion and absolute path resolution
    ///
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
    pub fn readlink_abs<T: AsRef<Path>>(link: T) -> RvResult<PathBuf> {
        Ok(StdfsEntry::from(link)?.alt_buf())
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
    pub fn remove<T: AsRef<Path>>(path: T) -> RvResult<()> {
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
    pub fn remove_all<T: AsRef<Path>>(path: T) -> RvResult<()> {
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
    pub fn root() -> PathBuf {
        let mut root = PathBuf::new();
        root.push(Component::RootDir);
        root
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
    pub fn set_cwd<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
        let path = Stdfs::abs(path)?;
        std::env::set_current_dir(&path)?;
        Ok(path)
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
    pub fn symlink<T: AsRef<Path>, U: AsRef<Path>>(link: T, target: U) -> RvResult<PathBuf> {
        let target = target.as_ref().to_owned();

        // Ensure link is rooted properly
        let link = Stdfs::abs(link)?;

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
    pub fn set_file_time<T: AsRef<Path>>(path: T, atime: SystemTime, mtime: SystemTime) -> RvResult<()> {
        let atime_spec = TimeSpec::from(atime.duration_since(std::time::UNIX_EPOCH)?);
        let mtime_spec = TimeSpec::from(mtime.duration_since(std::time::UNIX_EPOCH)?);
        stat::utimensat(None, path.as_ref(), &atime_spec, &mtime_spec, UtimensatFlags::NoFollowSymlink)?;
        Ok(())
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
    pub fn uid<T: AsRef<Path>>(path: T) -> RvResult<u32> {
        Ok(fs::metadata(Stdfs::abs(path)?)?.uid())
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
    /// let mut f = Stdfs::write(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// assert_vfs_read_all!(vfs, &file, "foobar".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn write<T: AsRef<Path>>(path: T) -> RvResult<Box<dyn Write>> {
        Ok(Box::new(File::create(Stdfs::abs(path)?)?))
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_write_all");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1");
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(path: T, data: U) -> RvResult<()> {
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
    /// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs(), "stdfs_func_write_lines");
    /// let file = tmpdir.mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert!(vfs.write_lines(&file, &["1", "2"]).is_ok());
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "1\n2\n".to_string());
    /// assert_vfs_remove_all!(vfs, &tmpdir);
    /// ```
    pub fn write_lines<T: AsRef<Path>, U: AsRef<str>>(path: T, lines: &[U]) -> RvResult<()> {
        let lines = lines.iter().map(|x| x.as_ref()).collect::<Vec<&str>>().join("\n");
        if !lines.is_empty() {
            Stdfs::write_all(path, lines + "\n")?;
        }
        Ok(())
    }
}

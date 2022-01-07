use crate::{
    errors::*,
    fs::{Entry, FileSystem, StdfsEntry, Vfs},
    iters::*,
};
use nix::sys::{
    stat::{self, UtimensatFlags},
    time::TimeSpec,
};
use std::{
    fs::{self, File},
    io::Write,
    path::{Component, Path, PathBuf},
    os::unix::{
        self,
        fs::PermissionsExt,
    },
    time::SystemTime,
};

/// `Stdfs` is a Vfs backend implementation that wraps the standard library `std::fs`
/// functions for use with Vfs.
#[derive(Debug)]
pub struct Stdfs;
impl Stdfs {
    /// Create a new instance of the Stdfs Vfs backend implementation
    pub fn new() -> Self {
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
    pub fn abs<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
        let path = path.as_ref();

        // Check for empty string
        if Stdfs::is_empty(path) {
            return Err(PathError::Empty.into());
        }

        // Expand home directory
        let mut path_buf = Stdfs::expand(path)?;

        // Trim protocol prefix if needed
        path_buf = Stdfs::trim_protocol(path_buf);

        // Clean the resulting path
        path_buf = Stdfs::clean(path_buf)?;

        // Expand relative directories if needed
        if !path_buf.is_absolute() {
            let mut curr = Stdfs::cwd()?;
            while let Ok(path) = path_buf.components().first_result() {
                match path {
                    Component::CurDir => {
                        path_buf = Stdfs::trim_first(path_buf);
                    },
                    Component::ParentDir => {
                        if curr.to_string()? == "/" {
                        return Err(PathError::ParentNotFound(curr).into());
                        }
                        curr = Stdfs::dir(curr)?;
                        path_buf = Stdfs::trim_first(path_buf);
                    },
                    _ => return Ok(Stdfs::mash(curr, path_buf))
                };
            }
            return Ok(curr);
        }

        Ok(path_buf)
    }

    /// Returns the final component of the `Path`, if there is one.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!("bar", PathBuf::from("/foo/bar").base().unwrap());
    /// ```
    pub fn base<T: AsRef<Path>>(path: T) -> RvResult<String> {
        let path = path.as_ref();
        path.file_name().ok_or_else(|| PathError::filename_not_found(path))?.to_string()
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
    pub fn chmod<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<()> {
        fs::set_permissions(path.as_ref(), fs::Permissions::from_mode(mode))?;
        Ok(())
    }

    /// Return the shortest path equivalent to the path by purely lexical processing and thus does
    /// not handle links correctly in some cases, use canonicalize in those cases. It applies
    /// the following rules interatively until no further processing can be done.
    ///
    /// 1. Replace multiple slashes with a single
    /// 2. Eliminate each . path name element (the current directory)
    /// 3. Eliminate each inner .. path name element (the parent directory)
    ///    along with the non-.. element that precedes it.
    /// 4. Eliminate .. elements that begin a rooted path:
    ///    that is, replace "/.." by "/" at the beginning of a path.
    /// 5. Leave intact ".." elements that begin a non-rooted path.
    /// 6. Drop trailing '/' unless it is the root
    ///
    /// If the result of this process is an empty string, return the string `.`, representing the
    /// current directory.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Stdfs::clean("./foo/./bar").unwrap(), PathBuf::from("foo/bar"));
    /// ```
    pub fn clean<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
        // Components already handles the following cases:
        // 1. Repeated separators are ignored, so a/b and a//b both have a and b as components.
        // 2. Occurrences of . are normalized away, except if they are at the beginning of the path.
        //    e.g. a/./b, a/b/, a/b/. and a/b all have a and b as components, but ./a/b starts with
        // an
        // additional CurDir component.
        // 6. A trailing slash is normalized away, /a/b and /a/b/ are equivalent.
        let mut cnt = 0;
        let mut prev = None;
        let mut path_buf = PathBuf::new();
        for component in path.as_ref().components() {
            match component {
                // 2. Eliminate . path name at begining of path for simplicity
                x if x == Component::CurDir && cnt == 0 => continue,

                // 5. Leave .. begining non rooted path
                x if x == Component::ParentDir && cnt > 0 && !prev.has(Component::ParentDir) => {
                    match prev.unwrap() {
                        // 4. Eliminate .. elements that begin a root path
                        Component::RootDir => {},

                        // 3. Eliminate inner .. path name elements
                        Component::Normal(_) => {
                            cnt -= 1;
                            path_buf.pop();
                            prev = path_buf.components().last();
                        },
                        _ => {},
                    }
                    continue;
                },

                // Normal
                _ => {
                    cnt += 1;
                    path_buf.push(component);
                    prev = Some(component);
                },
            };
        }

        // Ensure if empty the current dir is returned
        if Stdfs::is_empty(&path_buf) {
            path_buf.push(".");
        }
        Ok(path_buf)
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
    pub fn cwd() -> RvResult<PathBuf> {
        let path = std::env::current_dir()?;
        Ok(path)
    }

    /// Returns the `Path` without its final component, if there is one.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Stdfs::dir("/foo/bar").unwrap(), PathBuf::from("/foo").as_path());
    /// ```
    pub fn dir<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
        let path = path.as_ref();
        let dir = path.parent().ok_or_else(|| PathError::parent_not_found(path))?;
        Ok(dir.to_path_buf())
    }

    /// Returns true if the `Path` exists. Handles path expansion.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/etc").exists(), true);
    /// ```
    pub fn exists<T: AsRef<Path>>(path: T) -> bool {
        match Stdfs::abs(path) {
            Ok(abs) => fs::metadata(abs).is_ok(),
            Err(_) => false,
        }
    }

    /// Expand all environment variables in the path as well as the home directory.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let home = Stdfs::home_dir().unwrap();
    /// assert_eq!(Stdfs::expand("~/foo").unwrap(), PathBuf::from(&home).join("foo"));
    /// assert_eq!(Stdfs::expand("$HOME/foo").unwrap(), PathBuf::from(&home).join("foo"));
    /// assert_eq!(Stdfs::expand("${HOME}/foo").unwrap(), PathBuf::from(&home).join("foo"));
    /// ```
    pub fn expand<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
        let path = path.as_ref();
        let pathstr = path.to_string()?;

        // Expand home directory
        let path = match pathstr.matches('~').count() {
            // Only a single home expansion is allowed
            cnt if cnt > 1 => {
                return Err(PathError::multiple_home_symbols(path).into())
            },

            // Home expansion only makes sense at the beinging of a path
            cnt if cnt == 1 && !Stdfs::has_prefix(path, "~/") && pathstr != "~" => {
                return Err(PathError::invalid_expansion(path).into())
            },

            // Single tilda only
            cnt if cnt == 1 && pathstr == "~" => {
                Stdfs::home_dir()?
            },

            // Replace prefix with home directory
            1 => Stdfs::mash(Stdfs::home_dir()?, &pathstr[2..]),
            _ => path.to_path_buf(),
        };

        // Expand other variables that may exist in the path
        let pathstr = path.to_string()?;
        let path = if pathstr.matches('$').some() {
            let mut path_buf = PathBuf::new();
            for x in path.components() {
                match x {
                    Component::Normal(y) => {
                        let mut str = String::new();
                        let seg = y.to_string()?;
                        let mut chars = seg.chars().peekable();

                        while chars.peek().is_some()
                        {
                            // Extract chars up to $ and consumes $ as it has to look at it
                            str += &chars.by_ref().take_while(|&x| x != '$').collect::<String>();

                            // Read variable if it exists
                            if chars.peek().is_some() {
                                chars.next_if_eq(&'{'); // drop {
                                let var = &chars.take_while_p(|&x| x != '$' && x != '}').collect::<String>();
                                chars.next_if_eq(&'}'); // drop }
                                if var == "" {
                                    return Err(PathError::invalid_expansion(seg).into());
                                }
                                str += &std::env::var(var)?;
                            }
                        }

                        path_buf.push(str);
                    },
                    _ => path_buf.push(x),
                };
            }
            path_buf
        } else {
            path
        };

        Ok(path)
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

    // /// Returns true if the `Path` contains the given path or string.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// let path = PathBuf::from("/foo/bar");
    // /// assert_eq!(path.has("foo"), true);
    // /// assert_eq!(path.has("/foo"), true);
    // /// ```
    // pub fn has<T: AsRef<Path>>(&self, path: T) -> bool {
    //     match (self.to_string(), path.as_ref().to_string()) {
    //         (Ok(base), Ok(path)) => base.contains(&path),
    //         _ => false,
    //     }
    // }

    /// Returns true if the `Path` as a String has the given prefix
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let path = PathBuf::from("/foo/bar");
    /// assert_eq!(Stdfs::has_prefix(&path, "/foo"), true);
    /// assert_eq!(Stdfs::has_prefix(&path, "foo"), false);
    /// ```
    pub fn has_prefix<T: AsRef<Path>, U: AsRef<Path>>(path: T, prefix: U) -> bool {
        match (path.as_ref().to_string(), prefix.as_ref().to_string()) {
            (Ok(base), Ok(prefix)) => base.starts_with(&prefix),
            _ => false,
        }
    }

    // /// Returns true if the `Path` as a String has the given suffix
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// let path = PathBuf::from("/foo/bar");
    // /// assert_eq!(path.has_suffix("/bar"), true);
    // /// assert_eq!(path.has_suffix("foo"), false);
    // /// ```
    // pub fn has_suffix<T: AsRef<Path>>(&self, suffix: T) -> bool {
    //     match (self.to_string(), suffix.as_ref().to_string()) {
    //         (Ok(base), Ok(suffix)) => base.ends_with(&suffix),
    //         _ => false,
    //     }
    // }

    /// Returns the full path to the current user's home directory.
    ///
    /// Alternate implementation as the Rust std::env::home_dir implementation has been
    /// deprecated https://doc.rust-lang.org/std/env/fn.home_dir.html
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert!(Stdfs::home_dir().is_ok());
    /// ```
    pub fn home_dir() -> RvResult<PathBuf> {
        let home = std::env::var("HOME")?;
        let dir = PathBuf::from(home);
        Ok(dir)
    }

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
    pub fn is_dir<T: AsRef<Path>>(path: T) -> bool {
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
    pub fn is_file<T: AsRef<Path>>(path: T) -> bool {
        match fs::symlink_metadata(path.as_ref()) {
            Ok(x) => !x.file_type().is_symlink() && x.is_file(),
            _ => false,
        }
    }

    /// Returns true if the `Path` is empty.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Stdfs::is_empty(""), true);
    /// ```
    pub fn is_empty<T: Into<PathBuf>>(path: T) -> bool {
        path.into() == PathBuf::new()
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

    // /// Returns the last path component.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // /// use std::path::Component;
    // ///
    // /// let first = Component::Normal(OsStr::new("bar"));
    // /// assert_eq!(PathBuf::from("foo/bar").last().unwrap(), first);
    // /// ```
    // pub fn last(&self) -> RvResult<Component> {
    //     self.components().last_result()
    // }

    /// Returns a new owned [`PathBuf`] from `self` mashed together with `path`.
    /// Differs from the `join` implementation in that it drops root prefix of the given `path` if
    /// it exists and also drops any trailing '/' on the new resulting path. More closely aligns
    /// with the Golang implementation of join.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Stdfs::mash("/foo", "/bar"), PathBuf::from("/foo/bar"));
    /// ```
    pub fn mash<T: AsRef<Path>, U: AsRef<Path>>(dir: T, base: U) -> PathBuf {
        let base = Stdfs::trim_prefix(base, "/");
        let path = dir.as_ref().join(base);
        path.components().collect::<PathBuf>()
    }

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
    pub fn mkdir_m<T: AsRef<Path>>(path: T, mode: u32) -> RvResult<PathBuf> {
        let path = Stdfs::abs(path)?;

        // For each directory created apply the same permission given
        let path_str = path.to_string()?;
        let mut dir = PathBuf::from("/");
        let mut components = path_str.split('/').rev().collect::<Vec<&str>>();
        while !components.is_empty() {
            dir = Stdfs::mash(dir, components.pop().unwrap());
            if !dir.exists() {
                fs::create_dir(&dir)?;
                fs::set_permissions(&dir, fs::Permissions::from_mode(mode))?;
            }
        }
        Ok(path)
    }

    /// Creates the given directory and any parent directories needed, handling path expansion and
    /// returning an absolute path created. If the path already exists and is a dir no change is
    /// made and the path is returned.  If the path already exists and isn't a dir an error is
    /// returned.
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
    pub fn mode<T: AsRef<Path>>(path: T) -> RvResult<u32> {
        let path = Stdfs::abs(path)?;
        let meta = fs::symlink_metadata(path)?;
        Ok(meta.permissions().mode())
    }

    /// Returns the final component of the `Path` without an extension if there is one
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(StdPathBuf::from("/foo/bar.foo").name().unwrap(), "bar");
    /// ```
    pub fn name<T: AsRef<Path>>(path: T) -> RvResult<String> {
        Stdfs::base(Stdfs::trim_ext(path)?)
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
    pub fn read<T: AsRef<Path>>(path: T) -> RvResult<String> {
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
    pub fn readlink_abs<T: AsRef<Path>>(link: T) -> RvResult<PathBuf> {
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
    pub fn relative<T: AsRef<Path>, U: AsRef<Path>>(path: T, base: U) -> RvResult<PathBuf> {
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
    pub fn remove<T: AsRef<Path>>(path: T) -> RvResult<()> {
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
    pub fn remove_all<T: AsRef<Path>>(path: T) -> RvResult<()> {
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
    pub fn symlink<T: AsRef<Path>, U: AsRef<Path>>(src: T, dst: U) -> RvResult<PathBuf> {
        let src = src.as_ref().to_owned();

        // Ensure dst is rooted properly standard lookup
        let dst = Stdfs::abs(dst.as_ref())?;

        // If source is not rooted then it is already relative to the dst thus mashing the dst's
        // directory
        // to the src and cleaning it will given an absolute path.
        let src = Stdfs::abs(if !src.is_absolute() {
            Stdfs::mash(Stdfs::dir(&dst)?, src)
        } else {
            src
        })?;

        // Keep the source path relative if possible,
        let src = Stdfs::relative(src, Stdfs::dir(&dst)?)?;

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
    pub fn touch<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
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
    ///
    /// ```
    pub fn set_file_time<T: AsRef<Path>>(path: T, atime: SystemTime, mtime: SystemTime) -> RvResult<()> {
        let atime_spec = TimeSpec::from(atime.duration_since(std::time::UNIX_EPOCH)?);
        let mtime_spec = TimeSpec::from(mtime.duration_since(std::time::UNIX_EPOCH)?);
        stat::utimensat(None, path.as_ref(), &atime_spec, &mtime_spec, UtimensatFlags::NoFollowSymlink)?;
        Ok(())
    }

    /// Returns a new [`PathBuf`] with the file extension trimmed off.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("foo.exe").trim_ext().unwrap(), PathBuf::from("foo"));
    /// ```
    pub fn trim_ext<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
        let path = path.as_ref();
        Ok(match path.extension() {
            Some(val) => Stdfs::trim_suffix(path, format!(".{}", val.to_string()?)),
            None => path.to_path_buf(),
        })
    }

    /// Returns a new [`PathBuf`] with first [`Component`] trimmed off.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Stdfs::trim_first("/foo"), PathBuf::from("foo"));
    /// ```
    pub fn trim_first<T: AsRef<Path>>(path: T) -> PathBuf {
        path.as_ref().components().drop(1).as_path().to_path_buf()
    }

    // /// Returns a new [`PathBuf`] with last [`Component`] trimmed off.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // ///
    // /// assert_eq!(PathBuf::from("/foo").trim_last(), PathBuf::from("/"));
    // /// ```
    // pub fn trim_last(&self) -> PathBuf {
    //     self.components().drop(-1).as_path().to_path_buf()
    // }

    /// Returns a new [`PathBuf`] with the given prefix trimmed off else the original `path`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Stdfs::trim_prefix("/foo/bar", "/foo"), PathBuf::from("/bar"));
    /// ```
    pub fn trim_prefix<T: AsRef<Path>, U: AsRef<Path>>(path: T, prefix: U) -> PathBuf {
        let path = path.as_ref();
        match (path.to_string(), prefix.as_ref().to_string()) {
            (Ok(base), Ok(prefix)) if base.starts_with(&prefix) => {
                PathBuf::from(&base[prefix.size()..])
            },
            _ => path.to_path_buf(),
        }
    }

    /// Returns a new [`PathBuf`] with well known protocol prefixes trimmed off else the original
    /// `path`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Stdfs::trim_protocol("ftp://foo"), PathBuf::from("foo"));
    /// ```
    pub fn trim_protocol<T: AsRef<Path>>(path: T) -> PathBuf {
        let path = path.as_ref();
        match path.to_string() {
            Ok(base) => match base.find("//") {
                Some(i) => {
                    let (prefix, suffix) = base.split_at(i + 2);
                    let lower = prefix.to_lowercase();
                    let lower = lower.trim_start_matches("file://");
                    let lower = lower.trim_start_matches("ftp://");
                    let lower = lower.trim_start_matches("http://");
                    let lower = lower.trim_start_matches("https://");
                    if lower != "" {
                        PathBuf::from(format!("{}{}", prefix, suffix))
                    } else {
                        PathBuf::from(suffix)
                    }
                },
                _ => PathBuf::from(base),
            },
            _ => path.to_path_buf(),
        }
    }

    /// Returns a new [`PathBuf`] with the given `suffix` trimmed off else the original `path`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(PathBuf::from("/foo/bar").trim_suffix("/bar"), PathBuf::from("/foo"));
    /// ```
    pub fn trim_suffix<T: AsRef<Path>, U: AsRef<Path>>(path: T, suffix: U) -> PathBuf {
        let path = path.as_ref();
        match (path.to_string(), suffix.as_ref().to_string()) {
            (Ok(base), Ok(suffix)) if base.ends_with(&suffix) => {
                PathBuf::from(&base[..base.size() - suffix.size()])
            },
            _ => path.to_path_buf(),
        }
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
    ///
    /// ```
    pub fn write<T: AsRef<Path>>(path: T, data: &[u8]) -> RvResult<()> {
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

    /// Read all data from the given file and return it as a String
    fn read(&self, path: &Path) -> RvResult<String>
    {
        Stdfs::read(path)
    }

    /// Write the given data to to the indicated file creating the file first if it doesn't exist
    /// or truncating it first if it does.
    fn write(&self, path: &Path, data: &[u8]) -> RvResult<()>
    {
        Stdfs::write(path, data)
    }

    /// Up cast the trait type to the enum wrapper
    fn upcast(self) -> Vfs {
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
    fn test_stdfs_abs() -> RvResult<()> {
        let cwd = Stdfs::cwd()?;
        let prev = Stdfs::dir(&cwd)?;

        // expand relative directory
        assert_eq!(Stdfs::abs("foo")?, Stdfs::mash(&cwd, "foo"));

        // expand previous directory and drop trailing slashes
        assert_eq!(Stdfs::abs("..//")?, prev);
        assert_eq!(Stdfs::abs("../")?, prev);
        assert_eq!(Stdfs::abs("..")?, prev);

        // expand current directory and drop trailing slashes
        assert_eq!(Stdfs::abs(".//")?, cwd);
        assert_eq!(Stdfs::abs("./")?, cwd);
        assert_eq!(Stdfs::abs(".")?, cwd);

        // home dir
        let home = PathBuf::from(Stdfs::home_dir()?);
        assert_eq!(Stdfs::abs("~")?, home);
        assert_eq!(Stdfs::abs("~/")?, home);

        // expand home path
        assert_eq!(Stdfs::abs("~/foo")?, Stdfs::mash(&home, "foo"));

        // More complicated
        assert_eq!(Stdfs::abs("~/foo/bar/../.")?, Stdfs::mash(&home, "foo"));
        assert_eq!(Stdfs::abs("~/foo/bar/../")?, Stdfs::mash(&home, "foo"));
        assert_eq!(Stdfs::abs("~/foo/bar/../blah")?, Stdfs::mash(&home, "foo/blah"));

        // Move up the path multiple levels
        assert_eq!(Stdfs::abs("/foo/bar/blah/../../foo1")?, PathBuf::from("/foo/foo1"));
        assert_eq!(Stdfs::abs("/../../foo")?, PathBuf::from("/foo"));

        // Move up until invalid
        assert_eq!(Stdfs::abs("../../../../../../../foo").unwrap_err().to_string(), PathError::ParentNotFound(PathBuf::from("/")).to_string());
        Ok(())
    }

    #[test]
    fn test_stdfs_clean() {
        let tests = vec![
            // Root
            ("/", "/"),
            // Remove trailing slashes
            ("/", "//"),
            ("/", "///"),
            (".", ".//"),
            // Remove duplicates and handle rooted parent ref
            ("/", "//.."),
            ("..", "..//"),
            ("/", "/..//"),
            ("foo/bar/blah", "foo//bar///blah"),
            ("/foo/bar/blah", "/foo//bar///blah"),
            // Unneeded current dirs and duplicates
            ("/", "/.//./"),
            (".", "././/./"),
            (".", "./"),
            ("/", "/./"),
            ("foo", "./foo"),
            ("foo/bar", "./foo/./bar"),
            ("/foo/bar", "/foo/./bar"),
            ("foo/bar", "foo/bar/."),
            // Handle parent references
            ("/", "/.."),
            ("/foo", "/../foo"),
            (".", "foo/.."),
            ("../foo", "../foo"),
            ("/bar", "/foo/../bar"),
            ("foo", "foo/bar/.."),
            ("bar", "foo/../bar"),
            ("/bar", "/foo/../bar"),
            (".", "foo/bar/../../"),
            ("..", "foo/bar/../../.."),
            ("/", "/foo/bar/../../.."),
            ("/", "/foo/bar/../../../.."),
            ("../..", "foo/bar/../../../.."),
            ("blah/bar", "foo/bar/../../blah/bar"),
            ("blah", "foo/bar/../../blah/bar/.."),
            ("../foo", "../foo"),
            ("../foo", "../foo/"),
            ("../foo/bar", "../foo/bar"),
            ("..", "../foo/.."),
            ("~/foo", "~/foo"),
        ];
        for test in tests {
            assert_eq!(Stdfs::clean(test.1).unwrap(), PathBuf::from(test.0));
        }
    }

    #[test]
    fn test_stdfs_dirname() {
        assert_eq!(Stdfs::dir("/foo/").unwrap(), PathBuf::from("/").as_path(), );
        assert_eq!(Stdfs::dir("/foo/bar").unwrap(), PathBuf::from("/foo").as_path());
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

    #[test]
    fn test_stdfs_expand() -> RvResult<()>
    {
        let home = Stdfs::home_dir()?;

        // Multiple home symbols should fail
        assert_eq!(Stdfs::expand("~/~").unwrap_err().to_string(), PathError::multiple_home_symbols("~/~").to_string());

        // Only home expansion at the begining of the path is allowed
        assert_eq!(Stdfs::expand("foo/~").unwrap_err().to_string(), PathError::invalid_expansion("foo/~").to_string());

        // Tilda only
        assert_eq!(Stdfs::expand("~")?, PathBuf::from(&home));

        // Standard prefix
        assert_eq!(Stdfs::expand("~/foo")?, PathBuf::from(&home).join("foo"));

        // Variable expansion
        assert_eq!(Stdfs::expand("${HOME}")?, PathBuf::from(&home));
        assert_eq!(Stdfs::expand("${HOME}/foo")?, PathBuf::from(&home).join("foo"));
        assert_eq!(Stdfs::expand("/foo/${HOME}")?, PathBuf::from("/foo").join(&home));
        assert_eq!(Stdfs::expand("/foo/${HOME}/bar")?, PathBuf::from("/foo").join(&home).join("bar"));
        assert_eq!(Stdfs::expand("/foo${HOME}/bar")?, PathBuf::from("/foo".to_string() + &home.to_string()? + &"/bar".to_string()));
        assert_eq!(Stdfs::expand("/foo${HOME}${HOME}")?, PathBuf::from("/foo".to_string() + &home.to_string()? + &home.to_string()?));
        assert_eq!(Stdfs::expand("/foo$HOME$HOME")?, PathBuf::from("/foo".to_string() + &home.to_string()? + &home.to_string()?));
        Ok(())
    }

    #[test]
    fn test_stdfs_has_prefix() {
        let path = PathBuf::from("/foo/bar");
        assert_eq!(Stdfs::has_prefix(&path, "/foo"), true);
        assert_eq!(Stdfs::has_prefix(&path, "foo"), false);
    }

    #[test]
    fn test_stdfs_home_dir()
    {
        let home = Stdfs::home_dir().unwrap();
        assert!(home != PathBuf::new());
        assert!(home.starts_with("/"));
        assert_eq!(home.join("foo"), PathBuf::from(&home).join("foo"));
    }

    #[test]
    fn test_stdfs_is_empty()
    {
        assert_eq!(Stdfs::is_empty(""), true);
        assert_eq!(Stdfs::is_empty(Path::new("")), true);
        assert_eq!(Stdfs::is_empty("/"), false);
    }

    #[test]
    fn test_stdfs_mash()
    {
        // mashing nothing should yield no change
        assert_eq!(Stdfs::mash("", ""), PathBuf::from(""));
        assert_eq!(Stdfs::mash("/foo", ""), PathBuf::from("/foo"));

        // strips off root on path
        assert_eq!(Stdfs::mash("/foo", "/bar"), PathBuf::from("/foo/bar"));

        // strips off trailing slashes
        assert_eq!(Stdfs::mash("/foo", "bar/"), PathBuf::from("/foo/bar"));
    }

    #[test]
    fn test_stdfs_trim_first() {
        assert_eq!(Stdfs::trim_first("/"), PathBuf::new(), );
        assert_eq!(Stdfs::trim_first("/foo"), PathBuf::from("foo"));
    }

    #[test]
    fn test_stdfs_trim_prefix()
    {
        // drop root
        assert_eq!(Stdfs::trim_prefix("/", "/"), PathBuf::new());

        // drop start
        assert_eq!(Stdfs::trim_prefix("/foo/bar", "/foo"), PathBuf::from("/bar"));

        // no change
        assert_eq!(Stdfs::trim_prefix("/", ""), PathBuf::from("/"));
        assert_eq!(Stdfs::trim_prefix("/foo", "blah"), PathBuf::from("/foo"));
    }

    #[test]
    fn test_stdfs_trim_protocol()
    {
        // no change
        assert_eq!(Stdfs::trim_protocol("/foo"), PathBuf::from("/foo"));

        // file://
        assert_eq!(Stdfs::trim_protocol("file:///foo"), PathBuf::from("/foo"));

        // ftp://
        assert_eq!(Stdfs::trim_protocol("ftp://foo"), PathBuf::from("foo"));

        // http://
        assert_eq!(Stdfs::trim_protocol("http://foo"), PathBuf::from("foo"));

        // https://
        assert_eq!(Stdfs::trim_protocol("https://foo"), PathBuf::from("foo"));

        // Check case is being considered
        assert_eq!(Stdfs::trim_protocol("HTTPS://Foo"), PathBuf::from("Foo"));
        assert_eq!(Stdfs::trim_protocol("Https://Foo"), PathBuf::from("Foo"));
        assert_eq!(Stdfs::trim_protocol("HttpS://FoO"), PathBuf::from("FoO"));

        // Check non protocol matches are ignored
        assert_eq!(Stdfs::trim_protocol("foo"), PathBuf::from("foo"));
        assert_eq!(Stdfs::trim_protocol("foo/bar"), PathBuf::from("foo/bar"));
        assert_eq!(Stdfs::trim_protocol("foo//bar"), PathBuf::from("foo//bar"));
        assert_eq!(Stdfs::trim_protocol("ntp:://foo"), PathBuf::from("ntp:://foo"));
    }
}
//! Grouping path extensions here based on the concreate implementations in the sys module.

// use crate::{
//     error::*,
//     iter::*,
//     option::*,
//     path_error::*,
//     string::*,
// };
// use std::{
//     path::{Component, Path, PathBuf},
// };

// // Path extensions
// // -------------------------------------------------------------------------------------------------
// pub trait PathExt {
//     /// Return the path in an absolute clean form
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let home = user::home_dir().unwrap();
//     /// assert_eq!(PathBuf::from(&home), sys::abs("~").unwrap());
//     /// ```
//     fn abs(&self) -> RvResult<PathBuf>;

//     /// Simply a wrapper for `file_name` to return the final component of the `Path`, if there is
//     /// one else an error.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!("bar", PathBuf::from("/foo/bar").base().unwrap());
//     /// ```
//     fn base(&self) -> RvResult<String>;

//     /// Set the given mode for the `Path` and return the `Path`
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_chmod");
//     /// let file1 = tmpdir.mash("file1");
//     /// assert_mkfile!(&file1);
//     /// assert!(file1.chmod(0o644).is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100644);
//     /// assert!(file1.chmod(0o555).is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100555);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn chmod(&self, mode: u32) -> RvResult<()>;

//     /// Return the shortest path equivalent to the path by purely lexical processing and thus does
//     /// not handle links correctly in some cases, use canonicalize in those cases. It applies
//     /// the following rules interatively until no further processing can be done.
//     ///
//     /// 1. Replace multiple slashes with a single
//     /// 2. Eliminate each . path name element (the current directory)
//     /// 3. Eliminate each inner .. path name element (the parent directory)
//     ///    along with the non-.. element that precedes it.
//     /// 4. Eliminate .. elements that begin a rooted path:
//     ///    that is, replace "/.." by "/" at the beginning of a path.
//     /// 5. Leave intact ".." elements that begin a non-rooted path.
//     /// 6. Drop trailing '/' unless it is the root
//     ///
//     /// If the result of this process is an empty string, return the string `.`, representing the
//     /// current directory.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("./foo/./bar").clean().unwrap(), PathBuf::from("foo/bar"));
//     /// ```
//     fn clean(&self) -> RvResult<PathBuf>;

//     /// Returns the `Path` with the given string concatenated on without injecting
//     /// path separators.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/foo/bar").concat(".rs").unwrap(), PathBuf::from("/foo/bar.rs"));
//     /// ```
//     fn concat<T: AsRef<str>>(&self, val: T) -> RvResult<PathBuf>;

//     /// Returns the `Path` without its final component, if there is one.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let dir = PathBuf::from("/foo/bar").dir().unwrap();
//     /// assert_eq!(PathBuf::from("/foo").as_path(), dir);
//     /// ```
//     fn dir(&self) -> RvResult<PathBuf>;

//     /// Returns true if the `Path` exists. Handles path expansion.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/etc").exists(), true);
//     /// ```
//     fn exists(&self) -> bool;

//     /// Expand the path to include the home prefix if necessary
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let home = user::home_dir().unwrap();
//     /// assert_eq!(PathBuf::from(&home).mash("foo"), PathBuf::from("~/foo").expand().unwrap());
//     /// ```
//     fn expand(&self) -> RvResult<PathBuf>;

//     /// Returns the extension of the path or an error.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("foo.bar").ext().unwrap(), "bar");
//     /// ```
//     fn ext(&self) -> RvResult<String>;

//     /// Returns the first path component.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     /// use std::path::Component;
//     ///
//     /// let first = Component::Normal(OsStr::new("foo"));
//     /// assert_eq!(PathBuf::from("foo/bar").first().unwrap(), first);
//     /// ```
//     fn first(&self) -> RvResult<Component>;

//     /// Returns the group ID of the owner of this file.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/etc").gid().unwrap(), 0);
//     /// ```
//     fn gid(&self) -> RvResult<u32>;

//     /// Returns true if the `Path` contains the given path or string.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let path = PathBuf::from("/foo/bar");
//     /// assert_eq!(path.has("foo"), true);
//     /// assert_eq!(path.has("/foo"), true);
//     /// ```
//     fn has<T: AsRef<Path>>(&self, path: T) -> bool;

//     /// Returns true if the `Path` as a String has the given prefix
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let path = PathBuf::from("/foo/bar");
//     /// assert_eq!(path.has_prefix("/foo"), true);
//     /// assert_eq!(path.has_prefix("foo"), false);
//     /// ```
//     fn has_prefix<T: AsRef<Path>>(&self, prefix: T) -> bool;

//     /// Returns true if the `Path` as a String has the given suffix
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let path = PathBuf::from("/foo/bar");
//     /// assert_eq!(path.has_suffix("/bar"), true);
//     /// assert_eq!(path.has_suffix("foo"), false);
//     /// ```
//     fn has_suffix<T: AsRef<Path>>(&self, suffix: T) -> bool;

//     /// Returns true if the `Path` exists and is a directory. Handles path expansion.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/etc").is_dir(), true);
//     /// ```
//     fn is_dir(&self) -> bool;

//     /// Returns true if the `Path` is empty.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("").is_empty(), true);
//     /// ```
//     fn is_empty(&self) -> bool;

//     /// Returns true if the `Path` exists and is an executable. Handles path expansion.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_is_exec");
//     /// let file1 = tmpdir.mash("file1");
//     /// assert!(sys::mkfile_m(&file1, 0o644).is_ok());
//     /// assert_eq!(file1.is_exec(), false);
//     /// assert!(sys::chmod_b(&file1).unwrap().sym("a:a+x").exec().is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100755);
//     /// assert_eq!(file1.is_exec(), true);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn is_exec(&self) -> bool;

//     /// Returns true if the `Path` exists and is a file. Handles path expansion
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/etc/hosts").is_file(), true);
//     /// ```
//     fn is_file(&self) -> bool;

//     /// Returns true if the `Path` exists and is readonly. Handles path expansion.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_is_readonly");
//     /// let file1 = tmpdir.mash("file1");
//     /// assert!(sys::mkfile_m(&file1, 0o644).is_ok());
//     /// assert_eq!(file1.is_readonly(), false);
//     /// assert!(sys::chmod_b(&file1).unwrap().readonly().exec().is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100444);
//     /// assert_eq!(file1.is_readonly(), true);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn is_readonly(&self) -> bool;

//     /// Returns true if the `Path` exists and is a symlink. Handles path expansion
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_is_symlink");
//     /// let file1 = tmpdir.mash("file1");
//     /// let link1 = tmpdir.mash("link1");
//     /// assert_mkfile!(&file1);
//     /// assert!(sys::symlink(&file1, &link1).is_ok());
//     /// assert_eq!(link1.is_symlink(), true);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn is_symlink(&self) -> bool;

//     /// Returns true if the `Path` exists and is a symlinked directory. Handles path expansion
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_is_symlink_dir");
//     /// let dir1 = tmpdir.mash("dir1");
//     /// let link1 = tmpdir.mash("link1");
//     /// assert_mkdir!(&dir1);
//     /// assert!(sys::symlink(&dir1, &link1).is_ok());
//     /// assert_eq!(link1.is_symlink_dir(), true);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn is_symlink_dir(&self) -> bool;

//     /// Returns true if the given `Path` exists and is a symlinked file. Handles path
//     /// expansion
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_is_symlink_file");
//     /// let file1 = tmpdir.mash("file1");
//     /// let link1 = tmpdir.mash("link1");
//     /// assert_mkfile!(&file1);
//     /// assert!(sys::symlink(&file1, &link1).is_ok());
//     /// assert_eq!(link1.is_symlink_file(), true);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn is_symlink_file(&self) -> bool;

//     /// Returns the last path component.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     /// use std::path::Component;
//     ///
//     /// let first = Component::Normal(OsStr::new("bar"));
//     /// assert_eq!(PathBuf::from("foo/bar").last().unwrap(), first);
//     /// ```
//     fn last(&self) -> RvResult<Component>;

//     /// Returns a new owned [`PathBuf`] from `self` mashed together with `path`.
//     /// Differs from the `mash` implementation as `mash` drops root prefix of the given `path` if
//     /// it exists and also drops any trailing '/' on the new resulting path. More closely aligns
//     /// with the Golang implementation of join.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/foo").mash("/bar"), PathBuf::from("/foo/bar"));
//     /// ```
//     fn mash<T: AsRef<Path>>(&self, path: T) -> PathBuf;

//     /// Returns the Mode of the `Path` if it exists else and error
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_mode");
//     /// let file1 = tmpdir.mash("file1");
//     /// assert_mkfile!(&file1);
//     /// assert!(file1.chmod(0o644).is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100644);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn mode(&self) -> RvResult<u32>;

//     /// Returns the final component of the `Path` without an extension if there is one
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("/foo/bar.foo").name().unwrap(), "bar");
//     /// ```
//     fn name(&self) -> RvResult<String>;

//     /// Returns the absolute path for the link target. Handles path expansion
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_readlink");
//     /// let file1 = tmpdir.mash("file1");
//     /// let link1 = tmpdir.mash("link1");
//     /// assert_mkfile!(&file1);
//     /// assert!(sys::symlink(&file1, &link1).is_ok());
//     /// assert_eq!(link1.readlink().unwrap(), PathBuf::from("file1"));
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn readlink(&self) -> RvResult<PathBuf>;

//     /// Returns the absolute path for the given link target. Handles path expansion for
//     /// the given link. Useful for determining the absolute path of source relative to the
//     /// link rather than cwd.
//     //
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_readlink_abs");
//     /// let file1 = tmpdir.mash("file1");
//     /// let link1 = tmpdir.mash("link1");
//     /// assert_mkfile!(&file1);
//     /// assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);
//     /// assert_eq!(Stdfs::readlink_abs(link1).unwrap(), file1);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn readlink_abs(&self) -> RvResult<PathBuf>;

//     /// Returns the `Path` relative to the given `base` path. Think what is the path navigation
//     /// required to get from `base` to self. Every path used should represent a directory not a file
//     /// or link. For files or links trim off the last segement of the path before calling this
//     /// method. No attempt is made by this method to trim off the file segment.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(
//     ///     PathBuf::from("foo/bar1").relative("foo/bar2").unwrap(),
//     ///     PathBuf::from("../bar1")
//     /// );
//     /// ```
//     fn relative<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>;

//     /// Set the given [`Mode`] on the `Path` and return the `Path`
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_set_mode");
//     /// let file1 = tmpdir.mash("file1");
//     /// assert_mkfile!(&file1);
//     /// assert!(file1.chmod(0o644).is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100644);
//     /// assert!(file1.set_mode(0o555).is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100555);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn set_mode(&self, mode: u32) -> RvResult<PathBuf>;

//     /// Returns the shared path prefix between `self` and `Path`. All paths will share root `/`
//     /// so this case is being dropped to simplify detection of shared components.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("bar1").shared_prefix("bar2").unwrap(), Stdfs::cwd().unwrap());
//     /// ```
//     fn shared_prefix<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>;

//     /// Returns a new [`PathBuf`] with the file extension trimmed off.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("foo.exe").trim_ext().unwrap(), PathBuf::from("foo"));
//     /// ```
//     fn trim_ext(&self) -> RvResult<PathBuf>;

//     /// Returns a new [`PathBuf`] with first [`Component`] trimmed off.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("/foo").trim_first(), PathBuf::from("foo"));
//     /// ```
//     fn trim_first(&self) -> PathBuf;

//     /// Returns a new [`PathBuf`] with last [`Component`] trimmed off.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("/foo").trim_last(), PathBuf::from("/"));
//     /// ```
//     fn trim_last(&self) -> PathBuf;

//     /// Returns a new [`PathBuf`] with the given prefix trimmed off else the original `path`.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/foo/bar").trim_prefix("/foo"), PathBuf::from("/bar"));
//     /// ```
//     fn trim_prefix<T: AsRef<Path>>(&self, prefix: T) -> PathBuf;

//     /// Returns a new [`PathBuf`] with well known protocol prefixes trimmed off else the original
//     /// `path`.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("ftp://foo").trim_protocol(), PathBuf::from("foo"));
//     /// ```
//     fn trim_protocol(&self) -> PathBuf;

//     /// Returns a new [`PathBuf`] with the given `suffix` trimmed off else the original `path`.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("/foo/bar").trim_suffix("/bar"), PathBuf::from("/foo"));
//     /// ```
//     fn trim_suffix<T: AsRef<Path>>(&self, suffix: T) -> PathBuf;

//     /// Returns the user ID of the owner of this file.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/etc").uid().unwrap(), 0);
//     /// ```
//     fn uid(&self) -> RvResult<u32>;
// }

// impl PathExt for Path {
//     /// Return the path in an absolute clean form
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let home = user::home_dir().unwrap();
//     /// assert_eq!(PathBuf::from(&home), sys::abs("~").unwrap());
//     /// ```
//     fn abs(&self) -> RvResult<PathBuf> {
//         sys::abs(self)
//     }

//     /// Returns the final component of the `Path`, if there is one.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!("bar", PathBuf::from("/foo/bar").base().unwrap());
//     /// ```
//     fn base(&self) -> RvResult<String> {
//         self.file_name().ok_or_else(|| PathError::filename_not_found(self))?.to_string()
//     }

//     /// Set the given mode for the `Path` and return the `Path`
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_chmod");
//     /// let file1 = tmpdir.mash("file1");
//     /// assert_mkfile!(&file1);
//     /// assert!(file1.chmod(0o644).is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100644);
//     /// assert!(file1.chmod(0o555).is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100555);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn chmod(&self, mode: u32) -> RvResult<()> {
//         sys::chmod(self, mode)?;
//         Ok(())
//     }

//     /// Return the shortest path equivalent to the path by purely lexical processing and thus does
//     /// not handle links correctly in some cases, use canonicalize in those cases. It applies
//     /// the following rules interatively until no further processing can be done.
//     ///
//     /// 1. Replace multiple slashes with a single
//     /// 2. Eliminate each . path name element (the current directory)
//     /// 3. Eliminate each inner .. path name element (the parent directory)
//     ///    along with the non-.. element that precedes it.
//     /// 4. Eliminate .. elements that begin a rooted path:
//     ///    that is, replace "/.." by "/" at the beginning of a path.
//     /// 5. Leave intact ".." elements that begin a non-rooted path.
//     /// 6. Drop trailing '/' unless it is the root
//     ///
//     /// If the result of this process is an empty string, return the string `.`, representing the
//     /// current directory.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("./foo/./bar").clean().unwrap(), PathBuf::from("foo/bar"));
//     /// ```
//     fn clean(&self) -> RvResult<PathBuf> {
//         // Components already handles the following cases:
//         // 1. Repeated separators are ignored, so a/b and a//b both have a and b as components.
//         // 2. Occurrences of . are normalized away, except if they are at the beginning of the path.
//         //    e.g. a/./b, a/b/, a/b/. and a/b all have a and b as components, but ./a/b starts with
//         // an
//         // additional CurDir component.
//         // 6. A trailing slash is normalized away, /a/b and /a/b/ are equivalent.
//         let mut cnt = 0;
//         let mut prev = None;
//         let mut path_buf = PathBuf::new();
//         for component in self.components() {
//             match component {
//                 // 2. Eliminate . path name at begining of path for simplicity
//                 x if x == Component::CurDir && cnt == 0 => continue,

//                 // 5. Leave .. begining non rooted path
//                 x if x == Component::ParentDir && cnt > 0 && !prev.has(Component::ParentDir) => {
//                     match prev.unwrap() {
//                         // 4. Eliminate .. elements that begin a root path
//                         Component::RootDir => {},

//                         // 3. Eliminate inner .. path name elements
//                         Component::Normal(_) => {
//                             cnt -= 1;
//                             path_buf.pop();
//                             prev = path_buf.components().last();
//                         },
//                         _ => {},
//                     }
//                     continue;
//                 },

//                 // Normal
//                 _ => {
//                     cnt += 1;
//                     path_buf.push(component);
//                     prev = Some(component);
//                 },
//             };
//         }

//         // Ensure if empty the current dir is returned
//         if path_buf.is_empty() {
//             path_buf.push(".");
//         }
//         Ok(path_buf)
//     }

//     /// Returns the `Path` with the given string concatenated on without injecting
//     /// path separators.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/foo/bar").concat(".rs").unwrap(), PathBuf::from("/foo/bar.rs"));
//     /// ```
//     fn concat<T: AsRef<str>>(&self, val: T) -> RvResult<PathBuf> {
//         Ok(PathBuf::from(format!("{}{}", self.to_string()?, val.as_ref())))
//     }

//     /// Returns the `Path` without its final component, if there is one.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let dir = PathBuf::from("/foo/bar").dir().unwrap();
//     /// assert_eq!(PathBuf::from("/foo").as_path(), dir);
//     /// ```
//     fn dir(&self) -> RvResult<PathBuf> {
//         let dir = self.parent().ok_or_else(|| PathError::parent_not_found(self))?;
//         Ok(dir.to_path_buf())
//     }

//     /// Returns true if the `Path` exists. Handles path expansion.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/etc").exists(), true);
//     /// ```
//     fn exists(&self) -> bool {
//         sys::exists(&self)
//     }

//     /// Expand the path to include the home prefix if necessary
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let home = user::home_dir().unwrap();
//     /// assert_eq!(PathBuf::from(&home).mash("foo"), PathBuf::from("~/foo").expand().unwrap());
//     /// ```
//     fn expand(&self) -> RvResult<PathBuf> {
//         sys::expand(&self)
//     }

//     /// Returns the extension of the path or an error.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("foo.bar").ext().unwrap(), "bar");
//     /// ```
//     fn ext(&self) -> RvResult<String> {
//         match self.extension() {
//             Some(val) => val.to_string(),
//             None => Err(PathError::extension_not_found(self).into()),
//         }
//     }

//     /// Returns the first path component.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     /// use std::path::Component;
//     ///
//     /// let first = Component::Normal(OsStr::new("foo"));
//     /// assert_eq!(PathBuf::from("foo/bar").first().unwrap(), first);
//     /// ```
//     fn first(&self) -> RvResult<Component> {
//         self.components().first_result()
//     }

//     /// Returns the group ID of the owner of this file.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/etc").gid().unwrap(), 0);
//     /// ```
//     fn gid(&self) -> RvResult<u32> {
//         sys::gid(&self)
//     }

//     /// Returns true if the `Path` contains the given path or string.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let path = PathBuf::from("/foo/bar");
//     /// assert_eq!(path.has("foo"), true);
//     /// assert_eq!(path.has("/foo"), true);
//     /// ```
//     fn has<T: AsRef<Path>>(&self, path: T) -> bool {
//         match (self.to_string(), path.as_ref().to_string()) {
//             (Ok(base), Ok(path)) => base.contains(&path),
//             _ => false,
//         }
//     }

//     /// Returns true if the `Path` as a String has the given prefix
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let path = PathBuf::from("/foo/bar");
//     /// assert_eq!(path.has_prefix("/foo"), true);
//     /// assert_eq!(path.has_prefix("foo"), false);
//     /// ```
//     fn has_prefix<T: AsRef<Path>>(&self, prefix: T) -> bool {
//         match (self.to_string(), prefix.as_ref().to_string()) {
//             (Ok(base), Ok(prefix)) => base.starts_with(&prefix),
//             _ => false,
//         }
//     }

//     /// Returns true if the `Path` as a String has the given suffix
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// let path = PathBuf::from("/foo/bar");
//     /// assert_eq!(path.has_suffix("/bar"), true);
//     /// assert_eq!(path.has_suffix("foo"), false);
//     /// ```
//     fn has_suffix<T: AsRef<Path>>(&self, suffix: T) -> bool {
//         match (self.to_string(), suffix.as_ref().to_string()) {
//             (Ok(base), Ok(suffix)) => base.ends_with(&suffix),
//             _ => false,
//         }
//     }

//     /// Returns true if the `Path` exists and is a directory. Handles path expansion.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/etc").is_dir(), true);
//     /// ```
//     fn is_dir(&self) -> bool {
//         sys::is_dir(self)
//     }

//     /// Returns true if the `Path` is empty.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("").is_empty(), true);
//     /// ```
//     fn is_empty(&self) -> bool {
//         self == PathBuf::new()
//     }

//     /// Returns true if the `Path` exists and is an executable. Handles path expansion.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_is_exec");
//     /// let file1 = tmpdir.mash("file1");
//     /// assert!(sys::mkfile_m(&file1, 0o644).is_ok());
//     /// assert_eq!(file1.is_exec(), false);
//     /// assert!(sys::chmod_b(&file1).unwrap().sym("a:a+x").exec().is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100755);
//     /// assert_eq!(file1.is_exec(), true);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn is_exec(&self) -> bool {
//         sys::is_exec(self)
//     }

//     /// Returns true if the `Path` exists and is a file. Handles path expansion
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/etc/hosts").is_file(), true);
//     /// ```
//     fn is_file(&self) -> bool {
//         sys::is_file(self)
//     }

//     /// Returns true if the `Path` exists and is readonly. Handles path expansion.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_is_readonly");
//     /// let file1 = tmpdir.mash("file1");
//     /// assert!(sys::mkfile_m(&file1, 0o644).is_ok());
//     /// assert_eq!(file1.is_readonly(), false);
//     /// assert!(sys::chmod_b(&file1).unwrap().readonly().exec().is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100444);
//     /// assert_eq!(file1.is_readonly(), true);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn is_readonly(&self) -> bool {
//         sys::is_readonly(self)
//     }

//     /// Returns true if the `Path` exists and is a symlink. Handles path expansion
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_is_symlink");
//     /// let file1 = tmpdir.mash("file1");
//     /// let link1 = tmpdir.mash("link1");
//     /// assert_mkfile!(&file1);
//     /// assert!(sys::symlink(&file1, &link1).is_ok());
//     /// assert_eq!(link1.is_symlink(), true);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn is_symlink(&self) -> bool {
//         sys::is_symlink(self)
//     }

//     /// Returns true if the `Path` exists and is a symlinked directory. Handles path expansion
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_is_symlink_dir");
//     /// let dir1 = tmpdir.mash("dir1");
//     /// let link1 = tmpdir.mash("link1");
//     /// assert_mkdir!(&dir1);
//     /// assert!(sys::symlink(&dir1, &link1).is_ok());
//     /// assert_eq!(link1.is_symlink_dir(), true);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn is_symlink_dir(&self) -> bool {
//         sys::is_symlink_dir(self)
//     }

//     /// Returns true if the given `Path` exists and is a symlinked file. Handles path
//     /// expansion
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_is_symlink_file");
//     /// let file1 = tmpdir.mash("file1");
//     /// let link1 = tmpdir.mash("link1");
//     /// assert_mkfile!(&file1);
//     /// assert!(sys::symlink(&file1, &link1).is_ok());
//     /// assert_eq!(link1.is_symlink_file(), true);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn is_symlink_file(&self) -> bool {
//         sys::is_symlink_file(self)
//     }

//     /// Returns the last path component.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     /// use std::path::Component;
//     ///
//     /// let first = Component::Normal(OsStr::new("bar"));
//     /// assert_eq!(PathBuf::from("foo/bar").last().unwrap(), first);
//     /// ```
//     fn last(&self) -> RvResult<Component> {
//         self.components().last_result()
//     }

//     /// Returns a new owned [`PathBuf`] from `self` mashed together with `path`.
//     /// Differs from the `mash` implementation as `mash` drops root prefix of the given `path` if
//     /// it exists and also drops any trailing '/' on the new resulting path. More closely aligns
//     /// with the Golang implementation of join.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/foo").mash("/bar"), PathBuf::from("/foo/bar"));
//     /// ```
//     fn mash<T: AsRef<Path>>(&self, path: T) -> PathBuf {
//         mash(self, path)
//     }

//     /// Returns the Mode of the `Path` if it exists else and error
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_mode");
//     /// let file1 = tmpdir.mash("file1");
//     /// assert_mkfile!(&file1);
//     /// assert!(file1.chmod(0o644).is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100644);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn mode(&self) -> RvResult<u32> {
//         sys::mode(self)
//     }

//     /// Returns the final component of the `Path` without an extension if there is one
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("/foo/bar.foo").name().unwrap(), "bar");
//     /// ```
//     fn name(&self) -> RvResult<String> {
//         self.trim_ext()?.base()
//     }

//     /// Returns the absolute path for the link target. Handles path expansion
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_readlink");
//     /// let file1 = tmpdir.mash("file1");
//     /// let link1 = tmpdir.mash("link1");
//     /// assert_mkfile!(&file1);
//     /// assert!(sys::symlink(&file1, &link1).is_ok());
//     /// assert_eq!(link1.readlink().unwrap(), PathBuf::from("file1"));
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn readlink(&self) -> RvResult<PathBuf> {
//         sys::readlink(self)
//     }

//     /// Returns the absolute path for the given link target. Handles path expansion for
//     /// the given link. Useful for determining the absolute path of source relative to the
//     /// link rather than cwd.
//     //
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_readlink_abs");
//     /// let file1 = tmpdir.mash("file1");
//     /// let link1 = tmpdir.mash("link1");
//     /// assert_mkfile!(&file1);
//     /// assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);
//     /// assert_eq!(Stdfs::readlink_abs(link1).unwrap(), file1);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn readlink_abs(&self) -> RvResult<PathBuf> {
//         sys::readlink_abs(self)
//     }

//     /// Returns the `Path` relative to the given `base` path. Think what is the path navigation
//     /// required to get from `base` to self. Every path used should represent a directory not a file
//     /// or link. For files or links trim off the last segement of the path before calling this
//     /// method. No attempt is made by this method to trim off the file segment.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(
//     ///     PathBuf::from("foo/bar1").relative("foo/bar2").unwrap(),
//     ///     PathBuf::from("../bar1")
//     /// );
//     /// ```
//     fn relative<T: AsRef<Path>>(&self, base: T) -> RvResult<PathBuf> {
//         let path = self.abs()?;
//         let base = base.as_ref().abs()?;
//         if path != base {
//             let mut x = path.components();
//             let mut y = base.components();
//             let mut comps: Vec<Component> = vec![];
//             loop {
//                 match (x.next(), y.next()) {
//                     // nothing were done
//                     (None, None) => break,

//                     // base is ahead one
//                     (None, _) => comps.push(Component::ParentDir),

//                     // self is ahead the remaining
//                     (Some(a), None) => {
//                         comps.push(a);
//                         comps.extend(x.by_ref());
//                         break;
//                     },

//                     // both components are the same and we haven't processed anything yet skip it
//                     (Some(a), Some(b)) if comps.is_empty() && a == b => continue,

//                     // any additional components in the base need to be backed tracked from self
//                     (Some(a), Some(_)) => {
//                         // backtrack the current component and all remaining ones
//                         comps.push(Component::ParentDir);
//                         for _ in y {
//                             comps.push(Component::ParentDir);
//                         }

//                         // now include the current self and all remaining components
//                         comps.push(a);
//                         comps.extend(x.by_ref());
//                         break;
//                     },
//                 }
//             }
//             return Ok(comps.iter().collect::<PathBuf>());
//         }
//         Ok(path)
//     }

//     /// Set the given [`Mode`] on the `Path` and return the `Path`
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_setup_func!();
//     /// let tmpdir = assert_setup!("pathext_trait_set_mode");
//     /// let file1 = tmpdir.mash("file1");
//     /// assert_mkfile!(&file1);
//     /// assert!(file1.chmod(0o644).is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100644);
//     /// assert!(file1.set_mode(0o555).is_ok());
//     /// assert_eq!(file1.mode().unwrap(), 0o100555);
//     /// assert_remove_all!(&tmpdir);
//     /// ```
//     fn set_mode(&self, mode: u32) -> RvResult<PathBuf> {
//         sys::set_mode(self, mode)?;
//         Ok(self.to_path_buf())
//     }

//     /// Returns the shared path prefix between `self` and `Path`. All paths will share root `/`
//     /// so this case is being dropped to simplify detection of shared components.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("bar1").shared_prefix("bar2").unwrap(), Stdfs::cwd().unwrap());
//     /// ```
//     fn shared_prefix<T: AsRef<Path>>(&self, base: T) -> RvResult<PathBuf> {
//         let path = self.abs()?;
//         let base = base.as_ref().abs()?;
//         if path != base {
//             let mut x = path.components();
//             let mut y = base.components();
//             let mut comps: Vec<Component> = vec![];
//             loop {
//                 match (x.next(), y.next()) {
//                     (Some(a), Some(b)) if a == b => comps.push(a),
//                     (..) => break,
//                 }
//             }

//             // If all that is shared is the root then drop it to help detect this case better
//             if comps.len() == 1 {
//                 if let Some(x) = comps.first() {
//                     if x == &Component::RootDir {
//                         comps.remove(0);
//                     }
//                 }
//             }

//             return Ok(comps.iter().collect::<PathBuf>());
//         }
//         Ok(path)
//     }

//     /// Returns a new [`PathBuf`] with the file extension trimmed off.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("foo.exe").trim_ext().unwrap(), PathBuf::from("foo"));
//     /// ```
//     fn trim_ext(&self) -> RvResult<PathBuf> {
//         Ok(match self.extension() {
//             Some(val) => self.trim_suffix(format!(".{}", val.to_string()?)),
//             None => self.to_path_buf(),
//         })
//     }

//     /// Returns a new [`PathBuf`] with first [`Component`] trimmed off.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("/foo").trim_first(), PathBuf::from("foo"));
//     /// ```
//     fn trim_first(&self) -> PathBuf {
//         self.components().drop(1).as_path().to_path_buf()
//     }

//     /// Returns a new [`PathBuf`] with last [`Component`] trimmed off.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("/foo").trim_last(), PathBuf::from("/"));
//     /// ```
//     fn trim_last(&self) -> PathBuf {
//         self.components().drop(-1).as_path().to_path_buf()
//     }

//     /// Returns a new [`PathBuf`] with the given prefix trimmed off else the original `path`.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/foo/bar").trim_prefix("/foo"), PathBuf::from("/bar"));
//     /// ```
//     fn trim_prefix<T: AsRef<Path>>(&self, prefix: T) -> PathBuf {
//         match (self.to_string(), prefix.as_ref().to_string()) {
//             (Ok(base), Ok(prefix)) if base.starts_with(&prefix) => {
//                 PathBuf::from(&base[prefix.size()..])
//             },
//             _ => self.to_path_buf(),
//         }
//     }

//     /// Returns a new [`PathBuf`] with well known protocol prefixes trimmed off else the original
//     /// `path`.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("ftp://foo").trim_protocol(), PathBuf::from("foo"));
//     /// ```
//     fn trim_protocol(&self) -> PathBuf {
//         match self.to_string() {
//             Ok(base) => match base.find("//") {
//                 Some(i) => {
//                     let (prefix, suffix) = base.split_at(i + 2);
//                     let lower = prefix.to_lowercase();
//                     let lower = lower.trim_start_matches("file://");
//                     let lower = lower.trim_start_matches("ftp://");
//                     let lower = lower.trim_start_matches("http://");
//                     let lower = lower.trim_start_matches("https://");
//                     if lower != "" {
//                         PathBuf::from(format!("{}{}", prefix, suffix))
//                     } else {
//                         PathBuf::from(suffix)
//                     }
//                 },
//                 _ => PathBuf::from(base),
//             },
//             _ => self.to_path_buf(),
//         }
//     }

//     /// Returns a new [`PathBuf`] with the given `suffix` trimmed off else the original `path`.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(PathBuf::from("/foo/bar").trim_suffix("/bar"), PathBuf::from("/foo"));
//     /// ```
//     fn trim_suffix<T: AsRef<Path>>(&self, suffix: T) -> PathBuf {
//         match (self.to_string(), suffix.as_ref().to_string()) {
//             (Ok(base), Ok(suffix)) if base.ends_with(&suffix) => {
//                 PathBuf::from(&base[..base.size() - suffix.size()])
//             },
//             _ => self.to_path_buf(),
//         }
//     }

//     /// Returns the user ID of the owner of this file.
//     ///
//     /// ### Examples
//     /// ```
//     /// use rivia::*;
//     ///
//     /// assert_eq!(Path::new("/etc").uid().unwrap(), 0);
//     /// ```
//     fn uid(&self) -> RvResult<u32> {
//         sys::uid(&self)
//     }
// }
// WARNING: Only those functions that are filesystem agnostic should be included here.
use std::path::{self, Component, Path, PathBuf};

use crate::{core::*, errors::*};

/// Returns the final component of the given `path` if there is one
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::base("/foo/bar").unwrap(), "bar".to_string());
/// ```
pub fn base<T: AsRef<Path>>(path: T) -> RvResult<String> {
    path.as_ref().components().last_result()?.to_string()
}

/// Return the shortest equivalent to the given `path` by purely lexical processing
///
/// * Purely lexical processing may not handle links correctly in some cases, use `canonicalize` in
/// those cases
///
/// ### Algorithm
/// Applies the following rules interatively until no further processing can be done.
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
/// assert_eq!(sys::clean("./foo/./bar"), PathBuf::from("foo/bar"));
/// ```
pub fn clean<T: AsRef<Path>>(path: T) -> PathBuf {
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
    if is_empty(&path_buf) {
        path_buf.push(".");
    }
    path_buf
}

/// Returns the `Path` with the given string concatenated on without injecting
/// path separators.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::concat("/foo/bar", ".rs").unwrap(), PathBuf::from("/foo/bar.rs"));
/// ```
pub fn concat<T: AsRef<Path>, U: AsRef<str>>(path: T, val: U) -> RvResult<PathBuf> {
    Ok(PathBuf::from(format!("{}{}", path.as_ref().to_string()?, val.as_ref())))
}

/// Returns the given `path` without its final component if there is one.
///
/// ### Errors
/// * PathError:ParentNotFound if the path has no parent
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::dir("/foo/bar").unwrap(), PathBuf::from("/foo").as_path());
/// ```
pub fn dir<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
    let path = path.as_ref();
    let dir = path.parent().ok_or_else(|| PathError::parent_not_found(path))?;
    Ok(dir.to_path_buf())
}

/// Expand home variable `~` and all environment variables in the path
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let home = sys::home_dir().unwrap();
/// assert_eq!(sys::expand("~/foo").unwrap(), PathBuf::from(&home).join("foo"));
/// assert_eq!(sys::expand("$HOME/foo").unwrap(), PathBuf::from(&home).join("foo"));
/// assert_eq!(sys::expand("${HOME}/foo").unwrap(), PathBuf::from(&home).join("foo"));
/// ```
pub fn expand<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
    let path = path.as_ref();
    let pathstr = path.to_string()?;

    // Expand home directory
    let path = match pathstr.matches('~').count() {
        // Only a single home expansion is allowed
        cnt if cnt > 1 => return Err(PathError::multiple_home_symbols(path).into()),

        // Home expansion only makes sense at the beinging of a path
        cnt if cnt == 1 && !has_prefix(path, "~/") && pathstr != "~" => {
            return Err(PathError::invalid_expansion(path).into())
        },

        // Single tilda only
        cnt if cnt == 1 && pathstr == "~" => home_dir()?,

        // Replace prefix with home directory
        1 => mash(home_dir()?, &pathstr[2..]),
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

                    while chars.peek().is_some() {
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

/// Returns the extension of the path or an error.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::ext("foo.bar").unwrap(), "bar");
/// ```
pub fn ext<T: AsRef<Path>>(path: T) -> RvResult<String> {
    match path.as_ref().extension() {
        Some(val) => val.to_string(),
        None => Err(PathError::extension_not_found(path).into()),
    }
}

/// Returns the first path component.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::first("foo/bar").unwrap(), "foo".to_string());
/// ```
pub fn first<T: AsRef<Path>>(path: T) -> RvResult<String> {
    path.as_ref().components().first_result()?.to_string()
}

/// Returns the final component of the `Path` without an extension if there is one
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::name("/foo/bar.foo").unwrap(), "bar");
/// ```
pub fn name<T: AsRef<Path>>(path: T) -> RvResult<String> {
    base(trim_ext(path)?)
}

/// Returns true if the `Path` contains the given path or string.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let path = PathBuf::from("/foo/bar");
/// assert_eq!(sys::has(&path, "foo"), true);
/// assert_eq!(sys::has(&path, "/foo"), true);
/// ```
pub fn has<T: AsRef<Path>, U: AsRef<Path>>(path: T, val: U) -> bool {
    match (path.as_ref().to_string(), val.as_ref().to_string()) {
        (Ok(base), Ok(path)) => base.contains(&path),
        _ => false,
    }
}

/// Returns true if the `Path` as a String has the given prefix
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let path = PathBuf::from("/foo/bar");
/// assert_eq!(sys::has_prefix(&path, "/foo"), true);
/// assert_eq!(sys::has_prefix(&path, "foo"), false);
/// ```
pub fn has_prefix<T: AsRef<Path>, U: AsRef<Path>>(path: T, prefix: U) -> bool {
    match (path.as_ref().to_string(), prefix.as_ref().to_string()) {
        (Ok(base), Ok(prefix)) => base.starts_with(&prefix),
        _ => false,
    }
}

/// Returns true if the `Path` as a String has the given suffix
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let path = PathBuf::from("/foo/bar");
/// assert_eq!(sys::has_suffix(&path, "/bar"), true);
/// assert_eq!(sys::has_suffix(&path, "foo"), false);
/// ```
pub fn has_suffix<T: AsRef<Path>, U: AsRef<Path>>(path: T, suffix: U) -> bool {
    match (path.as_ref().to_string(), suffix.as_ref().to_string()) {
        (Ok(base), Ok(suffix)) => base.ends_with(&suffix),
        _ => false,
    }
}

/// Returns the full path to the current user's home directory.
///
/// Alternate implementation as the Rust std::env::home_dir implementation has been
/// deprecated <https://doc.rust-lang.org/std/env/fn.home_dir.html>
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(sys::home_dir().is_ok());
/// ```
pub fn home_dir() -> RvResult<PathBuf> {
    let home = std::env::var("HOME")?;
    let dir = PathBuf::from(home);
    Ok(dir)
}

/// Returns true if the `Path` is empty.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::is_empty(""), true);
/// ```
pub fn is_empty<T: Into<PathBuf>>(path: T) -> bool {
    path.into() == PathBuf::new()
}

/// Returns the last path component. Alias to `base`
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::last("foo/bar").unwrap(), "bar".to_string());
/// ```
pub fn last<T: AsRef<Path>>(path: T) -> RvResult<String> {
    base(path)
}

/// Returns a new owned [`PathBuf`] mashed together with the given `path`
///
/// ### Safer implementation than `join` in that it more closely aligns with Golang's implementation
/// * Drops the root prefix of the given `path` if it exists unlike `join`
/// * Drops any trailing separator e.g. `/`
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::mash("/foo", "/bar"), PathBuf::from("/foo/bar"));
/// ```
pub fn mash<T: AsRef<Path>, U: AsRef<Path>>(dir: T, base: U) -> PathBuf {
    let base = trim_prefix(base, path::MAIN_SEPARATOR.to_string());
    let path = dir.as_ref().join(base);
    path.components().collect::<PathBuf>()
}

/// Parse unix shell pathing e.g. $PATH, $XDG_DATA_DIRS or $XDG_CONFIG_DIRS
///
/// * Splits a given colon delimited value into a list
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let paths = vec![PathBuf::from("/foo1"), PathBuf::from("/foo2/bar")];
/// assert_iter_eq(sys::parse_paths("/foo1:/foo2/bar").unwrap(), paths);
/// ```
pub fn parse_paths<T: AsRef<str>>(value: T) -> RvResult<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = vec![];
    for dir in value.as_ref().split(':') {
        // Ignoring - Unix shell semantics: path element "" means "."
        if dir != "" {
            paths.push(PathBuf::from(dir));
        }
    }
    Ok(paths)
}

/// Returns the `Path` relative to the given `base` path
///
/// Think what is the path navigation required to get from `base` to `path`. Every path used should
/// represent a directory not a file or link. For files or links trim off the last segement of the
/// path before calling this method. No attempt is made by this method to trim off the file segment.
///
/// ### Arguments
/// * `path` - path to return the navigation relative to base for, expected to be in absolute form
/// * `base` - path to calculate navigation from, expected to be in absolute form
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::relative("foo/bar1", "foo/bar2").unwrap(), PathBuf::from("../bar1"));
/// ```
pub fn relative<T: AsRef<Path>, U: AsRef<Path>>(path: T, base: U) -> RvResult<PathBuf> {
    let path = path.as_ref();
    let base = base.as_ref();
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
    Ok(path.to_owned())
}

/// Returns a new [`PathBuf`] with the file extension trimmed off.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::trim_ext("foo.exe").unwrap(), PathBuf::from("foo"));
/// ```
pub fn trim_ext<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
    let path = path.as_ref();
    Ok(match path.extension() {
        Some(val) => trim_suffix(path, format!(".{}", val.to_string()?)),
        None => path.to_path_buf(),
    })
}

/// Returns a new [`PathBuf`] with first [`Component`] trimmed off.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::trim_first("/foo"), PathBuf::from("foo"));
/// ```
pub fn trim_first<T: AsRef<Path>>(path: T) -> PathBuf {
    path.as_ref().components().drop(1).as_path().to_path_buf()
}

/// Returns a new [`PathBuf`] with last [`Component`] trimmed off.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::trim_last("/foo"), PathBuf::from("/"));
/// ```
pub fn trim_last<T: AsRef<Path>>(path: T) -> PathBuf {
    path.as_ref().components().drop(-1).as_path().to_path_buf()
}

/// Returns a new [`PathBuf`] with the given prefix trimmed off else the original `path`.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::trim_prefix("/foo/bar", "/foo"), PathBuf::from("/bar"));
/// ```
pub fn trim_prefix<T: AsRef<Path>, U: AsRef<Path>>(path: T, prefix: U) -> PathBuf {
    let path = path.as_ref();
    match (path.to_string(), prefix.as_ref().to_string()) {
        (Ok(base), Ok(prefix)) if base.starts_with(&prefix) => PathBuf::from(&base[prefix.size()..]),
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
/// assert_eq!(sys::trim_protocol("ftp://foo"), PathBuf::from("foo"));
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
/// assert_eq!(sys::trim_suffix("/foo/bar", "/bar"), PathBuf::from("/foo"));
/// ```
pub fn trim_suffix<T: AsRef<Path>, U: AsRef<Path>>(path: T, suffix: U) -> PathBuf {
    let path = path.as_ref();
    match (path.to_string(), suffix.as_ref().to_string()) {
        (Ok(base), Ok(suffix)) if base.ends_with(&suffix) => PathBuf::from(&base[..base.size() - suffix.size()]),
        _ => path.to_path_buf(),
    }
}

/// Defines additional pathing extension functions for [`Path`] and [`PathBuf`]
pub trait PathExt {
    /// Simply a wrapper for `file_name` to return the final component of the `Path`, if there is
    /// one else an error.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar").base().unwrap(), "bar".to_string());
    /// ```
    fn base(&self) -> RvResult<String>;

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
    /// assert_eq!(Path::new("./foo/./bar").clean(), PathBuf::from("foo/bar"));
    /// ```
    fn clean(&self) -> PathBuf;

    /// Returns the `Path` with the given string concatenated on without injecting
    /// path separators.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar").concat(".rs").unwrap(), PathBuf::from("/foo/bar.rs"));
    /// ```
    fn concat<T: AsRef<str>>(&self, val: T) -> RvResult<PathBuf>;

    /// Returns the `Path` without its final component, if there is one.
    ///
    /// ### Errors
    /// * PathError:ParentNotFound if the path has no parent
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar").dir().unwrap(), PathBuf::from("/foo"));
    /// ```
    fn dir(&self) -> RvResult<PathBuf>;

    /// Expand the path to include the home prefix if necessary
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(Path::new("~/foo").expand().unwrap(), PathBuf::from(&home).mash("foo"));
    /// ```
    fn expand(&self) -> RvResult<PathBuf>;

    /// Returns the extension of the path or an error.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("foo.bar").ext().unwrap(), "bar");
    /// ```
    fn ext(&self) -> RvResult<String>;

    /// Returns the first path component.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// use std::path::Component;
    ///
    /// assert_eq!(Path::new("foo/bar").first().unwrap(), "foo".to_string());
    /// ```
    fn first(&self) -> RvResult<String>;

    /// Returns true if the `Path` contains the given path or string.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let path = PathBuf::from("/foo/bar");
    /// assert_eq!(path.has("foo"), true);
    /// assert_eq!(path.has("/foo"), true);
    /// ```
    fn has<T: AsRef<Path>>(&self, path: T) -> bool;

    /// Returns true if the `Path` as a String has the given prefix
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let path = PathBuf::from("/foo/bar");
    /// assert_eq!(path.has_prefix("/foo"), true);
    /// assert_eq!(path.has_prefix("foo"), false);
    /// ```
    fn has_prefix<T: AsRef<Path>>(&self, prefix: T) -> bool;

    /// Returns true if the `Path` as a String has the given suffix
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let path = PathBuf::from("/foo/bar");
    /// assert_eq!(path.has_suffix("/bar"), true);
    /// assert_eq!(path.has_suffix("foo"), false);
    /// ```
    fn has_suffix<T: AsRef<Path>>(&self, suffix: T) -> bool;

    /// Returns true if the `Path` is empty.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(PathBuf::from("").is_empty(), true);
    /// ```
    fn is_empty(&self) -> bool;

    /// Returns the last component of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("foo/bar").last().unwrap(), "bar".to_string());
    /// ```
    fn last(&self) -> RvResult<String>;

    /// Returns a new owned [`PathBuf`] from `self` mashed together with `path`.
    /// Differs from the `mash` implementation as `mash` drops root prefix of the given `path` if
    /// it exists and also drops any trailing '/' on the new resulting path. More closely aligns
    /// with the Golang implementation of join.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo").mash("/bar"), PathBuf::from("/foo/bar"));
    /// ```
    fn mash<T: AsRef<Path>>(&self, path: T) -> PathBuf;

    /// Returns the final component of the `Path` without an extension if there is one
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar.foo").name().unwrap(), "bar");
    /// ```
    fn name(&self) -> RvResult<String>;

    /// Returns the `Path` relative to the given `base` path
    ///
    /// Think what is the path navigation required to get from `base` to `path`. Every path used
    /// should represent a directory not a file or link. For files or links trim off the last
    /// segement of the path before calling this method. No attempt is made by this method to
    /// trim off the file segment.
    ///
    /// ### Arguments
    /// * `path` - path to return the navigation relative to base for, expected to be in absolute
    ///   form
    /// * `base` - path to calculate navigation from, expected to be in absolute form
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("foo/bar1").relative("foo/bar2").unwrap(), PathBuf::from("../bar1"));
    /// ```
    fn relative<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>;

    /// Returns a new [`PathBuf`] with the file extension trimmed off.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("foo.exe").trim_ext().unwrap(), PathBuf::from("foo"));
    /// ```
    fn trim_ext(&self) -> RvResult<PathBuf>;

    /// Returns a new [`PathBuf`] with first [`Component`] trimmed off.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo").trim_first(), PathBuf::from("foo"));
    /// ```
    fn trim_first(&self) -> PathBuf;

    /// Returns a new [`PathBuf`] with last [`Component`] trimmed off.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo").trim_last(), PathBuf::from("/"));
    /// ```
    fn trim_last(&self) -> PathBuf;

    /// Returns a new [`PathBuf`] with the given prefix trimmed off else the original `path`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar").trim_prefix("/foo"), PathBuf::from("/bar"));
    /// ```
    fn trim_prefix<T: AsRef<Path>>(&self, prefix: T) -> PathBuf;

    /// Returns a new [`PathBuf`] with well known protocol prefixes trimmed off else the original
    /// `path`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("ftp://foo").trim_protocol(), PathBuf::from("foo"));
    /// ```
    fn trim_protocol(&self) -> PathBuf;

    /// Returns a new [`PathBuf`] with the given `suffix` trimmed off else the original `path`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar").trim_suffix("/bar"), PathBuf::from("/foo"));
    /// ```
    fn trim_suffix<T: AsRef<Path>>(&self, suffix: T) -> PathBuf;
}

/// Provides extension method ergonomics for all the system module helper functions for paths
impl PathExt for Path {
    /// Simply a wrapper for `file_name` to return the final component of the `Path`, if there is
    /// one else an error.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar").base().unwrap(), "bar".to_string());
    /// ```
    fn base(&self) -> RvResult<String> {
        base(self)
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
    /// assert_eq!(Path::new("./foo/./bar").clean(), PathBuf::from("foo/bar"));
    /// ```
    fn clean(&self) -> PathBuf {
        clean(self)
    }

    /// Returns the `Path` with the given string concatenated on without injecting
    /// path separators.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar").concat(".rs").unwrap(), PathBuf::from("/foo/bar.rs"));
    /// ```
    fn concat<T: AsRef<str>>(&self, val: T) -> RvResult<PathBuf> {
        concat(self, val)
    }

    /// Returns the `Path` without its final component, if there is one.
    ///
    /// ### Errors
    /// * PathError:ParentNotFound if the path has no parent
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar").dir().unwrap(), PathBuf::from("/foo"));
    /// ```
    fn dir(&self) -> RvResult<PathBuf> {
        dir(self)
    }

    /// Expand the path to include the home prefix if necessary
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(Path::new("~/foo").expand().unwrap(), PathBuf::from(&home).mash("foo"));
    /// ```
    fn expand(&self) -> RvResult<PathBuf> {
        expand(self)
    }

    /// Returns the extension of the path or an error.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("foo.bar").ext().unwrap(), "bar");
    /// ```
    fn ext(&self) -> RvResult<String> {
        ext(self)
    }
    /// Returns the first path component.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("foo/bar").first().unwrap(), "foo".to_string());
    /// ```
    fn first(&self) -> RvResult<String> {
        first(self)
    }

    /// Returns true if the `Path` is empty.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(PathBuf::from("").is_empty(), true);
    /// ```
    fn is_empty(&self) -> bool {
        is_empty(self)
    }

    /// Returns true if the `Path` contains the given path or string.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let path = PathBuf::from("/foo/bar");
    /// assert_eq!(path.has("foo"), true);
    /// assert_eq!(path.has("/foo"), true);
    /// ```
    fn has<T: AsRef<Path>>(&self, val: T) -> bool {
        has(self, val)
    }
    /// Returns true if the `Path` as a String has the given prefix
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let path = PathBuf::from("/foo/bar");
    /// assert_eq!(path.has_prefix("/foo"), true);
    /// assert_eq!(path.has_prefix("foo"), false);
    /// ```
    fn has_prefix<T: AsRef<Path>>(&self, prefix: T) -> bool {
        has_prefix(self, prefix)
    }

    /// Returns true if the `Path` as a String has the given suffix
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let path = PathBuf::from("/foo/bar");
    /// assert_eq!(path.has_suffix("/bar"), true);
    /// assert_eq!(path.has_suffix("foo"), false);
    /// ```
    fn has_suffix<T: AsRef<Path>>(&self, suffix: T) -> bool {
        has_suffix(self, suffix)
    }

    /// Returns the last path component. Alias to `base`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("foo/bar").last().unwrap(), "bar".to_string());
    /// ```
    fn last(&self) -> RvResult<String> {
        last(self)
    }

    /// Returns a new owned [`PathBuf`] from `self` mashed together with `path`. This is a safer
    /// implementation than the `join` method as it drops the root prefix of the given `path` if
    /// it exists and also drops any trailing '/' on the new resulting path. This implementation
    /// more closely aligns with the Golang implementation.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo").mash("bar"), PathBuf::from("/foo/bar"));
    /// assert_eq!(Path::new("/foo").mash("/bar"), PathBuf::from("/foo/bar"));
    /// ```
    fn mash<T: AsRef<Path>>(&self, path: T) -> PathBuf {
        mash(self, path)
    }

    /// Returns the final component of the `Path` without an extension if there is one
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar.foo").name().unwrap(), "bar");
    /// ```
    fn name(&self) -> RvResult<String> {
        name(self)
    }

    /// Returns the `Path` relative to the given `base` path
    ///
    /// Think what is the path navigation required to get from `base` to `path`. Every path used
    /// should represent a directory not a file or link. For files or links trim off the last
    /// segement of the path before calling this method. No attempt is made by this method to
    /// trim off the file segment.
    ///
    /// ### Arguments
    /// * `path` - path to return the navigation relative to base for, expected to be in absolute
    ///   form
    /// * `base` - path to calculate navigation from, expected to be in absolute form
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("foo/bar1").relative("foo/bar2").unwrap(), PathBuf::from("../bar1"));
    /// ```
    fn relative<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf> {
        relative(self, path)
    }

    /// Returns a new [`PathBuf`] with the file extension trimmed off.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("foo.exe").trim_ext().unwrap(), PathBuf::from("foo"));
    /// ```
    fn trim_ext(&self) -> RvResult<PathBuf> {
        trim_ext(self)
    }

    /// Returns a new [`PathBuf`] with first [`Component`] trimmed off.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo").trim_first(), PathBuf::from("foo"));
    /// ```
    fn trim_first(&self) -> PathBuf {
        trim_first(self)
    }

    /// Returns a new [`PathBuf`] with last [`Component`] trimmed off.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo").trim_last(), PathBuf::from("/"));
    /// ```
    fn trim_last(&self) -> PathBuf {
        trim_last(self)
    }

    /// Returns a new [`PathBuf`] with the given prefix trimmed off else the original `path`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar").trim_prefix("/foo"), PathBuf::from("/bar"));
    /// ```
    fn trim_prefix<T: AsRef<Path>>(&self, prefix: T) -> PathBuf {
        trim_prefix(self, prefix)
    }

    /// Returns a new [`PathBuf`] with well known protocol prefixes trimmed off else the original
    /// `path`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("ftp://foo").trim_protocol(), PathBuf::from("foo"));
    /// ```
    fn trim_protocol(&self) -> PathBuf {
        trim_protocol(self)
    }

    /// Returns a new [`PathBuf`] with the given `suffix` trimmed off else the original `path`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo/bar").trim_suffix("/bar"), PathBuf::from("/foo"));
    /// ```
    fn trim_suffix<T: AsRef<Path>>(&self, suffix: T) -> PathBuf {
        trim_suffix(self, suffix)
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_pathext_base() {
        assert_eq!(Path::new("").base().unwrap_err().to_string(), IterError::item_not_found().to_string());
        assert_eq!(Path::new("bar").base().unwrap(), "bar".to_string());
        assert_eq!(Path::new("/foo/bar").base().unwrap(), "bar".to_string());
        assert_eq!(Path::new("/foo/bar.bin").base().unwrap(), "bar.bin".to_string());
    }

    #[test]
    fn test_pathext_clean() {
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
            assert_eq!(Path::new(test.1).clean(), PathBuf::from(test.0));
        }
    }

    #[test]
    fn test_pathext_concat() {
        assert_eq!(Path::new("/foo/bar").concat(".rs").unwrap(), PathBuf::from("/foo/bar.rs"));
        assert_eq!(PathBuf::from("bar").concat(".rs").unwrap(), PathBuf::from("bar.rs"));
    }

    #[test]
    fn test_pathext_dir() {
        assert_eq!(Path::new("/foo/").dir().unwrap(), PathBuf::from("/").as_path(),);
        assert_eq!(Path::new("/foo/bar/").dir().unwrap(), PathBuf::from("/foo").as_path(),);
        assert_eq!(Path::new("/foo/bar").dir().unwrap(), PathBuf::from("/foo").as_path());
    }

    #[test]
    fn test_pathext_expand() -> RvResult<()> {
        let home = sys::home_dir()?;

        // Multiple home symbols should fail
        assert_eq!(
            Path::new("~/~").expand().unwrap_err().to_string(),
            PathError::multiple_home_symbols("~/~").to_string()
        );

        // Only home expansion at the begining of the path is allowed
        assert_eq!(
            Path::new("foo/~").expand().unwrap_err().to_string(),
            PathError::invalid_expansion("foo/~").to_string()
        );

        // Tilda only
        assert_eq!(Path::new("~").expand()?, PathBuf::from(&home));

        // Standard prefix
        assert_eq!(Path::new("~/foo").expand()?, PathBuf::from(&home).join("foo"));

        // Variable expansion
        assert_eq!(Path::new("${HOME}").expand()?, PathBuf::from(&home));
        assert_eq!(Path::new("${HOME}/foo").expand()?, PathBuf::from(&home).join("foo"));
        assert_eq!(Path::new("/foo/${HOME}").expand()?, PathBuf::from("/foo").join(&home));
        assert_eq!(Path::new("/foo/${HOME}/bar").expand()?, PathBuf::from("/foo").join(&home).join("bar"));
        assert_eq!(
            Path::new("/foo${HOME}/bar").expand()?,
            PathBuf::from("/foo".to_string() + &home.to_string()? + &"/bar".to_string())
        );
        assert_eq!(
            Path::new("/foo${HOME}${HOME}").expand()?,
            PathBuf::from("/foo".to_string() + &home.to_string()? + &home.to_string()?)
        );
        assert_eq!(
            Path::new("/foo$HOME$HOME").expand()?,
            PathBuf::from("/foo".to_string() + &home.to_string()? + &home.to_string()?)
        );
        Ok(())
    }

    #[test]
    fn test_pathext_ext() {
        assert_eq!(
            Path::new("base").ext().unwrap_err().to_string(),
            PathError::extension_not_found("base").to_string()
        );
        assert_eq!(Path::new("base.bin").ext().unwrap(), "bin".to_string());
        assert_eq!(Path::new("/foo/bar/base.blah").ext().unwrap(), "blah".to_string());
    }

    #[test]
    fn test_pathext_first() {
        assert_eq!(Path::new("").first().unwrap_err().to_string(), IterError::item_not_found().to_string());
        assert_eq!(Path::new("foo").first().unwrap(), "foo".to_string());
        assert_eq!(Path::new("/foo").first().unwrap(), "/".to_string());
    }

    #[test]
    fn test_pathext_has() {
        assert_eq!(Path::new("").has(""), true);
        assert_eq!(Path::new("/foo").has("fo"), true);
        assert_eq!(Path::new("/foo/bar").has("bar"), true);
        assert_eq!(Path::new("/foo/bar").has("bar/"), false);
    }

    #[test]
    fn test_pathext_has_prefix() {
        assert_eq!(Path::new("").has_prefix(""), true);
        assert_eq!(Path::new("/foo").has_prefix("/fo"), true);
        assert_eq!(Path::new("/foo/bar").has_prefix("bar/"), false);
    }

    #[test]
    fn test_pathext_has_suffix() {
        assert_eq!(Path::new("").has_suffix(""), true);
        assert_eq!(Path::new("/foo").has_suffix("/fo"), false);
        assert_eq!(Path::new("/foo/bar").has_suffix("bar"), true);
    }

    #[test]
    fn test_pathext_last() {
        assert_eq!(Path::new("").last().unwrap_err().to_string(), IterError::item_not_found().to_string());
        assert_eq!(Path::new("foo").last().unwrap(), "foo".to_string());
        assert_eq!(Path::new("/foo").last().unwrap(), "foo".to_string());
    }

    #[test]
    fn test_sys_home_dir() {
        let home = sys::home_dir().unwrap();
        assert!(home != PathBuf::new());
        assert!(home.starts_with("/"));
        assert_eq!(home.join("foo"), PathBuf::from(&home).join("foo"));
    }

    #[test]
    fn test_pathext_is_empty() {
        assert_eq!(Path::new("/").is_empty(), false);
        assert_eq!(Path::new("").is_empty(), true);
        assert_eq!(PathBuf::from("").is_empty(), true);
    }

    #[test]
    fn test_pathext_mash() {
        // mashing nothing should yield no change
        assert_eq!(Path::new("").mash(""), PathBuf::from(""));
        assert_eq!(Path::new("/foo").mash(""), PathBuf::from("/foo"));

        // strips off root on path
        assert_eq!(Path::new("/foo").mash("/bar"), PathBuf::from("/foo/bar"));

        // strips off trailing slashes
        assert_eq!(Path::new("/foo").mash("bar/"), PathBuf::from("/foo/bar"));
    }

    #[test]
    fn test_pathext_name() {
        assert_eq!(Path::new("").name().unwrap_err().to_string(), IterError::item_not_found().to_string());
        assert_eq!(Path::new("bar").name().unwrap(), "bar".to_string());
        assert_eq!(Path::new("/foo/bar").name().unwrap(), "bar".to_string());
        assert_eq!(Path::new("/foo/bar.bin").name().unwrap(), "bar".to_string());
    }

    #[test]
    fn test_pathext_relative() {
        // share same directory
        assert_eq!(Path::new("bar1").relative("bar2").unwrap(), PathBuf::from("../bar1"));
        assert_eq!(Path::new("foo/bar1").relative("foo/bar2").unwrap(), PathBuf::from("../bar1"));
        assert_eq!(Path::new("~/foo/bar1").relative("~/foo/bar2").unwrap(), PathBuf::from("../bar1"));
        assert_eq!(Path::new("../foo/bar1").relative("../foo/bar2").unwrap(), PathBuf::from("../bar1"));

        // share parent directory
        assert_eq!(Path::new("foo1/bar1").relative("foo2/bar2").unwrap(), PathBuf::from("../../foo1/bar1"));
        assert_eq!(Path::new("/foo1/bar1").relative("/foo2/bar2").unwrap(), PathBuf::from("../../foo1/bar1"));

        // share grandparent directory
        assert_eq!(
            Path::new("blah1/foo1/bar1").relative("blah2/foo2/bar2").unwrap(),
            PathBuf::from("../../../blah1/foo1/bar1")
        );
        assert_eq!(
            Path::new("/blah1/foo1/bar1").relative("/blah2/foo2/bar2").unwrap(),
            PathBuf::from("../../../blah1/foo1/bar1")
        );

        // symlink is the opposite i.e. src.relative(dst)
        assert_eq!(Path::new("/dir1").relative("/dir1/dir2").unwrap(), PathBuf::from(".."));
    }

    #[test]
    fn test_pathext_trim_ext() {
        assert_eq!(Path::new("/").trim_ext().unwrap(), PathBuf::from("/"));
        assert_eq!(Path::new("/foo").trim_ext().unwrap(), PathBuf::from("/foo"));
        assert_eq!(Path::new("/foo.bar").trim_ext().unwrap(), PathBuf::from("/foo"));
        assert_eq!(Path::new("/foo.bar.bar").trim_ext().unwrap(), PathBuf::from("/foo.bar"));
    }

    #[test]
    fn test_pathext_trim_first() {
        assert_eq!(Path::new("/").trim_first(), PathBuf::from(""),);
        assert_eq!(Path::new("foo/bar").trim_first(), PathBuf::from("bar"),);
        assert_eq!(Path::new("/foo/bar").trim_first(), PathBuf::from("foo/bar"),);
    }

    #[test]
    fn test_pathext_trim_last() {
        assert_eq!(Path::new("/").trim_last(), PathBuf::from(""),);
        assert_eq!(Path::new("foo/bar").trim_last(), PathBuf::from("foo"),);
        assert_eq!(Path::new("/foo/bar").trim_last(), PathBuf::from("/foo"),);
    }

    #[test]
    fn test_pathext_trim_prefix() {
        // drop root
        assert_eq!(Path::new("/").trim_prefix("/"), PathBuf::new());

        // drop start
        assert_eq!(Path::new("/foo/bar").trim_prefix("/foo"), PathBuf::from("/bar"));

        // no change
        assert_eq!(Path::new("/").trim_prefix(""), PathBuf::from("/"));
        assert_eq!(Path::new("/foo").trim_prefix("blah"), PathBuf::from("/foo"));
    }

    #[test]
    fn test_pathext_trim_protocol() {
        // no change
        assert_eq!(Path::new("/foo").trim_protocol(), PathBuf::from("/foo"));

        // file://
        assert_eq!(Path::new("file:///foo").trim_protocol(), PathBuf::from("/foo"));

        // ftp://
        assert_eq!(Path::new("ftp://foo").trim_protocol(), PathBuf::from("foo"));

        // http://
        assert_eq!(Path::new("http://foo").trim_protocol(), PathBuf::from("foo"));

        // https://
        assert_eq!(Path::new("https://foo").trim_protocol(), PathBuf::from("foo"));

        // Check case is being considered
        assert_eq!(Path::new("HTTPS://Foo").trim_protocol(), PathBuf::from("Foo"));
        assert_eq!(Path::new("Https://Foo").trim_protocol(), PathBuf::from("Foo"));
        assert_eq!(Path::new("HttpS://FoO").trim_protocol(), PathBuf::from("FoO"));

        // Check non protocol matches are ignored
        assert_eq!(Path::new("foo").trim_protocol(), PathBuf::from("foo"));
        assert_eq!(Path::new("foo/bar").trim_protocol(), PathBuf::from("foo/bar"));
        assert_eq!(Path::new("foo//bar").trim_protocol(), PathBuf::from("foo//bar"));
        assert_eq!(Path::new("ntp:://foo").trim_protocol(), PathBuf::from("ntp:://foo"));
    }

    #[test]
    fn test_pathext_trim_suffix() {
        // drop root
        assert_eq!(Path::new("/").trim_suffix("/"), PathBuf::new());

        // drop start
        assert_eq!(Path::new("/foo/bar").trim_suffix("/bar"), PathBuf::from("/foo"));

        // no change
        assert_eq!(Path::new("/").trim_suffix(""), PathBuf::from("/"));
        assert_eq!(Path::new("/foo").trim_suffix("blah"), PathBuf::from("/foo"));
    }
}

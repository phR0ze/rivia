//! The `path` module provides a number of helper functions to assist in manipulating paths.
//! Only those functions that are filesystem agnostic should be included here.

use std::path::{Component, Path, PathBuf};

use crate::{errors::*, exts::*};

/// Returns the final component of the `Path`, if there is one.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!("bar", sys::base("/foo/bar")).unwrap());
/// ```
pub fn base<T: AsRef<Path>>(path: T) -> RvResult<String>
{
    let path = path.as_ref();
    path.file_name().ok_or_else(|| PathError::filename_not_found(path))?.to_string()
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
/// assert_eq!(sys::clean("./foo/./bar").unwrap(), PathBuf::from("foo/bar"));
/// ```
pub fn clean<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
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
    Ok(path_buf)
}

/// Returns the `Path` without its final component, if there is one.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::dir("/foo/bar").unwrap(), PathBuf::from("/foo").as_path());
/// ```
pub fn dir<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
    let path = path.as_ref();
    let dir = path.parent().ok_or_else(|| PathError::parent_not_found(path))?;
    Ok(dir.to_path_buf())
}

/// Expand all environment variables in the path as well as the home directory.
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
pub fn expand<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
    let path = path.as_ref();
    let pathstr = path.to_string()?;

    // Expand home directory
    let path = match pathstr.matches('~').count() {
        // Only a single home expansion is allowed
        cnt if cnt > 1 => return Err(PathError::multiple_home_symbols(path).into()),

        // Home expansion only makes sense at the beinging of a path
        cnt if cnt == 1 && !has_prefix(path, "~/") && pathstr != "~" => return Err(PathError::invalid_expansion(path).into()),

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

/// Returns the final component of the `Path` without an extension if there is one
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::name("/foo/bar.foo").unwrap(), "bar");
/// ```
pub fn name<T: AsRef<Path>>(path: T) -> RvResult<String>
{
    base(trim_ext(path)?)
}

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
/// assert_eq!(sys::has_prefix(&path, "/foo"), true);
/// assert_eq!(sys::has_prefix(&path, "foo"), false);
/// ```
pub fn has_prefix<T: AsRef<Path>, U: AsRef<Path>>(path: T, prefix: U) -> bool
{
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
/// assert_eq!(sys::has_suffix("/bar"), true);
/// assert_eq!(sys::has_suffix("foo"), false);
/// ```
pub fn has_suffix<T: AsRef<Path>, U: AsRef<Path>>(path: T, suffix: U) -> bool
{
    match (path.as_ref().to_string(), suffix.as_ref().to_string()) {
        (Ok(base), Ok(suffix)) => base.ends_with(&suffix),
        _ => false,
    }
}

/// Returns the full path to the current user's home directory.
///
/// Alternate implementation as the Rust std::env::home_dir implementation has been
/// deprecated https://doc.rust-lang.org/std/env/fn.home_dir.html
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(sys::home_dir().is_ok());
/// ```
pub fn home_dir() -> RvResult<PathBuf>
{
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
pub fn is_empty<T: Into<PathBuf>>(path: T) -> bool
{
    path.into() == PathBuf::new()
}

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
// pub fn last<T: AsRef<Path>>(path: T) -> RvResult<String>
// {
//     path.as_ref().components().last_result()?.to_string()?;
//     Ok(())
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
/// assert_eq!(sys::mash("/foo", "/bar"), PathBuf::from("/foo/bar"));
/// ```
pub fn mash<T: AsRef<Path>, U: AsRef<Path>>(dir: T, base: U) -> PathBuf
{
    let base = trim_prefix(base, "/");
    let path = dir.as_ref().join(base);
    path.components().collect::<PathBuf>()
}

/// Returns a new [`PathBuf`] with the file extension trimmed off.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::trim_ext("foo.exe").unwrap(), PathBuf::from("foo"));
/// ```
pub fn trim_ext<T: AsRef<Path>>(path: T) -> RvResult<PathBuf>
{
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
pub fn trim_first<T: AsRef<Path>>(path: T) -> PathBuf
{
    path.as_ref().components().drop(1).as_path().to_path_buf()
}

/// Returns a new [`PathBuf`] with last [`Component`] trimmed off.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(sys::trim_last("/foo")), PathBuf::from("/"));
/// ```
pub fn trim_last<T: AsRef<Path>>(path: T) -> PathBuf
{
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
pub fn trim_prefix<T: AsRef<Path>, U: AsRef<Path>>(path: T, prefix: U) -> PathBuf
{
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
pub fn trim_protocol<T: AsRef<Path>>(path: T) -> PathBuf
{
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
pub fn trim_suffix<T: AsRef<Path>, U: AsRef<Path>>(path: T, suffix: U) -> PathBuf
{
    let path = path.as_ref();
    match (path.to_string(), suffix.as_ref().to_string()) {
        (Ok(base), Ok(suffix)) if base.ends_with(&suffix) => PathBuf::from(&base[..base.size() - suffix.size()]),
        _ => path.to_path_buf(),
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[test]
    fn test_stdfs_clean()
    {
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
            assert_eq!(sys::clean(test.1).unwrap(), PathBuf::from(test.0));
        }
    }

    #[test]
    fn test_stdfs_expand() -> RvResult<()>
    {
        let home = sys::home_dir()?;

        // Multiple home symbols should fail
        assert_eq!(sys::expand("~/~").unwrap_err().to_string(), PathError::multiple_home_symbols("~/~").to_string());

        // Only home expansion at the begining of the path is allowed
        assert_eq!(sys::expand("foo/~").unwrap_err().to_string(), PathError::invalid_expansion("foo/~").to_string());

        // Tilda only
        assert_eq!(sys::expand("~")?, PathBuf::from(&home));

        // Standard prefix
        assert_eq!(sys::expand("~/foo")?, PathBuf::from(&home).join("foo"));

        // Variable expansion
        assert_eq!(sys::expand("${HOME}")?, PathBuf::from(&home));
        assert_eq!(sys::expand("${HOME}/foo")?, PathBuf::from(&home).join("foo"));
        assert_eq!(sys::expand("/foo/${HOME}")?, PathBuf::from("/foo").join(&home));
        assert_eq!(sys::expand("/foo/${HOME}/bar")?, PathBuf::from("/foo").join(&home).join("bar"));
        assert_eq!(sys::expand("/foo${HOME}/bar")?, PathBuf::from("/foo".to_string() + &home.to_string()? + &"/bar".to_string()));
        assert_eq!(sys::expand("/foo${HOME}${HOME}")?, PathBuf::from("/foo".to_string() + &home.to_string()? + &home.to_string()?));
        assert_eq!(sys::expand("/foo$HOME$HOME")?, PathBuf::from("/foo".to_string() + &home.to_string()? + &home.to_string()?));
        Ok(())
    }

    #[test]
    fn test_sys_dirname()
    {
        assert_eq!(sys::dir("/foo/").unwrap(), PathBuf::from("/").as_path(),);
        assert_eq!(sys::dir("/foo/bar").unwrap(), PathBuf::from("/foo").as_path());
    }

    #[test]
    fn test_sys_has_prefix()
    {
        let path = PathBuf::from("/foo/bar");
        assert_eq!(sys::has_prefix(&path, "/foo"), true);
        assert_eq!(sys::has_prefix(&path, "foo"), false);
    }

    #[test]
    fn test_sys_home_dir()
    {
        let home = sys::home_dir().unwrap();
        assert!(home != PathBuf::new());
        assert!(home.starts_with("/"));
        assert_eq!(home.join("foo"), PathBuf::from(&home).join("foo"));
    }

    #[test]
    fn test_sys_is_empty()
    {
        assert_eq!(sys::is_empty(""), true);
        assert_eq!(sys::is_empty(Path::new("")), true);
        assert_eq!(sys::is_empty("/"), false);
    }

    #[test]
    fn test_sys_mash()
    {
        // mashing nothing should yield no change
        assert_eq!(sys::mash("", ""), PathBuf::from(""));
        assert_eq!(sys::mash("/foo", ""), PathBuf::from("/foo"));

        // strips off root on path
        assert_eq!(sys::mash("/foo", "/bar"), PathBuf::from("/foo/bar"));

        // strips off trailing slashes
        assert_eq!(sys::mash("/foo", "bar/"), PathBuf::from("/foo/bar"));
    }

    #[test]
    fn test_sys_trim_first()
    {
        assert_eq!(sys::trim_first("/"), PathBuf::new(),);
        assert_eq!(sys::trim_first("/foo"), PathBuf::from("foo"));
    }

    #[test]
    fn test_sys_trim_prefix()
    {
        // drop root
        assert_eq!(sys::trim_prefix("/", "/"), PathBuf::new());

        // drop start
        assert_eq!(sys::trim_prefix("/foo/bar", "/foo"), PathBuf::from("/bar"));

        // no change
        assert_eq!(sys::trim_prefix("/", ""), PathBuf::from("/"));
        assert_eq!(sys::trim_prefix("/foo", "blah"), PathBuf::from("/foo"));
    }

    #[test]
    fn test_sys_trim_protocol()
    {
        // no change
        assert_eq!(sys::trim_protocol("/foo"), PathBuf::from("/foo"));

        // file://
        assert_eq!(sys::trim_protocol("file:///foo"), PathBuf::from("/foo"));

        // ftp://
        assert_eq!(sys::trim_protocol("ftp://foo"), PathBuf::from("foo"));

        // http://
        assert_eq!(sys::trim_protocol("http://foo"), PathBuf::from("foo"));

        // https://
        assert_eq!(sys::trim_protocol("https://foo"), PathBuf::from("foo"));

        // Check case is being considered
        assert_eq!(sys::trim_protocol("HTTPS://Foo"), PathBuf::from("Foo"));
        assert_eq!(sys::trim_protocol("Https://Foo"), PathBuf::from("Foo"));
        assert_eq!(sys::trim_protocol("HttpS://FoO"), PathBuf::from("FoO"));

        // Check non protocol matches are ignored
        assert_eq!(sys::trim_protocol("foo"), PathBuf::from("foo"));
        assert_eq!(sys::trim_protocol("foo/bar"), PathBuf::from("foo/bar"));
        assert_eq!(sys::trim_protocol("foo//bar"), PathBuf::from("foo//bar"));
        assert_eq!(sys::trim_protocol("ntp:://foo"), PathBuf::from("ntp:://foo"));
    }
}
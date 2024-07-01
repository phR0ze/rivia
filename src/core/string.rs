use std::{
    ffi::OsStr,
    path::{Component, Path, PathBuf},
    str,
};

use crate::errors::*;

/// Provides string manipulation extensions for the [`str`] and [`String`] types
pub trait StringExt {
    /// Returns the length in characters rather than bytes i.e. this is a human understandable
    /// value. However it is more costly to perform.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!("foo".size(), 3);
    /// assert_eq!("ƒoo".len(), 4); // fancy f!
    /// assert_eq!("ƒoo".size(), 3); // fancy f!
    /// ```
    fn size(&self) -> usize;

    /// Convert the string into a bool
    ///
    /// * Returns `true` if non-empty and any value other than `0` or case insensitive `false`
    /// * Returns `false` if empty or value is `0` or case insensitive `false`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!("foo".to_bool(), true);
    /// assert_eq!("".to_bool(), false);
    /// assert_eq!("0".to_bool(), false);
    /// assert_eq!("false".to_bool(), false);
    /// assert_eq!("FALSE".to_bool(), false);
    /// ```
    fn to_bool(&self) -> bool;

    /// Returns a new [`String`] with the given `suffix` trimmed off else the original `String`.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!("/foo/bar".to_string().trim_suffix("/bar"), "/foo");
    /// ```
    fn trim_suffix<T: Into<String>>(&self, suffix: T) -> String;
}

impl StringExt for str {
    fn size(&self) -> usize {
        self.chars().count()
    }

    /// Convert the string into a bool
    ///
    /// * Returns `true` if non-empty and any value other than `0` or case insensitive `false`
    /// * Returns `false` if empty or value is `0` or case insensitive `false`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!("foo".to_bool(), true);
    /// assert_eq!("".to_bool(), false);
    /// assert_eq!("0".to_bool(), false);
    /// assert_eq!("false".to_bool(), false);
    /// assert_eq!("FALSE".to_bool(), false);
    /// ```
    fn to_bool(&self) -> bool {
        self.to_string().to_bool()
    }

    fn trim_suffix<T: Into<String>>(&self, suffix: T) -> String {
        let target = suffix.into();
        match self.ends_with(&target) {
            true => self[..self.len() - target.len()].to_owned(),
            _ => self.to_owned(),
        }
    }
}

impl StringExt for String {
    fn size(&self) -> usize {
        self.chars().count()
    }

    /// Convert the string into a bool
    ///
    /// * Returns `true` if non-empty and any value other than `0` or case insensitive `false`
    /// * Returns `false` if empty or value is `0` or case insensitive `false`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!("foo".to_string().to_bool(), true);
    /// assert_eq!("".to_string().to_bool(), false);
    /// assert_eq!("0".to_string().to_bool(), false);
    /// assert_eq!("false".to_string().to_bool(), false);
    /// assert_eq!("FALSE".to_string().to_bool(), false);
    /// ```
    fn to_bool(&self) -> bool {
        let x = self.to_lowercase();
        !(x.is_empty() || x == "false" || x == "0")
    }

    fn trim_suffix<T: Into<String>>(&self, suffix: T) -> String {
        let target = suffix.into();
        match self.ends_with(&target) {
            true => self[..self.len() - target.len()].to_owned(),
            _ => self.to_owned(),
        }
    }
}

/// Provides to_string extension for the [`Path`], [`OsStr`] and [`Component`] types
pub trait ToStringExt {
    /// Returns a new [`String`] from the given type.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(Path::new("/foo").to_string().unwrap(), "/foo");
    /// ```
    fn to_string(&self) -> RvResult<String>;
}

impl ToStringExt for Path {
    fn to_string(&self) -> RvResult<String> {
        let _str = self.to_str().ok_or(PathError::failed_to_string(self))?;
        Ok(String::from(_str))
    }
}

impl ToStringExt for OsStr {
    fn to_string(&self) -> RvResult<String> {
        Ok(String::from(self.to_str().ok_or(StringError::FailedToString)?))
    }
}

impl ToStringExt for Component<'_> {
    fn to_string(&self) -> RvResult<String> {
        let mut path = PathBuf::new();
        path.push(self);
        path.to_string()
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use std::{
        ffi::OsStr,
        path::{Component, Path, PathBuf},
    };

    use crate::prelude::*;

    #[test]
    fn test_str_size() {
        assert_eq!("foo".size(), 3);
        assert_eq!("ƒoo".len(), 4); // fancy f!
        assert_eq!("ƒoo".size(), 3); // fancy f!
    }

    #[test]
    fn test_string_size() {
        assert_eq!("foo".to_string().size(), 3);
        assert_eq!("ƒoo".to_string().len(), 4); // fancy f!
        assert_eq!("ƒoo".to_string().size(), 3); // fancy f!
    }

    #[test]
    fn test_str_to_bool() {
        assert_eq!("foo".to_bool(), true);
        assert_eq!("true".to_bool(), true);
        assert_eq!("TRUE".to_bool(), true);
        assert_eq!("".to_bool(), false);
        assert_eq!("0".to_bool(), false);
        assert_eq!("false".to_bool(), false);
        assert_eq!("FALSE".to_bool(), false);
    }

    #[test]
    fn test_string_to_bool() {
        assert_eq!("foo".to_string().to_bool(), true);
        assert_eq!("true".to_string().to_bool(), true);
        assert_eq!("TRUE".to_string().to_bool(), true);
        assert_eq!("".to_string().to_bool(), false);
        assert_eq!("0".to_string().to_bool(), false);
        assert_eq!("false".to_string().to_bool(), false);
        assert_eq!("FALSE".to_string().to_bool(), false);
    }

    #[test]
    fn test_str_trim_suffix() {
        assert_eq!("foo".trim_suffix("boo"), "foo"); // no change
        assert_eq!("foo".trim_suffix("oo"), "f");
        assert_eq!("ƒoo".trim_suffix("o"), "ƒo"); // fancy f!
    }

    #[test]
    fn test_string_trim_suffix() {
        assert_eq!("foo".to_string().trim_suffix("boo"), "foo"); // no change
        assert_eq!("foo".to_string().trim_suffix("oo"), "f");
        assert_eq!("ƒoo".to_string().trim_suffix("o"), "ƒo"); // fancy f!
    }

    #[test]
    fn test_osstr_to_string() {
        assert_eq!(OsStr::new("foo").to_string().unwrap(), "foo");
    }

    #[test]
    fn test_path_to_string() {
        assert_eq!(Path::new("/foo").to_string().unwrap(), "/foo");
        assert_eq!(PathBuf::from("/foo").to_string().unwrap(), "/foo");
    }

    #[test]
    fn test_component_to_string() {
        assert_eq!(Component::RootDir.to_string().unwrap(), "/");
        assert_eq!(Component::CurDir.to_string().unwrap(), ".");
        assert_eq!(Component::ParentDir.to_string().unwrap(), "..");
        assert_eq!(Component::Normal(OsStr::new("foo")).to_string().unwrap(), "foo");
    }
}

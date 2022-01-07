use crate::{
    errors::*,
    fs::{Entry, FileSystem, Stdfs, StdfsEntry, Vfs},
    iters::*,
};
use std::{
    fs::{self, File},
    path::{Component, Path, PathBuf},
    os::unix::{
        self,
        fs::PermissionsExt,
    },
};

/// `Memfs` is a Vfs backend implementation that is purely memory based
#[derive(Debug)]
pub struct Memfs;
impl Memfs {
    /// Create a new instance of the Memfs Vfs backend implementation
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

    /// Write the given data to to the indicated file creating the file first if it doesn't exist
    /// or truncating it first if it does. If the path exists an isn't a file an error will be
    /// returned.
    /// 
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// ```
    pub fn mkfile<T: AsRef<Path>>(path: T, data: &[u8]) -> RvResult<()> {
        let path = Stdfs::abs(path)?;

        Ok(())
    }
}

impl FileSystem for Memfs
{
    /// Return the path in an absolute clean form
    fn abs(&self, path: &Path) -> RvResult<PathBuf>
    {
        Stdfs::abs(path)
    }

    /// Write the given data to to the indicated file creating the file first if it doesn't exist
    /// or truncating it first if it does.
    fn mkfile(&self, path: &Path, data: &[u8]) -> RvResult<()>
    {
        Stdfs::mkfile(path, data)
    }

    /// Up cast the trait type to the enum wrapper
    fn upcast(self) -> Vfs {
        Vfs::Memfs(self)
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
}
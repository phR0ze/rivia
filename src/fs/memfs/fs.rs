use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};

use crate::{
    errors::*,
    fs::{FileSystem, MemfsEntry, Stdfs, Vfs},
    iters::*,
};

/// `Memfs` is a Vfs backend implementation that is purely memory based
#[derive(Debug)]
pub struct Memfs
{
    cwd: PathBuf,                     // Current working directory
    fs: HashMap<PathBuf, MemfsEntry>, // filesystem
}

impl Memfs
{
    /// Create a new instance of the Memfs Vfs backend implementation
    pub fn new() -> Self
    {
        let mut fs = HashMap::new();
        fs.insert(PathBuf::from("/"), MemfsEntry::default());
        Self {
            cwd: PathBuf::from("/"),
            fs,
        }
    }

    /// Returns the current working directory as a [`PathBuf`].
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// println!("current working directory: {:?}", Memfs::cwd().unwrap());
    /// ```
    pub fn cwd(&self) -> RvResult<PathBuf>
    {
        Ok(self.cwd.clone())
    }
}

impl FileSystem for Memfs
{
    /// Return the path in an absolute clean form
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn abs(&self, path: &Path) -> RvResult<PathBuf>
    {
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
                    _ => return Ok(Stdfs::mash(curr, path_buf)),
                };
            }
            return Ok(curr);
        }

        Ok(path_buf)
    }

    /// Read all data from the given file and return it as a String
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn read_all(&self, path: &Path) -> RvResult<String>
    {
        let path = self.abs(path.as_ref())?;
        Ok("".to_string())
    }

    /// Write the given data to to the indicated file creating the file first if it doesn't exist or
    /// truncating it first if it does.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn write_all(&self, path: &Path, data: &[u8]) -> RvResult<()>
    {
        Ok(())
    }

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn upcast(self) -> Vfs
    {
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
    fn test_memfs_cwd() -> RvResult<()>
    {
        let memfs = Memfs::new();
        assert_eq!(memfs.cwd()?, PathBuf::from("/"));
        Ok(())
    }
}

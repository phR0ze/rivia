use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::{
    errors::*,
    fs::{Entry, FileSystem, MemfsEntry, MemfsEntryOpts, Stdfs, Vfs},
    iters::*,
};

/// `Memfs` is a Vfs backend implementation that is purely memory based
#[derive(Debug)]
pub struct Memfs
{
    cwd: PathBuf,     // Current working directory
    root: MemfsEntry, // Root Entry in the filesystem
}

impl Memfs
{
    /// Create a new instance of the Memfs Vfs backend implementation
    pub fn new() -> Self
    {
        Self {
            cwd: PathBuf::from("/"),
            root: MemfsEntryOpts::new("/").entry(),
        }
    }

    /// Returns true if the `Path` exists. Handles path expansion.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn exists<T: AsRef<Path>>(&self, path: T) -> bool
    {
        // match self.abs(path.as_ref()) {
        //     Ok(abs) => {
        //         let fs = self.fs.read().unwrap();
        //         let entry = fs.get("/");
        //         for component in abs.components() {
        //             if let Component::Normal(x) = component {
        //                 println!("Path: {:?}", x);
        //             }
        //         }
        //         false
        //     },
        //     Err(_) => false,
        // }
        false
    }

    /// Creates the given directory and any parent directories needed, handling path expansion and
    /// returning an absolute path created.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn mkdir_p<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        let abs = self.abs(path.as_ref())?;
        let entry = &self.root;

        // Build up the path to report with
        let mut path = PathBuf::from(Component::RootDir.as_os_str());

        for component in abs.components() {
            if let Component::Normal(x) = component {
                path.push(&x);

                // Non directories are invalid at this point
                if !entry.is_dir() {
                    return Err(PathError::IsNotDir(path).into());
                }

                // Lookup the given directory component
                //entry.dir.read().unwrap().get();
                println!("Path: {:?}", x);
            }
        }
        Ok(path)
    }

    // Get the indicated entry if it exists
    pub(crate) fn get<T: AsRef<Path>>(&self, path: T) -> RvResult<MemfsEntry>
    {
        // let path = self.abs(path.as_ref())?;
        // let fs = self.fs.read().unwrap();

        // for component in path.components() {
        //     if let Component::Normal(x) = component {
        //         println!("Path: {:?}", x);
        //     }
        // }
        Err(PathError::Empty.into())
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
        Stdfs::abs(path)
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
        // TODO: check if the file's parent directories exist
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
    fn test_read_write_file() -> RvResult<()>
    {
        let memfs = Memfs::new();
        memfs.write_all(Path::new("foo"), b"foobar")?;

        Ok(())
    }

    #[test]
    fn test_add_remove() -> RvResult<()>
    {
        // Add a file to a directory
        let mut memfile1 = MemfsEntryOpts::new("/").entry();
        assert_eq!(memfile1.dir.write().unwrap().len(), 0);
        let memfile2 = MemfsEntryOpts::new("foo").entry();
        memfile1.add(memfile2.clone())?;
        assert_eq!(memfile1.dir.write().unwrap().len(), 1);

        // Remove a file from a directory
        memfile1.remove(&memfile2)?;
        assert_eq!(memfile1.dir.write().unwrap().len(), 0);
        Ok(())
    }

    #[test]
    fn test_memfs_cwd() -> RvResult<()>
    {
        let memfs = Memfs::new();
        assert_eq!(memfs.cwd()?, PathBuf::from("/"));
        Ok(())
    }
}

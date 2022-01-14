use std::{
    fmt,
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::{
    errors::*,
    exts::*,
    sys::{self, Entry, FileSystem, MemfsEntry, MemfsEntryOpts, Vfs},
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

    /// Implementation for `exists`
    ///
    /// # Arguments
    /// * `parent` - entry to potentially add the target to
    /// * `abs` - target path expected to already be in absolute form
    fn exists_recurse(parent: &MemfsEntry, abs: &Path) -> RvResult<bool>
    {
        for component in sys::trim_prefix(&abs, &parent.path).components() {
            // Using if let here to ensure that we don't consider the first slash at any depth
            if let Component::Normal(x) = component {
                if parent.child_exists(x)? {
                    return Memfs::exists_recurse(&parent.files.read().unwrap()[&x.to_string()?], abs);
                } else {
                    return Err(PathError::does_not_exist(x).into());
                }
            }
        }
        Ok(true)
    }

    /// Creates the given directory and any parent directories needed, handling path expansion and
    /// returning an absolute path created.
    ///
    /// # Arguments
    /// * `path` - the directory path to create
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn set_cwd<T: AsRef<Path>>(&mut self, path: T) -> RvResult<()>
    {
        self.cwd = self.abs(path.as_ref())?;
        Ok(())
    }
}

impl fmt::Display for Memfs
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        writeln!(f, "cwd: {}", &self.cwd.display())?;
        write!(f, "root: ")?;
        self.root.display(f, None)
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
            let mut curr = self.cwd()?;
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

    /// Returns the current working directory as a [`PathBuf`].
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn cwd(&self) -> RvResult<PathBuf>
    {
        Ok(self.cwd.clone())
    }

    /// Returns true if the `Path` exists. Handles path expansion.
    ///
    /// # Arguments
    /// * `path` - the directory path to validate exists
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let mut memfs = Memfs::new();
    /// assert_eq!(memfs.exists("foo"), false);
    /// memfs.mkdir_p("foo").unwrap();
    /// assert_eq!(memfs.exists("foo"), true);
    /// ```
    fn exists(&self, path: &Path) -> bool
    {
        match self.abs(path) {
            Ok(abs) => Memfs::exists_recurse(&self.root, &abs).is_ok(),
            Err(_) => false,
        }
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
    /// let mut memfs = Memfs::new();
    /// assert_eq!(memfs.exists("foo"), false);
    /// memfs.mkdir_p("foo").unwrap();
    /// assert_eq!(memfs.exists("foo"), true);
    /// ```
    fn mkdir_p(&mut self, path: &Path) -> RvResult<PathBuf>
    {
        let abs = self.abs(path)?;
        self.root.mkdir_p_recurse(&abs)?;
        Ok(abs)
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

    /// Write the given data to to the target file creating the file first if it doesn't exist or
    /// truncating it first if it does.
    ///
    /// # Arguments
    /// * `path` - target file to create or overwrite
    /// * `data` - data to write to the target file
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
    use std::{
        sync::{Arc, RwLock},
        thread,
        time::Duration,
    };

    use crate::prelude::*;

    #[test]
    fn test_read_write_file() -> RvResult<()>
    {
        let memfs = Memfs::new();
        memfs.write_all(Path::new("foo"), b"foobar")?;

        Ok(())
    }

    #[test]
    fn test_memfs_cwd()
    {
        let mut memfs = Memfs::new();
        assert_eq!(memfs.cwd().unwrap(), PathBuf::from("/"));

        assert_eq!(memfs.exists(Path::new("foo")), false);
        assert_eq!(memfs.exists(Path::new("foo/bar")), false);
        memfs.set_cwd("foo").unwrap();
        memfs.mkdir_p(Path::new("bar")).unwrap();
        assert_eq!(memfs.exists(Path::new("foo")), false);
        assert_eq!(memfs.exists(Path::new("/foo")), true);
        assert_eq!(memfs.exists(Path::new("/foo/bar")), true);
    }

    #[test]
    fn test_memfs_mkdir_p()
    {
        let mut memfs = Memfs::new();

        // Check single top level
        assert_eq!(memfs.exists(Path::new("foo")), false);
        memfs.mkdir_p(Path::new("foo")).unwrap();
        assert_eq!(memfs.exists(Path::new("foo")), true);
        assert_eq!(memfs.exists(Path::new("/foo")), true);

        // Check nested
        memfs.mkdir_p(Path::new("/bar/blah/ugh")).unwrap();
        assert_eq!(memfs.exists(Path::new("bar/blah/ugh")), true);
        assert_eq!(memfs.exists(Path::new("/bar/blah/ugh")), true);
        assert_eq!(memfs.exists(Path::new("/foo")), true);
    }

    #[test]
    fn test_memfs_mkdir_p_multi_threaded()
    {
        let memfs1 = Arc::new(RwLock::new(Memfs::new()));
        let memfs2 = memfs1.clone();

        // Add a directory in another thread
        let thread = thread::spawn(move || {
            memfs2.write().unwrap().mkdir_p(Path::new("foo")).unwrap();
        });

        // Wait for the directory to exist in the main thread
        while !memfs1.read().unwrap().exists(Path::new("foo")) {
            thread::sleep(Duration::from_millis(5));
        }
        thread.join().unwrap();
    }
}

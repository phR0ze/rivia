use std::{
    collections::HashMap,
    fmt,
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
};

use itertools::Itertools;

use super::MemfsEntryIter;
use crate::{
    errors::*,
    exts::*,
    sys::{self, Entries, Entry, EntryIter, FileSystem, MemfsEntry, PathExt, Vfs, VfsEntry},
};

/// `Memfs` is a Vfs backend implementation that is purely memory based. `Memfs` is multi-thread
/// safe providing internal locking when necessary.
#[derive(Debug)]
pub struct Memfs(RwLock<MemfsInner>);

// Encapsulate the Memfs implementation to allow Memfs to transparently handle interior mutability
// and be multi-thread safe.
#[derive(Debug)]
pub(crate) struct MemfsInner
{
    pub(crate) cwd: PathBuf,  // Current working directory
    pub(crate) root: PathBuf, // Current root directory
    pub(crate) fs: HashMap<PathBuf, MemfsEntry>, /* Filesystem of path to entry
                               * pub(crate) data: HashMap<PathBuf, MemfsFile>, // Filesystem of path to entry */
}

impl Memfs
{
    /// Create a new Memfs instance
    pub fn new() -> Self
    {
        let mut root = PathBuf::new();
        root.push(Component::RootDir);

        let mut files = HashMap::new();
        files.insert(root.clone(), MemfsEntry::opts(root.clone()).new());

        Self(RwLock::new(MemfsInner {
            cwd: root.clone(),
            root,
            fs: files,
        }))
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
    pub fn set_cwd<T: AsRef<Path>>(&self, path: T) -> RvResult<()>
    {
        let abs = self.abs(path.as_ref())?;
        let mut fs = self.0.write().unwrap();
        fs.cwd = abs;
        Ok(())
    }

    /// Get a clone of the target entry. Handles converting path to absolute form.
    /// # Errors
    /// * PathError::DoesNotExist(PathBuf) when this entry doesn't exist
    pub(crate) fn get<T: AsRef<Path>>(&self, path: T) -> RvResult<MemfsEntry>
    {
        let abs = self.abs(path.as_ref())?;
        let guard = self.0.read().unwrap();

        match guard.fs.get(&abs) {
            Some(entry) => Ok(entry.clone()),
            None => Err(PathError::does_not_exist(&abs).into()),
        }
    }

    /// Clone the inner structure
    pub(crate) fn inner(&self) -> MemfsInner
    {
        let guard = self.0.read().unwrap();
        MemfsInner {
            cwd: guard.cwd.clone(),
            root: guard.root.clone(),
            fs: guard.fs.clone(),
        }
    }

    /// Returns the current root directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub(crate) fn root(&self) -> PathBuf
    {
        let guard = self.0.read().unwrap();
        guard.root.clone()
    }
}

impl fmt::Display for Memfs
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let guard = self.0.read().unwrap();
        writeln!(f, "cwd: {}", guard.cwd.display())?;
        writeln!(f, "root: {}", guard.root.display())?;
        for key in guard.fs.keys().sorted() {
            writeln!(f, "{}", key.display())?;
        }
        Ok(())
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
    fn abs<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        // Check for empty string
        if sys::is_empty(path.as_ref()) {
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
        let fs = self.0.read().unwrap();
        Ok(fs.cwd.clone())
    }

    /// Returns an iterator over the given path
    fn entries<T: AsRef<Path>>(&self, path: T) -> RvResult<Entries>
    {
        // Clone entry and memfs
        let entry = self.get(path)?;
        let memfs = Arc::new(self.inner());

        // Create closure with shared memfs instance
        let iter_func = move |path: &Path, follow: bool| -> RvResult<EntryIter> {
            let memfs = memfs.clone();
            Ok(EntryIter {
                path: path.to_path_buf(),
                cached: false,
                following: follow,
                iter: Box::new(MemfsEntryIter::new(path, memfs)?),
            })
        };

        Ok(Entries {
            root: entry.upcast(),
            dirs: false,
            files: false,
            follow: false,
            min_depth: 0,
            max_depth: std::usize::MAX,
            max_descriptors: sys::DEFAULT_MAX_DESCRIPTORS,
            dirs_first: false,
            files_first: false,
            contents_first: false,
            sort_by_name: false,
            pre_op: None,
            sort: None,
            iter_from: Box::new(iter_func),
        })
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
    fn exists<T: AsRef<Path>>(&self, path: T) -> bool
    {
        let abs = unwrap_or_false!(self.abs(path));
        let guard = self.0.read().unwrap();

        guard.fs.contains_key(&abs)
    }

    /// Creates the given directory and any parent directories needed, handling path expansion and
    /// returning the absolute path of the created directory
    ///
    /// # Arguments
    /// * `path` - the target directory to create
    ///
    /// # Errors
    /// * PathError::IsNotDir(PathBuf) when this entry already exists and is not a directory.
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
    fn mkdir_p<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        let abs = self.abs(path)?;
        let mut guard = self.0.write().unwrap();

        // Check each component along the way
        let mut path = PathBuf::new();
        for component in abs.components() {
            path.push(component);
            if let Some(entry) = guard.fs.get(&path) {
                // No component should be anything other than a directory
                if !entry.is_dir() {
                    return Err(PathError::is_not_dir(&path).into());
                }
            } else {
                // Add new entry
                guard.fs.insert(path.clone(), MemfsEntry::opts(&path).new());

                // Update the parent directory
                if let Some(entry) = guard.fs.get_mut(&path.dir()?) {
                    entry.add(component.to_string()?)?;
                }
            }
        }
        Ok(abs)
    }

    /// Read all data from the given file and return it as a String
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn read_all<T: AsRef<Path>>(&self, path: T) -> RvResult<String>
    {
        let path = self.abs(path)?;
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
    fn write_all<T: AsRef<Path>>(&self, path: T, data: &[u8]) -> RvResult<()>
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
    use std::{sync::Arc, thread, time::Duration};

    use crate::prelude::*;

    #[test]
    fn test_iter_over_entries()
    {
        let memfs = Memfs::new();
        memfs.mkdir_p(Path::new("foo/bar/blah")).unwrap();
        let mut iter = memfs.entries(Path::new("/")).unwrap().into_iter();

        assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/"));
        assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/foo"));
        assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/foo/bar"));
        assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/foo/bar/blah"));
        assert_eq!(iter.next().is_none(), true);
    }

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
        let memfs = Memfs::new();
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
        let memfs = Memfs::new();

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
        let memfs1 = Arc::new(Memfs::new());
        let memfs2 = memfs1.clone();

        // Add a directory in another thread
        let thread = thread::spawn(move || {
            memfs2.mkdir_p(Path::new("foo")).unwrap();
        });

        // Wait for the directory to exist in the main thread
        while !memfs1.exists(Path::new("foo")) {
            thread::sleep(Duration::from_millis(5));
        }
        thread.join().unwrap();
    }
}

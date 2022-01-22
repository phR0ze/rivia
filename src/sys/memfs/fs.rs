use std::{
    collections::HashMap,
    fmt,
    io::{Read, Seek, Write},
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
};

use itertools::Itertools;

use super::{MemfsEntry, MemfsEntryIter, MemfsFile};
use crate::{
    errors::*,
    exts::*,
    sys::{self, Entries, Entry, EntryIter, FileSystem, PathExt, Vfs},
};

// Helper aliases
pub(crate) type MemfsFiles = HashMap<PathBuf, MemfsFile>;
pub(crate) type MemfsEntries = HashMap<PathBuf, MemfsEntry>;

/// `Memfs` is a Vfs backend implementation that is purely memory based. `Memfs` is multi-thread
/// safe providing internal locking when necessary.
#[derive(Debug)]
pub struct Memfs(RwLock<MemfsInner>);

// Encapsulate the Memfs implementation to allow Memfs to transparently handle interior mutability
// and be multi-thread safe.
#[derive(Debug)]
pub(crate) struct MemfsInner
{
    pub(crate) cwd: PathBuf,     // Current working directory
    pub(crate) root: PathBuf,    // Current root directory
    pub(crate) fs: MemfsEntries, // Filesystem of path to entry
    pub(crate) data: MemfsFiles, // Filesystem of path to entry
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
            data: HashMap::new(),
        }))
    }

    /// Get a clone of the target entry. Handles converting path to absolute form.
    /// # Errors
    /// * PathError::DoesNotExist(PathBuf) when this entry doesn't exist
    pub(crate) fn clone_entry<T: AsRef<Path>>(&self, path: T) -> RvResult<MemfsEntry>
    {
        let abs = self.abs(path.as_ref())?;
        let guard = self.0.read().unwrap();

        match guard.fs.get(&abs) {
            Some(entry) => Ok(entry.clone()),
            None => Err(PathError::does_not_exist(&abs).into()),
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
        writeln!(f, "[cwd]: {}", guard.cwd.display())?;
        writeln!(f, "[root]: {}", guard.root.display())?;
        writeln!(f, "\n[fs]:")?;
        for key in guard.fs.keys().sorted() {
            writeln!(f, "{}", key.display())?;
        }
        writeln!(f, "\n[files]:")?;
        for key in guard.data.keys().sorted() {
            writeln!(f, "{}", key.display())?;
        }
        Ok(())
    }
}

impl FileSystem for Memfs
{
    /// Return the path in an absolute clean form
    ///
    /// ### Provides:
    /// * environment variable expansion
    /// * relative path resolution for `.` and `..`
    /// * no IO resolution so it will work even with paths that don't exist
    ///
    /// ### Errors
    /// * PathError::ParentNotFound(PathBuf) when parent is not found
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let home = sys::home_dir().unwrap();
    /// assert_eq!(memfs.abs("~").unwrap(), PathBuf::from(&home));
    /// ```
    fn abs<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        let path = path.as_ref();

        // Check for empty string
        if path.is_empty() {
            return Err(PathError::Empty.into());
        }

        // Expand home directory
        let mut path_buf = path.expand()?;

        // Trim protocol prefix if needed
        path_buf = path_buf.trim_protocol();

        // Clean the resulting path
        path_buf = path_buf.clean();

        // Expand relative directories if needed
        if !path_buf.is_absolute() {
            let mut curr = self.cwd()?;
            while let Ok(path) = path_buf.components().first_result() {
                match path {
                    Component::CurDir => {
                        path_buf = path_buf.trim_first();
                    },
                    Component::ParentDir => {
                        if curr.to_string()? == "/" {
                            return Err(PathError::ParentNotFound(curr).into());
                        }
                        curr = curr.dir()?;
                        path_buf = path_buf.trim_first();
                    },
                    _ => return Ok(curr.mash(path_buf)),
                };
            }
            return Ok(curr);
        }

        Ok(path_buf)
    }

    /// Returns the current working directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// assert_eq!(memfs.cwd().unwrap(), PathBuf::from("/"));
    /// assert_eq!(memfs.mkdir_p("foo").unwrap(), PathBuf::from("/foo"));
    /// assert_eq!(memfs.set_cwd("foo").unwrap(), PathBuf::from("/foo"))
    /// assert_eq!(memfs.cwd().unwrap(), PathBuf::from("/foo"));
    /// ```
    fn cwd(&self) -> RvResult<PathBuf>
    {
        let fs = self.0.read().unwrap();
        Ok(fs.cwd.clone())
    }

    /// Set the current working directory
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * relative path will use the current working directory
    ///
    /// ### Errors
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// assert_eq!(memfs.cwd().unwrap(), PathBuf::from("/"));
    /// assert_eq!(memfs.mkdir_p("foo").unwrap(), PathBuf::from("/foo"));
    /// assert_eq!(memfs.set_cwd("foo").unwrap(), PathBuf::from("/foo"))
    /// assert_eq!(memfs.cwd().unwrap(), PathBuf::from("/foo"));
    /// ```
    fn set_cwd<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        let path = self.abs(path)?;
        let mut guard = self.0.write().unwrap();
        if !guard.fs.contains_key(&path) {
            return Err(PathError::does_not_exist(&path).into());
        }
        guard.cwd = path.clone();
        Ok(path)
    }

    /// Returns an iterator over the given path
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    /// * recursive path traversal
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// assert_eq!(memfs.mkdir_p("foo").unwrap(), PathBuf::from("/foo"));
    /// assert_eq!(memfs.mkfile("foo/file").unwrap(), PathBuf::from("/foo/file"));
    /// let mut iter = memfs.entries("/").unwrap().into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/"));
    /// assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/foo"));
    /// assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/foo/file"));
    /// assert!(iter.next().is_none());
    /// ```
    fn entries<T: AsRef<Path>>(&self, path: T) -> RvResult<Entries>
    {
        // Create closure with cloned shared memfs instance
        let guard = self.0.read().unwrap();
        let entries = Arc::new(guard.fs.clone());
        let iter_func = move |path: &Path, follow: bool| -> RvResult<EntryIter> {
            let entries = entries.clone();
            Ok(EntryIter {
                path: path.to_path_buf(),
                cached: false,
                following: follow,
                iter: Box::new(MemfsEntryIter::new(path, entries)?),
            })
        };

        Ok(Entries {
            root: self.clone_entry(path)?.upcast(),
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

    /// Returns true if the `path` exists
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let mut memfs = Memfs::new();
    /// assert_eq!(memfs.exists("foo"), false);
    /// assert_eq!(memfs.mkdir_p("foo").unwrap(), PathBuf::from("/foo"));
    /// assert_eq!(memfs.exists("foo"), true);
    /// ```
    fn exists<T: AsRef<Path>>(&self, path: T) -> bool
    {
        let abs = unwrap_or_false!(self.abs(path));
        let guard = self.0.read().unwrap();

        guard.fs.contains_key(&abs)
    }

    /// Creates the given directory and any parent directories needed
    ///
    /// ### Provides
    /// * path expansion and absolute path resolution
    ///
    /// # Errors
    /// * PathError::IsNotDir(PathBuf) when the path already exists and is not a directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let mut memfs = Memfs::new();
    /// assert_eq!(memfs.exists("foo"), false);
    /// assert_eq!(memfs.mkdir_p("foo").unwrap(), PathBuf::from("/foo"));
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

    /// Create an empty file similar to the linux touch command
    ///
    /// ### Provides
    /// * handling path expansion and absolute path resolution
    /// * default file creation permissions 0o666 with umask usually ends up being 0o644
    ///
    /// ### Errors
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotDir(PathBuf) when the given path's parent isn't a directory
    /// * PathError::IsNotFile(PathBuf) when the given path exists but isn't a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// assert_eq!(memfs.exists("file1"), false);
    /// assert_eq!(memfs.mkfile("file1").unwrap(), PathBuf::from("/file1"));
    /// assert_eq!(memfs.exists("file1"), true);
    /// ```
    fn mkfile<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        let path = self.abs(path)?;
        let mut guard = self.0.write().unwrap();

        // Validate path components
        let dir = path.dir()?;
        if let Some(entry) = guard.fs.get(&dir) {
            if !entry.is_dir() {
                return Err(PathError::is_not_dir(dir).into());
            }
        } else {
            return Err(PathError::does_not_exist(dir).into());
        }

        // Validate the path itself
        if let Some(entry) = guard.fs.get(&path) {
            if !entry.is_file() {
                return Err(PathError::is_not_file(path).into());
            }
        } else {
            // Add the new file to the file system
            guard.fs.insert(path.clone(), MemfsEntry::opts(&path).file().new());

            // Add the new file to the data system
            guard.data.insert(path.clone(), MemfsFile::default());

            // Update the parent directory
            if let Some(parent) = guard.fs.get_mut(&dir) {
                parent.add(path.base()?)?;
            }
        }

        Ok(path)
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
        let guard = self.0.read().unwrap();

        if let Some(f) = guard.data.get(&path) {
            let mut f = f.clone();
            let mut buf = String::new();
            f.read_to_string(&mut buf)?;
        } else {
            return Err(PathError::does_not_exist(&path).into());
        }

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
    fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()>
    {
        let path = self.abs(path)?;
        let dir = path.dir()?;

        // Validate the dir exists
        if !self.exists(&dir) {
            return Err(PathError::does_not_exist(&dir).into());
        }

        // Create the file if necessary
        let mut guard = self.0.write().unwrap();
        if !guard.data.contains_key(&path) {
            guard.data.insert(path.clone(), MemfsFile::default());
        }

        // Write the data to a target file
        if let Some(f) = guard.data.get_mut(&path) {
            f.rewind()?;
            f.write_all(data.as_ref())?;
            f.flush()?;
        }

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
    fn test_memfs_abs()
    {
        let memfs = Memfs::new();
        memfs.set_cwd("foo").unwrap();
        let cwd = memfs.cwd().unwrap(); // foo
        let prev = cwd.dir().unwrap(); // /

        // expand relative directory
        assert_eq!(memfs.abs("foo").unwrap(), cwd.mash("foo"));

        // expand previous directory and drop trailing slashes
        assert_eq!(memfs.abs("..//").unwrap(), prev);
        assert_eq!(memfs.abs("../").unwrap(), prev);
        assert_eq!(memfs.abs("..").unwrap(), prev);

        // expand current directory and drop trailing slashes
        assert_eq!(memfs.abs(".//").unwrap(), cwd);
        assert_eq!(memfs.abs("./").unwrap(), cwd);
        assert_eq!(memfs.abs(".").unwrap(), cwd);

        // home dir
        let home = sys::home_dir().unwrap();
        assert_eq!(memfs.abs("~").unwrap(), home);
        assert_eq!(memfs.abs("~/").unwrap(), home);

        // expand home path
        assert_eq!(memfs.abs("~/foo").unwrap(), home.mash("foo"));

        // More complicated
        assert_eq!(memfs.abs("~/foo/bar/../.").unwrap(), home.mash("foo"));
        assert_eq!(memfs.abs("~/foo/bar/../").unwrap(), home.mash("foo"));
        assert_eq!(memfs.abs("~/foo/bar/../blah").unwrap(), home.mash("foo/blah"));

        // Move up the path multiple levels
        assert_eq!(memfs.abs("/foo/bar/blah/../../foo1").unwrap(), PathBuf::from("/foo/foo1"));
        assert_eq!(memfs.abs("/../../foo").unwrap(), PathBuf::from("/foo"));

        // Move up until invalid
        assert_eq!(
            memfs.abs("../../../../../../../foo").unwrap_err().to_string(),
            PathError::ParentNotFound(PathBuf::from("/")).to_string()
        );
    }

    #[test]
    fn test_memfs_cwd()
    {
        let memfs = Memfs::new();
        assert_eq!(memfs.cwd().unwrap(), PathBuf::from("/"));

        assert_eq!(memfs.exists("foo"), false);
        assert_eq!(memfs.exists("foo/bar"), false);
        memfs.set_cwd("foo").unwrap();
        memfs.mkdir_p("bar").unwrap();
        assert_eq!(memfs.exists("foo"), false);
        assert_eq!(memfs.exists("/foo"), true);
        assert_eq!(memfs.exists("/foo/bar"), true);
    }

    #[test]
    fn test_memfs_entries()
    {
        let memfs = Memfs::new();
        assert_eq!(memfs.mkdir_p("foo/bar").unwrap(), PathBuf::from("/foo/bar"));
        assert_eq!(memfs.mkfile("foo/bar/blah").unwrap(), PathBuf::from("/foo/bar/blah"));

        println!("{}", memfs);
        let mut iter = memfs.entries("/").unwrap().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/"));
        assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/foo"));
        assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/foo/bar"));
        assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/foo/bar/blah"));
        assert_eq!(iter.next().is_none(), true);
    }

    #[test]
    fn test_memfs_mkfile()
    {
        let memfs = Memfs::new();

        // Error: directory doesn't exist
        let err = memfs.mkfile("dir1/file1").unwrap_err();
        assert_eq!(err.downcast_ref::<PathError>().unwrap(), &PathError::does_not_exist("/dir1"));

        // Error: target exists and is not a file
        memfs.mkdir_p("dir1").unwrap();
        let err = memfs.mkfile("dir1").unwrap_err();
        assert_eq!(err.downcast_ref::<PathError>().unwrap(), &PathError::is_not_file("/dir1"));

        // Make a file in the root
        assert_eq!(memfs.exists("file1"), false);
        assert_eq!(memfs.mkfile("file1").unwrap(), PathBuf::from("/file1"));
        assert_eq!(memfs.exists("file1"), true);

        // Make a file in a directory
        assert_eq!(memfs.exists("dir1/file2"), false);
        assert_eq!(memfs.mkfile("dir1/file2").unwrap(), PathBuf::from("/dir1/file2"));
        assert_eq!(memfs.exists("dir1/file2"), true);

        // Error: parent exists and is not a directory
        let err = memfs.mkfile("file1/file2").unwrap_err();
        assert_eq!(err.downcast_ref::<PathError>().unwrap(), &PathError::is_not_dir("/file1"));
    }

    #[test]
    fn test_memfs_mkdir_p()
    {
        let memfs = Memfs::new();

        // Check single top level
        assert_eq!(memfs.exists("foo"), false);
        memfs.mkdir_p("foo").unwrap();
        assert_eq!(memfs.exists("foo"), true);
        assert_eq!(memfs.exists("/foo"), true);

        // Check nested
        memfs.mkdir_p("/bar/blah/ugh").unwrap();
        assert_eq!(memfs.exists("bar/blah/ugh"), true);
        assert_eq!(memfs.exists("/bar/blah/ugh"), true);
        assert_eq!(memfs.exists("/foo"), true);
    }

    #[test]
    fn test_memfs_mkdir_p_multi_threaded()
    {
        let memfs1 = Arc::new(Memfs::new());
        let memfs2 = memfs1.clone();

        // Add a directory in another thread
        let thread = thread::spawn(move || {
            memfs2.mkdir_p("foo").unwrap();
        });

        // Wait for the directory to exist in the main thread
        while !memfs1.exists("foo") {
            thread::sleep(Duration::from_millis(5));
        }
        thread.join().unwrap();
    }

    // #[test]
    // fn test_add_remove() -> RvResult<()>
    // {
    //     // Add a file to a directory
    //     let mut memfile1 = MemfsEntry::opts("/").new();
    //     assert_eq!(memfile1.entries.len(), 0);
    //     let memfile2 = MemfsEntry::opts("/foo").new();
    //     memfile1.add_child(memfile2.clone())?;
    //     assert_eq!(memfile1.entries.len(), 1);

    //     // Remove a file from a directory
    //     assert_eq!(memfile1.remove_child(&memfile2.path)?, Some(memfile2));
    //     assert_eq!(memfile1.entries.len(), 0);
    //     Ok(())
    // }

    // #[test]
    // fn test_remove_non_existing()
    // {
    //     let mut memfile = MemfsEntry::opts("foo").new();
    //     assert_eq!(memfile.remove_child("blah").unwrap(), None);
    // }

    // #[test]
    // fn test_remove_from_file_fails()
    // {
    //     let mut memfile = MemfsEntry::opts("foo").file().new();
    //     assert_eq!(memfile.remove_child("bar").unwrap_err().to_string(), "Target path is not a
    // directory: foo"); }

    // #[test]
    // fn test_add_already_exists_fails()
    // {
    //     let mut memfile1 = MemfsEntry::opts("/").new();
    //     let memfile2 = MemfsEntry::opts("/foo").file().new();
    //     memfile1.add_child(memfile2.clone()).unwrap();
    //     assert_eq!(memfile1.add_child(memfile2).unwrap_err().to_string(), "Target path exists
    // already: /foo"); }

    // #[test]
    // fn test_add_mismatch_path_fails()
    // {
    //     let mut memfile1 = MemfsEntry::opts("/").new();
    //     let memfile2 = MemfsEntry::opts("foo").file().new();
    //     assert_eq!(memfile1.add_child(memfile2).unwrap_err().to_string(), "Target path's
    // directory doesn't match parent: /"); }

    // #[test]
    // fn test_add_to_link_fails()
    // {
    //     let mut memfile = MemfsEntry::opts("foo").link().new();
    //     assert_eq!(memfile.add_child(MemfsEntry::opts("").new()).unwrap_err().to_string(),
    // "Target path filename not found: "); }

    // #[test]
    // fn test_add_to_file_fails()
    // {
    //     let mut memfile = MemfsEntry::opts("foo").file().new();
    //     assert_eq!(memfile.add_child(MemfsEntry::opts("").new()).unwrap_err().to_string(),
    // "Target path is not a directory: foo"); }

    // #[test]
    // fn test_ordering_and_equality()
    // {
    //     let entry1 = MemfsEntry::opts("1").new();
    //     let entry2 = MemfsEntry::opts("2").new();
    //     let entry3 = MemfsEntry::opts("3").new();

    //     let mut entries = vec![&entry1, &entry3, &entry2];
    //     entries.sort();

    //     assert_eq!(entries[0], &entry1);
    //     assert_ne!(entries[1], &entry3);
    //     assert_eq!(entries[1], &entry2);
    //     assert_eq!(entries[2], &entry3);
    // }

    // #[test]
    // fn test_not_readable_writable_file() -> RvResult<()>
    // {
    //     let mut memfile = MemfsEntry::opts("foo").new();

    //     // Not readable
    //     let mut buf = [0; 1];
    //     assert_eq!(memfile.read(&mut buf).unwrap_err().to_string(), "Target path 'foo' is not a
    // readable file");

    //     // Not writable
    //     assert_eq!(memfile.write(b"foobar1, ").unwrap_err().to_string(), "Target path 'foo' is
    // not a writable file");     Ok(())
    // }
}

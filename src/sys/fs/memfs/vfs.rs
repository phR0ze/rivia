use std::{
    collections::HashMap,
    fmt,
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
};

use itertools::Itertools;

use super::{MemfsEntry, MemfsEntryIter, MemfsFile};
use crate::{
    core::*,
    errors::*,
    sys::{self, Chmod, Entries, Entry, EntryIter, Mode, PathExt, ReadSeek, Vfs, VfsEntry, VirtualFileSystem},
};

// Helper aliases
pub(crate) type MemfsFiles = HashMap<PathBuf, MemfsFile>;
pub(crate) type MemfsEntries = HashMap<PathBuf, MemfsEntry>;

/// Provides a purely memory based, multi-thread safe [`VirtualFileSystem`] backend implementation
#[derive(Debug)]
pub struct Memfs(Arc<RwLock<MemfsInner>>);

// Encapsulate the Memfs implementation for interior mutability and transparent multi-thread safety
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

        // Add the default root entry
        let mut files = HashMap::new();
        files.insert(root.clone(), MemfsEntry::opts(root.clone()).new());

        Self(Arc::new(RwLock::new(MemfsInner {
            cwd: root.clone(),
            root,
            fs: files,
            data: HashMap::new(),
        })))
    }

    /// Get a clone of the target entry
    ///
    /// * Handles converting path to absolute form
    /// * Returns a PathError::DoesNotExist(PathBuf) when this entry doesn't exist
    pub(crate) fn clone_entry<T: AsRef<Path>>(&self, path: T) -> RvResult<MemfsEntry>
    {
        let abs = self.abs(path.as_ref())?;
        let guard = self.0.read().unwrap();

        match guard.fs.get(&abs) {
            Some(entry) => Ok(entry.clone()),
            None => Err(PathError::does_not_exist(&abs).into()),
        }
    }

    /// Get a clone of the target file
    ///
    /// * Handles converting path to absolute form
    /// * Returns a PathError::DoesNotExist(PathBuf) when this file doesn't exist
    pub(crate) fn clone_file<T: AsRef<Path>>(&self, path: T) -> RvResult<MemfsFile>
    {
        let path = self.abs(path.as_ref())?;
        let guard = self.0.read().unwrap();

        // Validate target is a file
        if let Some(f) = guard.fs.get(&path) {
            if !f.is_file() {
                return Err(PathError::is_not_file(&path).into());
            }
        }

        // Clone the file if it exists
        match guard.data.get(&path) {
            Some(entry) => Ok(entry.clone()),
            None => Err(PathError::does_not_exist(&path).into()),
        }
    }

    /// Create the given MemfsEntry if it doesn't already exist
    ///
    /// * Expects the entry's path to already be in absolute form
    ///
    /// ### Errors
    /// * PathError::IsNotDir(PathBuf) when the given path's parent exists but is not a directory
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotFile(PathBuf) when the given path exists but is not a file
    pub(crate) fn add(&self, entry: MemfsEntry) -> RvResult<PathBuf>
    {
        let path = entry.path.clone();
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
        if let Some(x) = guard.fs.get(&path) {
            if entry.is_file() && !x.is_file() {
                return Err(PathError::is_not_file(path).into());
            } else if entry.is_symlink() && !x.is_symlink() {
                return Err(PathError::is_not_symlink(path).into());
            }
        } else {
            // Add the new file to the data system if not a link
            if entry.is_file() {
                guard.data.insert(path.clone(), MemfsFile::default());
            }

            // Add the new file/link to the file system
            guard.fs.insert(path.clone(), entry);

            // Update the parent directory
            if let Some(parent) = guard.fs.get_mut(&dir) {
                parent.add(path.base()?)?;
            }
        }

        Ok(path)
    }

    // Execute chmod with the given [`Mode`] options
    fn _chmod(&self, mode: Mode) -> RvResult<()>
    {
        // Using `contents_first` to yield directories last so that revoking permissions happen to
        // directories as the last thing when completing the traversal, else we'll lock
        // ourselves out.
        let mut entries = self.entries(&mode.path)?.contents_first();

        // Set the `max_depth` based on recursion
        entries = entries.max_depth(match mode.recursive {
            true => std::usize::MAX,
            false => 0,
        });

        // Set `follow` as directed
        if mode.follow {
            entries = entries.follow();
        }

        // Using `dirs_first` and `pre_op` options here to grant addative permissions as a
        // pre-traversal operation to allow for the possible addition of permissions that would allow
        // directory traversal that otherwise wouldn't be allowed.
        let m = mode.clone();
        let vfs = Memfs(self.0.clone());
        entries = entries.dirs_first().pre_op(move |x| {
            let m1 = sys::mode(x, m.dirs, &m.sym)?;
            if (!x.is_symlink() || m.follow) && x.is_dir() && !sys::revoking_mode(x.mode(), m1) && x.mode() != m1 {
                let mut guard = vfs.0.write().unwrap();
                if let Some(entry) = guard.fs.get_mut(x.path()) {
                    entry.set_mode(m1);
                }
            }
            Ok(())
        });

        // Set permissions on the way out for everything specified
        for entry in entries {
            let src = entry?;

            // Compute mode based on octal and symbolic values
            let m2 = if src.is_dir() {
                sys::mode(&src, mode.dirs, &mode.sym)?
            } else if src.is_file() {
                sys::mode(&src, mode.files, &mode.sym)?
            } else {
                0
            };

            // Apply permission to entry if set
            if (!src.is_symlink() || mode.follow) && m2 != src.mode() && m2 != 0 {
                let mut guard = self.0.write().unwrap();
                if let Some(entry) = guard.fs.get_mut(src.path()) {
                    entry.set_mode(m2);
                }
            }
        }
        Ok(())
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
            write!(f, "{}", key.display())?;
            if guard.fs[key].link {
                write!(f, " -> {}", guard.fs[key].alt().display())?;
            }
            writeln!(f)?;
        }
        writeln!(f, "\n[files]:")?;
        for key in guard.data.keys().sorted() {
            writeln!(f, "{}", key.display())?;
        }
        Ok(())
    }
}

impl VirtualFileSystem for Memfs
{
    /// Return the path in an absolute clean form
    ///
    /// * Handles environment variable expansion
    /// * Handles relative path resolution for `.` and `..`
    /// * No IO resolution so it will work even with paths that don't exist
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

    /// Opens a file in append mode
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Creates a file if it does not exist or appends to it if it does
    ///
    /// ### Errors
    /// * PathError::IsNotDir(PathBuf) when the given path's parent exists but is not a directory
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotFile(PathBuf) when the given path exists but is not a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let file = memfs.root().mash("file");
    /// let mut f = memfs.create(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// let mut f = memfs.append(&file).unwrap();
    /// f.write_all(b"123").unwrap();
    /// f.flush().unwrap();
    /// assert_eq!(memfs.read_all(&file).unwrap(), "foobar123".to_string());
    /// ```
    fn append<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>>
    {
        // Make all the pre-flight validation checks and ensure the file exists.
        let path = self.abs(path)?;
        self.add(MemfsEntry::opts(&path).file().new())?;

        let guard = self.0.read().unwrap();
        if let Some(file) = guard.data.get(&path) {
            // Clone the file to append to
            let mut clone = file.clone();
            clone.path = Some(path.clone());
            clone.fs = Some(self.0.clone());

            // Seek to the end for appending
            clone.seek(SeekFrom::End(0))?;
            Ok(Box::new(clone))
        } else {
            return Err(PathError::does_not_exist(path).into());
        }
    }

    /// Change all file/dir permissions recursivly to `mode`
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Doesn't follow links by default, use the builder `chomd_b` for this option
    ///
    /// ### Errors
    /// * PathError::Empty when the given path is empty
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod(&file, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    /// ```
    fn chmod<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<()>
    {
        self.chmod_b(path)?.all(mode).exec()
    }

    /// Returns a new [`Chmod`] builder for advanced chmod options
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides options for recursion, following links, narrowing in on file types etc...
    ///
    /// ### Errors
    /// * PathError::Empty when the given path is empty
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40755);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod_b(&dir).unwrap().recurse().all(0o777).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40777);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100777);
    /// ```
    fn chmod_b<T: AsRef<Path>>(&self, path: T) -> RvResult<Chmod>
    {
        let path = self.abs(path)?;

        // Construct the chmod closure callback
        let vfs = Memfs(self.0.clone());
        let exec_func = move |mode: Mode| -> RvResult<()> { vfs._chmod(mode) };

        // Return the new Chmod builder
        Ok(Chmod {
            mode: Mode {
                path,
                dirs: 0,
                files: 0,
                follow: false,
                recursive: true,
                sym: "".to_string(),
            },
            exec: Box::new(exec_func),
        })
    }

    /// Opens a file in write-only mode
    ///
    /// * Creates a file if it does not exist or truncates it if it does
    ///
    /// ### Errors
    /// * PathError::IsNotDir(PathBuf) when the given path's parent exists but is not a directory
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotFile(PathBuf) when the given path exists but is not a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let file = memfs.root().mash("file");
    /// let mut f = memfs.create(&file).unwrap();
    /// f.write_all(b"foobar").unwrap();
    /// f.flush().unwrap();
    /// assert_eq!(memfs.read_all(&file).unwrap(), "foobar".to_string());
    /// ```
    fn create<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn Write>>
    {
        // Make all the pre-flight validation checks and ensure the file exists.
        let path = self.abs(path)?;
        self.add(MemfsEntry::opts(&path).file().new())?;

        // Create an empty file to write to
        Ok(Box::new(MemfsFile {
            pos: 0,
            data: vec![],
            path: Some(path),
            fs: Some(self.0.clone()),
        }))
    }

    /// Returns the current working directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// assert_eq!(memfs.cwd().unwrap(), memfs.root());
    /// assert_eq!(memfs.mkdir_p("foo").unwrap(), memfs.root().mash("foo"));
    /// assert_eq!(memfs.set_cwd("foo").unwrap(), memfs.root().mash("foo"));
    /// assert_eq!(memfs.cwd().unwrap(), memfs.root().mash("foo"));
    /// ```
    fn cwd(&self) -> RvResult<PathBuf>
    {
        let fs = self.0.read().unwrap();
        Ok(fs.cwd.clone())
    }

    /// Returns all directories for the given path, sorted by name
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned as abs paths
    /// * Doesn't include the path itself only its children nor is this recursive
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_iter_eq(vfs.dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
    /// ```
    fn dirs<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>>
    {
        let mut paths: Vec<PathBuf> = vec![];
        if !self.is_dir(&path) {
            return Err(PathError::is_not_dir(&path).into());
        }
        for entry in self.entries(path)?.min_depth(1).max_depth(1).sort_by_name().dirs() {
            let entry = entry?;
            paths.push(entry.path_buf());
        }
        Ok(paths)
    }

    /// Returns an iterator over the given path
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Handles recursive path traversal
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let dir = memfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_eq!(&memfs.mkdir_p(&dir).unwrap(), &dir);
    /// assert_eq!(&memfs.mkfile(&file).unwrap(), &file);
    /// let mut iter = memfs.entries(memfs.root()).unwrap().into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), memfs.root());
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &dir);
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &file);
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

    /// Return a virtual filesystem entry for the given path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.entry(&file).unwrap().is_file());
    /// ```
    fn entry<T: AsRef<Path>>(&self, path: T) -> RvResult<VfsEntry>
    {
        Ok(self.clone_entry(path)?.upcast())
    }

    /// Returns true if the `path` exists
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let mut memfs = Memfs::new();
    /// assert_eq!(memfs.exists("foo"), false);
    /// assert_eq!(memfs.mkdir_p("foo").unwrap(), memfs.root().mash("foo"));
    /// assert_eq!(memfs.exists("foo"), true);
    /// ```
    fn exists<T: AsRef<Path>>(&self, path: T) -> bool
    {
        let abs = unwrap_or_false!(self.abs(path));
        let guard = self.0.read().unwrap();

        guard.fs.contains_key(&abs)
    }

    /// Returns all files for the given path, sorted by name
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned as abs paths
    /// * Doesn't include the path itself only its children nor is this recursive
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let dir1 = tmpdir.mash("dir1");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_mkfile!(vfs, &file2);
    /// assert_iter_eq(vfs.files(&tmpdir).unwrap(), vec![file1, file2]);
    /// ```
    fn files<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>>
    {
        let mut paths: Vec<PathBuf> = vec![];
        if !self.is_dir(&path) {
            return Err(PathError::is_not_dir(&path).into());
        }
        for entry in self.entries(path)?.min_depth(1).max_depth(1).sort_by_name().files() {
            let entry = entry?;
            paths.push(entry.path_buf());
        }
        Ok(paths)
    }

    /// Returns true if the given path exists and is readonly
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert!(vfs.mkfile_m(&file, 0o644).is_ok());
    /// assert_eq!(vfs.is_exec(&file), false);
    /// assert!(vfs.chmod(&file, 0o777).is_ok());
    /// assert_eq!(vfs.is_exec(&file), true);
    /// ```
    fn is_exec<T: AsRef<Path>>(&self, path: T) -> bool
    {
        let abs = unwrap_or_false!(self.abs(path));
        let guard = self.0.read().unwrap();

        match guard.fs.get(&abs) {
            Some(entry) => entry.is_exec(),
            None => false,
        }
    }

    /// Returns true if the given path exists and is a directory
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides link exclusion i.e. links even if pointing to a directory return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// assert_eq!(memfs.is_dir("foo"), false);
    /// let tmpdir = memfs.mkdir_p("foo").unwrap();
    /// assert_eq!(memfs.is_dir(&tmpdir), true);
    /// ```
    fn is_dir<T: AsRef<Path>>(&self, path: T) -> bool
    {
        let abs = unwrap_or_false!(self.abs(path));
        let guard = self.0.read().unwrap();

        match guard.fs.get(&abs) {
            Some(entry) => entry.is_dir(),
            None => false,
        }
    }

    /// Returns true if the given path exists and is a file
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides link exclusion i.e. links even if pointing to a file return false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// assert_eq!(memfs.is_file("foo"), false);
    /// let tmpfile = memfs.mkfile("foo").unwrap();
    /// assert_eq!(memfs.is_file(&tmpfile), true);
    /// ```
    fn is_file<T: AsRef<Path>>(&self, path: T) -> bool
    {
        let abs = unwrap_or_false!(self.abs(path));
        let guard = self.0.read().unwrap();

        match guard.fs.get(&abs) {
            Some(entry) => entry.is_file(),
            None => false,
        }
    }

    /// Returns true if the given path exists and is readonly
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert!(vfs.mkfile_m(&file, 0o644).is_ok());
    /// assert_eq!(vfs.is_readonly(&file), false);
    /// assert!(vfs.chmod_b(&file).unwrap().readonly().exec().is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100444);
    /// assert_eq!(vfs.is_readonly(&file), true);
    /// ```
    fn is_readonly<T: AsRef<Path>>(&self, path: T) -> bool
    {
        let abs = unwrap_or_false!(self.abs(path));
        let guard = self.0.read().unwrap();

        match guard.fs.get(&abs) {
            Some(entry) => entry.is_readonly(),
            None => false,
        }
    }

    /// Returns true if the given path exists and is a symlink
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Checks the path itself and not what is potentially pointed to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Vfs::memfs();
    /// assert_eq!(memfs.is_symlink("foo"), false);
    /// let tmpfile = memfs.symlink("foo", "bar").unwrap();
    /// assert_eq!(memfs.is_symlink(&tmpfile), true);
    /// ```
    fn is_symlink<T: AsRef<Path>>(&self, path: T) -> bool
    {
        let abs = unwrap_or_false!(self.abs(path));
        let guard = self.0.read().unwrap();

        match guard.fs.get(&abs) {
            Some(entry) => entry.is_symlink(),
            None => false,
        }
    }

    /// Creates the given directory and any parent directories needed with the given mode
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// assert!(vfs.mkdir_m(&dir, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40555);
    /// ```
    fn mkdir_m<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<PathBuf>
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
                // Add new entry using the given mode
                guard.fs.insert(path.clone(), MemfsEntry::opts(&path).set_mode(mode).new());

                // Update the parent directory
                if let Some(entry) = guard.fs.get_mut(&path.dir()?) {
                    entry.add(component.to_string()?)?;
                }
            }
        }
        Ok(abs)
    }

    /// Creates the given directory and any parent directories needed
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// # Errors
    /// * PathError::IsNotDir(PathBuf) when the path already exists and is not a directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// assert_eq!(memfs.exists("foo"), false);
    /// assert_eq!(memfs.mkdir_p("foo").unwrap(), memfs.root().mash("foo"));
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
    /// * Handles path expansion and absolute path resolution
    /// * Default file creation permissions 0o666 with umask usually ends up being 0o644
    ///
    /// ### Errors
    /// * PathError::IsNotDir(PathBuf) when the given path's parent isn't a directory
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotFile(PathBuf) when the given path exists but isn't a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// assert_eq!(memfs.exists("file1"), false);
    /// assert_eq!(memfs.mkfile("file1").unwrap(), memfs.root().mash("file1"));
    /// assert_eq!(memfs.exists("file1"), true);
    /// ```
    fn mkfile<T: AsRef<Path>>(&self, path: T) -> RvResult<PathBuf>
    {
        self.add(MemfsEntry::opts(self.abs(path)?).file().new())
    }

    /// Wraps `mkfile` allowing for setting the file's mode.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert!(vfs.mkfile_m(&file, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    /// ```
    fn mkfile_m<T: AsRef<Path>>(&self, path: T, mode: u32) -> RvResult<PathBuf>
    {
        let path = self.mkfile(path)?;
        self.chmod(&path, mode)?;
        Ok(path)
    }

    /// Returns the permissions for a file, directory or link
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * PathError::Empty when the given path is empty
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// assert!(vfs.chmod(&file, 0o555).is_ok());
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    /// ```
    fn mode<T: AsRef<Path>>(&self, path: T) -> RvResult<u32>
    {
        let path = self.abs(path)?;
        let guard = self.0.read().unwrap();

        match guard.fs.get(&path) {
            Some(entry) => Ok(entry.mode()),
            None => Err(PathError::does_not_exist(&path).into()),
        }
    }

    /// Attempts to open a file in readonly mode
    ///
    /// * Provides a handle to a Read + Seek implementation
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * PathError::IsNotFile(PathBuf) when the given path isn't a file
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let file = memfs.root().mash("file");
    /// memfs.write_all(&file, b"foobar 1").unwrap();
    /// let mut file = memfs.open(&file).unwrap();
    /// let mut buf = String::new();
    /// file.read_to_string(&mut buf);
    /// assert_eq!(buf, "foobar 1".to_string());
    /// ```
    fn open<T: AsRef<Path>>(&self, path: T) -> RvResult<Box<dyn ReadSeek>>
    {
        Ok(Box::new(self.clone_file(&path)?))
    }

    /// Returns all paths for the given path, sorted by name
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Paths are returned as abs paths
    /// * Doesn't include the path itself only its children nor is this recursive
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// let file1 = tmpdir.mash("file1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_iter_eq(vfs.paths(&tmpdir).unwrap(), vec![dir1, dir2, file1]);
    /// ```
    fn paths<T: AsRef<Path>>(&self, path: T) -> RvResult<Vec<PathBuf>>
    {
        let mut paths: Vec<PathBuf> = vec![];
        if !self.is_dir(&path) {
            return Err(PathError::is_not_dir(&path).into());
        }
        for entry in self.entries(path)?.min_depth(1).max_depth(1).sort_by_name() {
            let entry = entry?;
            paths.push(entry.path_buf());
        }
        Ok(paths)
    }

    /// Read all data from the given file and return it as a String
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Errors
    /// * PathError::IsNotFile(PathBuf) when the given path isn't a file
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let file = memfs.root().mash("file");
    /// memfs.write_all(&file, b"foobar 1").unwrap();
    /// assert_eq!(memfs.read_all(&file).unwrap(), "foobar 1".to_string());
    /// ```
    fn read_all<T: AsRef<Path>>(&self, path: T) -> RvResult<String>
    {
        match self.open(path) {
            Ok(mut file) => {
                let mut buf = String::new();
                file.read_to_string(&mut buf)?;
                Ok(buf)
            },
            Err(e) => Err(e),
        }
    }

    /// Returns the relative path of the target the link points to
    ///
    /// * Handles path expansion and absolute path resolution
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let file = memfs.root().mash("file");
    /// let link = memfs.root().mash("link");
    /// assert_eq!(&memfs.mkfile(&file).unwrap(), &file);
    /// assert_eq!(&memfs.symlink(&link, &file).unwrap(), &link);
    /// assert_eq!(memfs.readlink(&link).unwrap(), PathBuf::from("file"));
    /// ```
    fn readlink<T: AsRef<Path>>(&self, link: T) -> RvResult<PathBuf>
    {
        let path = self.abs(link)?;
        let guard = self.0.read().unwrap();

        // Validate the link path
        if let Some(entry) = guard.fs.get(&path) {
            if !entry.is_symlink() {
                return Err(PathError::is_not_symlink(path).into());
            }
            return Ok(entry.rel().to_owned());
        } else {
            return Err(PathError::does_not_exist(path).into());
        }
    }

    /// Removes the given empty directory or file
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides link exclusion i.e. removes the link themselves not what its points to
    ///
    /// ### Errors
    /// * a directory containing files will trigger an error. use `remove_all` instead
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let file = memfs.root().mash("file");
    /// assert!(memfs.mkfile(&file).is_ok());
    /// assert_eq!(memfs.exists(&file), true);
    /// assert!(memfs.remove(&file).is_ok());
    /// assert_eq!(memfs.exists(&file), false);
    /// ```
    fn remove<T: AsRef<Path>>(&self, path: T) -> RvResult<()>
    {
        let path = self.abs(path)?;
        let mut guard = self.0.write().unwrap();

        // First check if the target contains files
        if let Some(entry) = guard.fs.get(&path) {
            if let Some(ref files) = entry.files {
                if files.len() > 0 {
                    return Err(PathError::dir_contains_files(path).into());
                }
            }
        }

        // Next remove the file from its parent
        let dir = path.dir()?;
        if let Some(entry) = guard.fs.get_mut(&dir) {
            entry.remove(path.base()?)?;
        }

        // Next remove its data file if it exists
        if let Some(entry) = guard.fs.get(&path) {
            if entry.is_file() {
                guard.data.remove(&path);
            }
        }

        // Finally remove the entry from the filesystem
        guard.fs.remove(&path);
        Ok(())
    }

    /// Removes the given directory after removing all of its contents
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Provides link exclusion i.e. removes the link themselves not what its points to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let dir = memfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert!(memfs.mkdir_p(&dir).is_ok());
    /// assert!(memfs.mkfile(&file).is_ok());
    /// assert_eq!(memfs.is_file(&file), true);
    /// assert!(memfs.remove_all(&dir).is_ok());
    /// assert_eq!(memfs.exists(&dir), false);
    /// assert_eq!(memfs.exists(&file), false);
    /// ```
    fn remove_all<T: AsRef<Path>>(&self, path: T) -> RvResult<()>
    {
        if self.exists(&path) {
            for entry in self.entries(path)?.contents_first().into_iter() {
                self.remove(entry?.path())?;
            }
        }
        Ok(())
    }

    /// Returns the current root directory
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let mut root = PathBuf::new();
    /// root.push(Component::RootDir);
    /// assert_eq!(memfs.root(), root);
    /// ```
    fn root(&self) -> PathBuf
    {
        let guard = self.0.read().unwrap();
        guard.root.clone()
    }

    /// Set the current working directory
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Relative path will use the current working directory
    ///
    /// ### Errors
    /// * PathError::DoesNotExist(PathBuf) when the given path doesn't exist
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let dir = memfs.root().mash("dir");
    /// assert_eq!(memfs.cwd().unwrap(), memfs.root());
    /// assert_eq!(&memfs.mkdir_p(&dir).unwrap(), &dir);
    /// assert_eq!(&memfs.set_cwd(&dir).unwrap(), &dir);
    /// assert_eq!(&memfs.cwd().unwrap(), &dir);
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

    /// Creates a new symbolic link
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Computes the target path `src` relative to the `dst` link name's absolute path
    /// * Returns the link path
    ///
    /// ### Arguments
    /// * `link` - the path of the link being created
    /// * `target` - the path that the link will point to
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let memfs = Memfs::new();
    /// let file = memfs.root().mash("file");
    /// let link = memfs.root().mash("link");
    /// assert_eq!(&memfs.mkfile(&file).unwrap(), &file);
    /// assert_eq!(&memfs.symlink(&link, &file).unwrap(), &link);
    /// assert_eq!(memfs.readlink(&link).unwrap(), PathBuf::from("file"));
    /// ```
    fn symlink<T: AsRef<Path>, U: AsRef<Path>>(&self, link: T, target: U) -> RvResult<PathBuf>
    {
        let link = self.abs(link)?;
        let target = target.as_ref().to_owned();

        // Convert relative links to absolute to ensure they are clean
        let target = self.abs(if !target.is_absolute() { link.dir()?.mash(target) } else { target })?;

        // Create the new entry as a link and set its target as a file by default
        let mut entry_opts = MemfsEntry::opts(&link).file().link_to(&target)?;

        // If the target exists and is a directory switch the type
        {
            let guard = self.0.read().unwrap();
            if let Some(ref x) = guard.fs.get(&target) {
                if x.is_dir() {
                    entry_opts = entry_opts.dir().link_to(&target)?;
                }
            }
        }

        self.add(entry_opts.new())?;

        Ok(link)
    }

    /// Write the given data to to the target file
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Create the file first if it doesn't exist or truncating it first if it does
    ///
    /// ### Errors
    /// * PathError::IsNotDir(PathBuf) when the given path's parent exists but is not a directory
    /// * PathError::DoesNotExist(PathBuf) when the given path's parent doesn't exist
    /// * PathError::IsNotFile(PathBuf) when the given path exists but is not a file
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_no_file!(vfs, &file);
    /// assert_vfs_write_all!(vfs, &file, b"foobar 1");
    /// assert_vfs_is_file!(vfs, &file);
    /// assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());
    /// ```
    fn write_all<T: AsRef<Path>, U: AsRef<[u8]>>(&self, path: T, data: U) -> RvResult<()>
    {
        let mut f = self.create(path)?;
        f.write_all(data.as_ref())?;
        Ok(())
    }

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new().upcast();
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
        memfs.mkdir_p("foo").unwrap();
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

        // absolute path doesn't exist
        assert_eq!(memfs.abs("").unwrap_err().to_string(), PathError::Empty.to_string());
    }

    #[test]
    fn test_memfs_append()
    {
        let vfs = Memfs::new();
        let file = vfs.root().mash("file");

        // abs fails
        if let Err(e) = vfs.append("") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        // Append to a new file and check the data wrote to it
        let mut f = vfs.append(&file).unwrap();
        f.write_all(b"foobar").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "foobar".to_string());
        f.write_all(b"123").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "foobar123".to_string());

        // Append to the file in another trasaction
        let mut f = vfs.append(&file).unwrap();
        f.write_all(b" this is a test").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "foobar123 this is a test".to_string());
    }

    #[test]
    fn test_memfs_chmod()
    {
        let vfs = Vfs::memfs();
        let file = vfs.root().mash("file");

        // abs fails
        if let Err(e) = vfs.chmod("", 0) {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        assert_vfs_mkfile!(vfs, &file);
        assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
        assert!(vfs.chmod(&file, 0o555).is_ok());
        assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    }

    #[test]
    fn test_memfs_chmod_b()
    {
        let vfs = Vfs::memfs();
        let dir = vfs.root().mash("dir");
        let file = dir.mash("file");

        // abs fails
        if let Err(e) = vfs.chmod_b("") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        assert_vfs_mkdir_p!(vfs, &dir);
        assert_vfs_mkfile!(vfs, &file);
        assert_eq!(vfs.mode(&dir).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
        assert!(vfs.chmod_b(&dir).unwrap().recurse().all(0o777).exec().is_ok());
        assert_eq!(vfs.mode(&dir).unwrap(), 0o40777);
        assert_eq!(vfs.mode(&file).unwrap(), 0o100777);
    }

    #[test]
    fn test_memfs_cwd()
    {
        let memfs = Memfs::new();
        assert_eq!(memfs.cwd().unwrap(), memfs.root());
        memfs.mkdir_p("foo").unwrap();
        memfs.set_cwd("foo").unwrap();
        assert_eq!(memfs.cwd().unwrap(), memfs.root().mash("foo"));
    }

    #[test]
    fn test_memfs_create()
    {
        let vfs = Vfs::memfs();
        let file = vfs.root().mash("file");

        // abs fails
        if let Err(e) = vfs.create("") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        // Create a new file and check the data wrote to it
        let mut f = vfs.create(&file).unwrap();
        f.write_all(b"foobar").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "foobar".to_string());
        f.write_all(b"123").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "foobar123".to_string());

        // Overwrite the file
        let mut f = vfs.create(&file).unwrap();
        f.write_all(b"this is a test").unwrap();
        f.flush().unwrap();
        assert_vfs_read_all!(vfs, &file, "this is a test".to_string());
    }

    #[test]
    fn test_memfs_dirs()
    {
        let vfs = Memfs::new();
        let tmpdir = vfs.root().mash("tmpdir");
        let dir1 = tmpdir.mash("dir1");
        let dir2 = tmpdir.mash("dir2");
        let file1 = tmpdir.mash("file1");

        // abs error
        assert_eq!(vfs.dirs("").unwrap_err().to_string(), PathError::is_not_dir("").to_string());

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_vfs_mkfile!(vfs, &file1);
        assert_iter_eq(vfs.dirs(&tmpdir).unwrap(), vec![dir1, dir2]);
    }

    #[test]
    fn test_memfs_entries()
    {
        let memfs = Memfs::new();
        let dir1 = memfs.root().mash("dir1");
        let dir2 = dir1.mash("dir2");
        let file = dir2.mash("file");
        assert_eq!(&memfs.mkdir_p(&dir2).unwrap(), &dir2);
        assert_eq!(&memfs.mkfile(&file).unwrap(), &file);

        // abs error
        assert_eq!(memfs.entries("").unwrap_err().to_string(), PathError::Empty.to_string());

        let mut iter = memfs.entries(memfs.root()).unwrap().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), memfs.root());
        assert_eq!(iter.next().unwrap().unwrap().path(), &dir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), &dir2);
        assert_eq!(iter.next().unwrap().unwrap().path(), &file);
        assert_eq!(iter.next().is_none(), true);
    }

    #[test]
    fn test_memfs_entry()
    {
        let vfs = Memfs::new();
        let file = vfs.root().mash("file");

        // abs error
        assert_eq!(vfs.entry("").unwrap_err().to_string(), PathError::Empty.to_string());

        assert_vfs_mkfile!(vfs, &file);
        assert!(vfs.entry(&file).unwrap().is_file());
    }

    #[test]
    fn test_memfs_exists()
    {
        let memfs = Memfs::new();
        let dir1 = memfs.root().mash("dir1");

        // abs fails
        assert_eq!(memfs.exists(""), false);

        // Doesn't exist
        assert_eq!(memfs.exists(&dir1), false);

        // Exists
        assert_eq!(&memfs.mkdir_p(&dir1).unwrap(), &dir1);
        assert_eq!(memfs.exists(&dir1), true);
    }

    #[test]
    fn test_memfs_files()
    {
        let vfs = Memfs::new();
        let tmpdir = vfs.root().mash("tmpdir");
        let dir1 = tmpdir.mash("dir1");
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");

        // abs error
        assert_eq!(vfs.files("").unwrap_err().to_string(), PathError::is_not_dir("").to_string());

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_iter_eq(vfs.files(&tmpdir).unwrap(), vec![file1, file2]);
    }

    #[test]
    fn test_memfs_is_exec()
    {
        let vfs = Vfs::memfs();
        let file = vfs.root().mash("file");

        // abs fails
        assert_eq!(vfs.is_exec(""), false);

        assert!(vfs.mkfile_m(&file, 0o644).is_ok());
        assert_eq!(vfs.is_exec(&file), false);
        assert!(vfs.chmod(&file, 0o777).is_ok());
        assert_eq!(vfs.is_exec(&file), true);
    }

    #[test]
    fn test_memfs_is_dir()
    {
        let memfs = Memfs::new();
        let dir1 = memfs.root().mash("dir1");

        // abs fails
        assert_eq!(memfs.is_dir(""), false);

        // Doesn't exist
        assert_eq!(memfs.is_dir(&dir1), false);

        // Exists
        assert_eq!(&memfs.mkdir_p(&dir1).unwrap(), &dir1);
        assert_eq!(memfs.is_dir(&dir1), true);
    }

    #[test]
    fn test_memfs_is_file()
    {
        let memfs = Memfs::new();
        let file = memfs.root().mash("file");

        // abs fails
        assert_eq!(memfs.is_file(""), false);

        // Doesn't exist
        assert_eq!(memfs.is_file(&file), false);

        // Exists
        assert_eq!(&memfs.mkfile(&file).unwrap(), &file);
        assert_eq!(memfs.is_file(&file), true);
    }

    #[test]
    fn test_memfs_is_readonly()
    {
        let vfs = Vfs::memfs();
        let file = vfs.root().mash("file");

        // abs fails
        assert_eq!(vfs.is_readonly(""), false);

        assert!(vfs.mkfile_m(&file, 0o644).is_ok());
        assert_eq!(vfs.is_readonly(&file), false);
        assert!(vfs.chmod_b(&file).unwrap().readonly().exec().is_ok());
        assert_eq!(vfs.mode(&file).unwrap(), 0o100444);
        assert_eq!(vfs.is_readonly(&file), true);
    }

    #[test]
    fn test_memfs_is_symlink()
    {
        let memfs = Memfs::new();
        let file = memfs.root().mash("file");
        let link = memfs.root().mash("link");

        // abs fails
        assert_eq!(memfs.is_symlink(""), false);

        // Doesn't exist
        assert_eq!(memfs.is_symlink(&file), false);

        // Exists
        assert_eq!(&memfs.symlink(&link, &file).unwrap(), &link);
        assert_eq!(memfs.is_symlink(&link), true);
    }

    #[test]
    fn test_memfs_mkdir_m()
    {
        let vfs = Vfs::memfs();
        let dir = vfs.root().mash("dir");

        // abs error
        assert_eq!(vfs.mkdir_m("", 0).unwrap_err().to_string(), PathError::Empty.to_string());

        assert!(vfs.mkdir_m(&dir, 0o555).is_ok());
        assert_eq!(vfs.mode(&dir).unwrap(), 0o40555);
    }

    #[test]
    fn test_memfs_mkdir_p()
    {
        let memfs = Memfs::new();
        let dir = memfs.root().mash("dir");

        // Check single top level
        assert_eq!(memfs.exists(&dir), false);
        assert_eq!(&memfs.mkdir_p(&dir).unwrap(), &dir);
        assert_eq!(memfs.exists(&dir), true);
        assert_eq!(memfs.exists("dir"), true); // check relative

        // Check nested
        let dir1 = memfs.root().mash("dir1");
        let dir2 = dir1.mash("dir2");
        let dir3 = dir2.mash("dir3");
        assert_eq!(&memfs.mkdir_p(&dir3).unwrap(), &dir3);
        assert_eq!(memfs.exists(&dir3), true);
        assert_eq!(memfs.exists(&dir2), true);
        assert_eq!(memfs.exists(&dir1), true);
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

    #[test]
    fn test_memfs_mkfile()
    {
        let memfs = Memfs::new();
        let dir1 = memfs.root().mash("dir1");
        let file1 = dir1.mash("file1");

        // abs error
        assert_eq!(memfs.mkfile("").unwrap_err().to_string(), PathError::Empty.to_string());

        // parent directory doesn't exist
        assert_eq!(memfs.mkfile(&file1).unwrap_err().to_string(), PathError::does_not_exist(&dir1).to_string());

        // Error: target exists and is not a file
        assert_eq!(&memfs.mkdir_p(&dir1).unwrap(), &dir1);
        assert_eq!(memfs.mkfile(&dir1).unwrap_err().to_string(), PathError::is_not_file(&dir1).to_string());

        // Make a file in the root
        assert_eq!(memfs.exists("file2"), false);
        assert_eq!(memfs.mkfile("file2").unwrap(), memfs.root().mash("file2"));
        assert_eq!(memfs.exists("file2"), true);

        // Make a file in a directory
        assert_eq!(memfs.exists(&file1), false);
        assert_eq!(&memfs.mkfile(&file1).unwrap(), &file1);
        assert_eq!(memfs.exists(&file1), true);

        // Error: parent exists and is not a directory
        let file2 = file1.mash("file2");
        assert_eq!(memfs.mkfile(&file2).unwrap_err().to_string(), PathError::is_not_dir(&file1).to_string());
    }

    #[test]
    fn test_memfs_mkfile_m()
    {
        let vfs = Vfs::memfs();
        let file = vfs.root().mash("file");

        // abs error
        assert_eq!(vfs.mkfile_m("", 0).unwrap_err().to_string(), PathError::Empty.to_string());

        assert!(vfs.mkfile_m(&file, 0o555).is_ok());
        assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    }

    #[test]
    fn test_memfs_mode()
    {
        let vfs = Vfs::memfs();
        let file = vfs.root().mash("file");

        // abs error
        assert_eq!(vfs.mode("").unwrap_err().to_string(), PathError::Empty.to_string());

        assert_vfs_mkfile!(vfs, &file);
        assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
        assert!(vfs.chmod(&file, 0o555).is_ok());
        assert_eq!(vfs.mode(&file).unwrap(), 0o100555);
    }

    #[test]
    fn test_memfs_open()
    {
        let vfs = Vfs::memfs();
        let file = vfs.root().mash("file");

        // abs fails
        if let Err(e) = vfs.open("") {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }

        assert_vfs_write_all!(vfs, &file, b"foobar 1");
        let mut file = vfs.open(&file).unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "foobar 1".to_string());
    }

    #[test]
    fn test_memfs_paths()
    {
        let vfs = Memfs::new();
        let tmpdir = vfs.root().mash("tmpdir");
        let dir1 = tmpdir.mash("dir1");
        let dir2 = tmpdir.mash("dir2");
        let file1 = tmpdir.mash("file1");

        // abs error
        assert_eq!(vfs.paths("").unwrap_err().to_string(), PathError::is_not_dir("").to_string());

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_vfs_mkfile!(vfs, &file1);
        assert_iter_eq(vfs.paths(&tmpdir).unwrap(), vec![dir1, dir2, file1]);
    }

    #[test]
    fn test_memfs_read_all()
    {
        let memfs = Memfs::new();
        let file = memfs.root().mash("file");

        // Doesn't exist error
        assert_eq!(memfs.read_all(&file).unwrap_err().to_string(), PathError::does_not_exist(&file).to_string());

        // Isn't a file
        let dir = memfs.root().mash("dir");
        assert_eq!(&memfs.mkdir_p(&dir).unwrap(), &dir);
        assert_eq!(memfs.read_all(&dir).unwrap_err().to_string(), PathError::is_not_file(&dir).to_string());

        // Create the file with the given data
        memfs.write_all(&file, b"foobar 1").unwrap();
        assert_eq!(memfs.read_all(&file).unwrap(), "foobar 1".to_string());

        // Read a second time
        assert_eq!(memfs.read_all(&file).unwrap(), "foobar 1".to_string());
    }

    #[test]
    fn test_memfs_readlink()
    {
        let vfs = Vfs::memfs();
        let file = vfs.root().mash("file");
        let link = vfs.root().mash("link");

        // Doesn't exist error
        assert_eq!(vfs.readlink("").unwrap_err().to_string(), PathError::Empty.to_string());

        assert_vfs_mkfile!(vfs, &file);
        assert_vfs_symlink!(vfs, &link, &file);
        assert_vfs_readlink!(vfs, &link, &file);
    }

    #[test]
    fn test_memfs_remove()
    {
        let vfs = Vfs::memfs();
        let dir1 = vfs.root().mash("dir1");
        let file1 = dir1.mash("file1");
        let file2 = vfs.root().mash("file2");

        // abs error
        assert_eq!(vfs.remove("").unwrap_err().to_string(), PathError::Empty.to_string());

        // Single file
        assert_vfs_mkfile!(vfs, &file2);
        assert_vfs_is_file!(vfs, &file2);
        assert_vfs_remove!(vfs, &file2);
        assert_vfs_no_file!(vfs, &file2);

        // Directory with files
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_eq!(vfs.remove(&dir1).unwrap_err().to_string(), PathError::dir_contains_files(&dir1).to_string());
        assert_vfs_remove!(vfs, &file1);
        assert_vfs_remove!(vfs, &dir1);
        assert_vfs_no_exists!(vfs, &dir1);
    }

    #[test]
    fn test_memfs_remove_all()
    {
        let vfs = Vfs::memfs();
        let dir = vfs.root().mash("dir");
        let file = dir.mash("file");

        assert_vfs_mkdir_p!(vfs, &dir);
        assert_vfs_mkfile!(vfs, &file);
        assert_vfs_is_file!(vfs, &file);
        assert_vfs_remove_all!(vfs, &dir);
        assert_vfs_no_exists!(vfs, &file);
        assert_vfs_no_exists!(vfs, &dir);
    }

    #[test]
    fn test_memfs_symlink()
    {
        let vfs = Memfs::new().upcast();
        let dir1 = vfs.root().mash("dir1");
        let file1 = dir1.mash("file1");
        let link1 = vfs.root().mash("link1");
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_symlink!(vfs, &link1, &dir1);

        // Validate the link was created correctly
        if let Vfs::Memfs(ref memfs) = vfs {
            let guard = memfs.0.read().unwrap();

            // Ensure that no file was created for the link
            assert_eq!(guard.data.contains_key(&link1), false);

            // Ensure that the entry has the right properties
            if let Some(entry) = guard.fs.get(&link1) {
                // Check the correct path is set for the link
                assert_eq!(entry.path(), &link1);

                // Check that the target is absolute
                assert_eq!(entry.alt(), &dir1);

                // Check that the target's relative path is accurate
                assert_eq!(entry.rel(), Path::new("dir1"));
            }
        }
    }

    #[test]
    fn test_write_all()
    {
        let vfs = Vfs::memfs();
        let dir = vfs.root().mash("dir");
        let file = dir.mash("file");

        // fail abs
        assert_eq!(vfs.write_all("", "").unwrap_err().to_string(), PathError::Empty.to_string());

        // parent doesn't exist
        assert_eq!(vfs.write_all(&file, "").unwrap_err().to_string(), PathError::does_not_exist(&dir).to_string());

        // exists but not a file
        assert_vfs_mkdir_p!(vfs, &dir);
        assert_eq!(vfs.write_all(&dir, "").unwrap_err().to_string(), PathError::is_not_file(&dir).to_string());

        // happy path
        assert!(vfs.write_all(&file, b"foobar 1").is_ok());
        assert_vfs_is_file!(vfs, &file);
        assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());
    }
}

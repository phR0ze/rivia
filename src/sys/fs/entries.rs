use std::{cmp::Ordering, fmt, path::Path};

use super::entry_iter::EntryIter;
use crate::{
    errors::*,
    sys::{Entry, VfsEntry},
    trying,
};

pub(crate) const DEFAULT_MAX_DESCRIPTORS: u16 = 50;

/// Provides a builder pattern for constructing iterators for travsersing a virtual file system
///
/// * Support for Rivia VFS
/// * Recursive directory traversal with depth control
/// * Symbolic link following
/// * Automatic link path reading
/// * Directory entries `.` and `..` are ommitted
/// * Use the builder functions on [`Vfs`], `Stdfs` and `Memfs` to create an `Entries` instance
///
/// ### Inspired by WalkDir
/// Entries provides a similar feature set in a simplified manner that is Virtual File System (VFS)
/// friendly.
///
/// ## Traversal
/// Entries is a depth first algorithm by default with directories yielded before their contents.
/// However this behavior can be changed by setting the `contents_first` options to direct Entries
/// to yield the contents of directories first before the directory its self which is useful for
/// operations like chmod that revoke permissions to read.
///
/// ## File Descriptors
/// Considering that most unix type systems have a limit of 1024 file descriptors, Paths is careful
/// not to exhaust this resource by limiting its internal consumption to no more than 50 at a time.
/// Anything beyond that will be read into memory and iterated from there internally rather than
/// holding more than 50 open file descriptors.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_mkdir_p!(vfs, "dir1");
/// assert_vfs_mkfile!(vfs, "file1");
/// assert_vfs_mkdir_p!(vfs, "dir2");
/// assert_vfs_mkfile!(vfs, "dir2/file2");
/// let mut iter = vfs.entries("/").unwrap().dirs_first().sort_by_name().into_iter();
/// assert_eq!(iter.next().unwrap().unwrap().path(), Path::new("/"));
/// assert_eq!(iter.next().unwrap().unwrap().path(), Path::new("/dir1"));
/// assert_eq!(iter.next().unwrap().unwrap().path(), Path::new("/dir2"));
/// assert_eq!(iter.next().unwrap().unwrap().path(), Path::new("/dir2/file2"));
/// assert_eq!(iter.next().unwrap().unwrap().path(), Path::new("/file1"));
/// assert!(iter.next().is_none());
/// ```
pub struct Entries
{
    pub(crate) root: VfsEntry,
    pub(crate) dirs: bool,
    pub(crate) files: bool,
    pub(crate) follow: bool,
    pub(crate) min_depth: usize,
    pub(crate) max_depth: usize,
    pub(crate) max_descriptors: u16,
    pub(crate) dirs_first: bool,
    pub(crate) files_first: bool,
    pub(crate) sort_by_name: bool,
    pub(crate) contents_first: bool,
    pub(crate) pre_op: Option<Box<dyn FnMut(&VfsEntry) -> RvResult<()>+Send+Sync+'static>>,
    pub(crate) sort: Option<Box<dyn Fn(&VfsEntry, &VfsEntry) -> Ordering+Send+Sync+'static>>,
    pub(crate) iter_from: Box<dyn Fn(&Path, bool) -> RvResult<EntryIter>+Send+Sync+'static>,
}

impl Entries
{
    /// Filter entries down to just directories
    ///
    /// * Default is `false`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_vfs_mkdir_p!(vfs, "zdir");
    /// assert_vfs_mkfile!(vfs, "file");
    /// let mut iter = vfs.entries(vfs.root()).unwrap().dirs().into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), vfs.root());
    /// assert_eq!(iter.next().unwrap().unwrap().path(), vfs.root().mash("zdir"));
    /// assert!(iter.next().is_none());
    /// ```
    pub fn dirs(mut self) -> Self
    {
        self.dirs = true;
        self.files = false;
        self
    }

    /// Filter entries down to just files
    ///
    /// * Default is `false`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// assert_vfs_mkdir_p!(vfs, "dir");
    /// assert_vfs_mkfile!(vfs, "file");
    /// let mut iter = vfs.entries(vfs.root()).unwrap().files().into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), vfs.root().mash("file"));
    /// assert!(iter.next().is_none());
    /// ```
    pub fn files(mut self) -> Self
    {
        self.dirs = false;
        self.files = true;
        self
    }

    /// Follow links that point to directories
    ///
    /// * Default is `false`
    /// * Will iterate over the contents of directories pointed to when `true`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &dir);
    /// let mut iter = vfs.entries(&link).unwrap().follow(true).into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &dir);
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &file);
    /// assert!(iter.next().is_none());
    /// ```
    pub fn follow(mut self, yes: bool) -> Self
    {
        self.follow = yes;
        self
    }

    /// Set the min depth that Entries should traverse
    ///
    /// * Default is `0`
    /// * The given path is considered depth 0
    /// * To only include the given path and not recurse set `max_depth(0)`
    /// * By default recursion is unbounded. use `max_depth(VALUE)` to bound it
    /// * Setting `min_depth` first will autocorrect later calls to `max_depth` to be consistent
    /// in relation to `min_depth`. The inverse would be true if `max_depth` was called first.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let mut iter = vfs.entries(vfs.root()).unwrap().min_depth(1).into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &file);
    /// assert!(iter.next().is_none());
    /// ```
    pub fn min_depth(mut self, min: usize) -> Self
    {
        self.min_depth = min;
        if self.min_depth > self.max_depth {
            self.min_depth = self.max_depth;
        }
        self
    }

    /// Set the max depth that Entries should traverse exclusive
    ///
    /// * Default is `std::usize::MAX`
    /// * The given path is considered depth 0
    /// * To only include the given path and not recurse set `max_depth(0)`
    /// * By default recursion is unbounded. use `max_depth(VALUE)` to bound it
    /// * Setting `min_depth` first will autocorrect later calls to `max_depth` to be consistent
    /// in relation to `min_depth`. The inverse would be true if `max_depth` was called first.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// let mut iter = vfs.entries(vfs.root()).unwrap().min_depth(1).max_depth(1).into_iter();
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &dir);
    /// assert!(iter.next().is_none());
    /// ```
    pub fn max_depth(mut self, max: usize) -> Self
    {
        self.max_depth = max;
        if self.max_depth < self.min_depth {
            self.max_depth = self.min_depth;
        }
        self
    }

    /// Set the pre-operation function to run over each directory before processing
    ///
    /// * Defaults to `None`
    /// * Runs the pre-operation before reading the filesystem
    /// * Useful for changing permissions or ownership on the way in to allow for recursion
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn pre_op(mut self, op: impl FnMut(&VfsEntry) -> RvResult<()>+Send+Sync+'static) -> Self
    {
        self.pre_op = Some(Box::new(op));
        self
    }

    /// Set the default sorter to be directories first by name
    ///
    /// * Defaults to `false`
    /// * Caches all entries and iterates from memory
    /// * Sorts directories first and then by name
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn dirs_first(mut self) -> Self
    {
        self.dirs_first = true;
        self.sort(|x, y| x.file_name().cmp(&y.file_name()))
    }

    /// Set the default sorter to be files first by name
    ///
    /// * Defaults to `false`
    /// * Caches all entries and iterates from memory
    /// * Sorts directories first and then by name
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn files_first(mut self) -> Self
    {
        self.files_first = true;
        self.sort(|x, y| x.file_name().cmp(&y.file_name()))
    }

    /// Return the contents of directories before the directory itself
    ///
    /// * Defaults to `false`
    /// * A recursive operation useful for things like revoking permission on the way out
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn contents_first(mut self) -> Self
    {
        self.contents_first = true;
        self
    }

    /// Set the default sorter to be by name
    ///
    /// * Defaults to `false`
    /// * Caches all entries and iterates from memory to enforce ordering
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn sort_by_name(mut self) -> Self
    {
        self.sort_by_name = true;
        self.sort(|x, y| x.file_name().cmp(&y.file_name()))
    }

    /// Set a function for sorting entries.
    ///
    /// * Defaults to `None`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn sort(mut self, cmp: impl Fn(&VfsEntry, &VfsEntry) -> Ordering+Send+Sync+'static) -> Self
    {
        self.sort = Some(Box::new(cmp));
        self
    }
}

impl fmt::Debug for Entries
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::result::Result<(), fmt::Error>
    {
        f.debug_struct("Entries")
            .field("root", &self.root)
            .field("dirs", &self.dirs)
            .field("files", &self.files)
            .field("follow", &self.follow)
            .field("min_depth", &self.min_depth)
            .field("max_depth", &self.max_depth)
            .field("max_descriptors", &self.max_descriptors)
            .field("dirs_first", &self.dirs_first)
            .field("files_first", &self.files_first)
            .field("contents_first", &self.contents_first)
            .field("sort_by_name", &self.sort_by_name)
            .finish()
    }
}

/// Convert Entries in an iterator over EntriesIter
impl IntoIterator for Entries
{
    type IntoIter = EntriesIter;
    type Item = RvResult<VfsEntry>;

    fn into_iter(self) -> EntriesIter
    {
        let mut iter = EntriesIter {
            opts: self,
            started: false,
            open_descriptors: 0,
            filter: None,
            deferred: vec![],
            iters: vec![],
        };

        // Create any configured filters
        if iter.opts.files {
            iter.filter = Some(Box::new(|x: &VfsEntry| -> bool { x.is_file() }));
        } else if iter.opts.dirs {
            iter.filter = Some(Box::new(|x: &VfsEntry| -> bool { x.is_dir() }));
        }

        iter
    }
}

/// Actual underlying iterator for traversing a virtual file system
///
/// Use the VFS builder functions to construct an instance e.g. vfs.entries or Stdfs::entries.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_mkdir_p!(vfs, "dir1");
/// assert_vfs_mkfile!(vfs, "file1");
/// assert_vfs_mkdir_p!(vfs, "dir2");
/// assert_vfs_mkfile!(vfs, "dir2/file2");
/// let mut iter = vfs.entries("/").unwrap().dirs_first().sort_by_name().into_iter();
/// assert_eq!(iter.next().unwrap().unwrap().path(), Path::new("/"));
/// assert_eq!(iter.next().unwrap().unwrap().path(), Path::new("/dir1"));
/// assert_eq!(iter.next().unwrap().unwrap().path(), Path::new("/dir2"));
/// assert_eq!(iter.next().unwrap().unwrap().path(), Path::new("/dir2/file2"));
/// assert_eq!(iter.next().unwrap().unwrap().path(), Path::new("/file1"));
/// assert!(iter.next().is_none());
/// ```
pub struct EntriesIter
{
    // Builder pattern options
    opts: Entries,

    // Flag to track root being processed which might not be a directory
    started: bool,

    // Number of open file descriptors
    open_descriptors: u16,

    // Stack of entry iterators for current directories being iterated over
    iters: Vec<EntryIter>,

    // Stack of deferred directories to return after their contents
    deferred: Vec<VfsEntry>,

    // Optional filter that yields only entries that match the predicate
    filter: Option<Box<dyn FnMut(&VfsEntry) -> bool>>,
}

impl EntriesIter
{
    /// Enqueue the entry if it is a directory or a directory link and follow is true.
    /// None will be returned if the given entry was filtered out.
    fn process(&mut self, entry: VfsEntry) -> Option<RvResult<VfsEntry>>
    {
        let depth = self.iters.len(); // save depth before possible recursion

        if entry.is_dir() && (!entry.is_symlink() || self.opts.follow) {
            // Throw an error if link looping is detected
            if entry.is_symlink() && self.iters.iter().any(|x| x.path() == entry.path()) {
                return Some(Err(PathError::link_looping(entry.path()).into()));
            }

            // Only add if max depth marker is satisfied
            if self.iters.len() < self.opts.max_depth {
                // Execute pre-op function if exists before traversal is started
                if let Some(pre_op) = &mut self.opts.pre_op {
                    trying!((pre_op)(&entry));
                }
                self.iters.push(trying!((self.opts.iter_from)(entry.path(), self.opts.follow)));

                // Cache entries if we've hit our open file descriptors max or if were sorting the
                // entries.
                if self.opts.sort.is_some() || (self.open_descriptors + 1 > self.opts.max_descriptors) {
                    if let Some(sort) = &self.opts.sort {
                        if self.opts.dirs_first {
                            self.iters.last_mut().unwrap().dirs_first(sort);
                        } else if self.opts.files_first {
                            self.iters.last_mut().unwrap().files_first(sort);
                        } else {
                            self.iters.last_mut().unwrap().sort(sort);
                        }
                    } else {
                        self.iters.last_mut().unwrap().cache();
                    }
                } else {
                    self.open_descriptors += 1;
                }
            }
        }

        // Return None if min depth marker is not satisfied
        if depth < self.opts.min_depth {
            return None;
        }

        // Defer directories as directed
        if entry.is_dir() && self.opts.contents_first {
            self.deferred.push(entry);
            return None;
        }

        // Filter as directed
        if let Some(filter) = &mut self.filter {
            if !(filter)(&entry) {
                return None;
            }
        }

        Some(Ok(entry))
    }

    /// Filter on entries such that only entries that match the given predicate are returned
    /// by calls to next(). This is convenient as you don't have to deal with a result type
    /// using this function.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Vfs::memfs();
    /// let dir1 = vfs.root().mash("dir1");
    /// let file1 = vfs.root().mash("file1");
    /// let dir2 = vfs.root().mash("dir2");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert_vfs_mkdir_p!(vfs, &dir2);
    /// let mut iter = vfs.entries(vfs.root()).unwrap().sort_by_name().into_iter().filter_p(|x| x.path().has_suffix("1"));
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &dir1);
    /// assert_eq!(iter.next().unwrap().unwrap().path(), &file1);
    /// assert!(iter.next().is_none());
    /// ```
    pub fn filter_p(mut self, predicate: impl FnMut(&VfsEntry) -> bool+'static) -> Self
    {
        self.filter = Some(Box::new(predicate));
        self
    }
}

impl Iterator for EntriesIter
{
    type Item = RvResult<VfsEntry>;

    fn next(&mut self) -> Option<RvResult<VfsEntry>>
    {
        if !self.started {
            self.started = true;

            // Create the root entry allowing for following links
            let result = self.process(self.opts.root.clone().follow(self.opts.follow));

            // Allow for the possibility that the root has been filtered out
            if result.is_some() {
                return result;
            }
        }

        // Loop here to ensure that we get the next entry when filtering or deferring
        while !self.iters.is_empty() {
            // Return deferred directories if we've already processed their children
            if self.opts.contents_first && self.iters.len() < self.deferred.len() {
                if let Some(entry) = self.deferred.pop() {
                    return Some(Ok(entry));
                }
            }

            // Process the next entry from the current iterator
            match self.iters.last_mut().unwrap().next() {
                Some(Ok(entry)) => match self.process(entry) {
                    Some(result) => return Some(result),
                    None => continue, // None indicates filtered out so get another
                },
                Some(Err(err)) => return Some(Err(err)),
                None => {
                    // Decrement open file descriptors appropriately
                    if let Some(iter) = self.iters.pop() {
                        if !iter.cached() {
                            self.open_descriptors -= 1;
                        }
                    }
                },
            };
        }

        // Return root directory for deferred case
        if self.opts.contents_first && self.iters.len() < self.deferred.len() {
            if let Some(entry) = self.deferred.pop() {
                return Some(Ok(entry));
            }
        }

        None
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    fn assert_iter_eq(iter: EntriesIter, paths: Vec<&PathBuf>)
    {
        // Using a vector here as there can be duplicates
        let mut entries = Vec::new();
        for entry in iter {
            entries.push(entry.unwrap().path().to_path_buf());
        }

        assert_eq!(entries.len(), paths.len());
        for path in paths.iter() {
            assert!(entries.contains(path));
        }
    }

    #[test]
    fn test_entries_debug()
    {
        let vfs = Vfs::memfs();
        let entries = vfs.entries(vfs.root()).unwrap();
        assert_eq!(format!("{:?}", &entries), format!("{:?}", &entries));
    }

    #[test]
    fn test_vfs_dirs()
    {
        test_dirs(assert_vfs_setup!(Vfs::memfs()));
        test_dirs(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_dirs((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("zdir");
        let file1 = tmpdir.mash("file");
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);

        // Without dirs set, but sorted to get consistency
        let mut iter = vfs.entries(&tmpdir).unwrap().sort_by_name().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), &tmpdir);
        assert_eq!(iter.next().unwrap().unwrap().path(), &file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), &dir1);
        assert!(iter.next().is_none());

        // Filter on dirs
        let mut iter = vfs.entries(&tmpdir).unwrap().dirs().sort_by_name().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), &tmpdir);
        assert_eq!(iter.next().unwrap().unwrap().path(), &dir1);
        assert!(iter.next().is_none());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_files()
    {
        test_files(assert_vfs_setup!(Vfs::memfs()));
        test_files(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_files((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("zdir");
        let file1 = tmpdir.mash("file");
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);

        // Without files set, but sorted to get consistency
        let mut iter = vfs.entries(&tmpdir).unwrap().sort_by_name().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), &tmpdir);
        assert_eq!(iter.next().unwrap().unwrap().path(), &file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), &dir1);
        assert!(iter.next().is_none());

        // Filter on files
        let mut iter = vfs.entries(&tmpdir).unwrap().files().sort_by_name().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), &file1);
        assert!(iter.next().is_none());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_follow()
    {
        test_follow(assert_vfs_setup!(Vfs::memfs()));
        test_follow(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_follow((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let link1 = tmpdir.mash("link1");
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_symlink!(vfs, &link1, &dir1);

        // Without follow
        let mut iter = vfs.entries(&link1).unwrap().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), &link1);
        assert!(iter.next().is_none());

        // With follow
        let mut iter = vfs.entries(&link1).unwrap().follow(true).sort_by_name().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), &dir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), &file1);
        assert!(iter.next().is_none());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_depth()
    {
        test_depth(assert_vfs_setup!(Vfs::memfs()));
        test_depth(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_depth((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let dir1file1 = dir1.mash("file1");
        let file1 = tmpdir.mash("file1");
        let dir2 = dir1.mash("dir2");
        let dir2file1 = dir2.mash("file1");

        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_vfs_mkfile!(vfs, &dir1file1);
        assert_vfs_mkfile!(vfs, &dir2file1);
        assert_vfs_mkfile!(vfs, &file1);

        // Min: 0, Max: 0 = only root
        let mut iter = vfs.entries(&tmpdir).unwrap().max_depth(0).into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert!(iter.next().is_none());

        // Min: 0, Max: 1 = root and immediate children
        let iter = vfs.entries(&tmpdir).unwrap().max_depth(1).into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &file1, &dir1]);

        // Min: 0, Max: 2 = root, its immediate children and their immediate children
        let iter = vfs.entries(&tmpdir).unwrap().max_depth(2).into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &file1, &dir1, &dir2, &dir1file1]);

        // Min: 1, Max: max = skip root, all rest
        let iter = vfs.entries(&tmpdir).unwrap().min_depth(1).into_iter();
        assert_iter_eq(iter, vec![&file1, &dir1, &dir2, &dir1file1, &dir2file1]);

        // Min: 1, Max: 1 = skip root, hit root's children only
        let iter = vfs.entries(&tmpdir).unwrap().min_depth(1).max_depth(1).into_iter();
        assert_iter_eq(iter, vec![&file1, &dir1]);

        // Min: 1, Max: 2 = skip root, hit root's chilren and theirs only
        let iter = vfs.entries(&tmpdir).unwrap().min_depth(1).max_depth(2).into_iter();
        assert_iter_eq(iter, vec![&file1, &dir1, &dir2, &dir1file1]);

        // Min: 2, Max: 1 - max should get corrected to 2 because of ordering
        let iter = vfs.entries(&tmpdir).unwrap().min_depth(2).max_depth(1).into_iter();
        assert_eq!(iter.opts.min_depth, 2);
        assert_eq!(iter.opts.max_depth, 2);
        assert_iter_eq(iter, vec![&dir2, &dir1file1]);

        // Min: 2, Max: 1 - min should get corrected to 1 because of ordering
        let iter = vfs.entries(&tmpdir).unwrap().max_depth(1).min_depth(2).into_iter();
        assert_eq!(iter.opts.min_depth, 1);
        assert_eq!(iter.opts.max_depth, 1);
        assert_iter_eq(iter, vec![&file1, &dir1]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_contents_first()
    {
        test_contents_first(assert_vfs_setup!(Vfs::memfs()));
        test_contents_first(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_contents_first((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");
        let file3 = tmpdir.mash("file3");
        let link1 = tmpdir.mash("link1");

        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_vfs_mkfile!(vfs, &file3);
        assert_vfs_symlink!(vfs, &link1, &file3);

        // contents first un-sorted
        let iter = vfs.entries(&tmpdir).unwrap().contents_first().into_iter();
        assert_iter_eq(iter, vec![&link1, &file3, &file2, &dir2, &file1, &dir1, &tmpdir]);

        // contents first sorted
        let mut iter = vfs.entries(&tmpdir).unwrap().contents_first().sort_by_name().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2);
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), file3);
        assert_eq!(iter.next().unwrap().unwrap().path(), link1);
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert!(iter.next().is_none());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_sort()
    {
        test_sort(assert_vfs_setup!(Vfs::memfs()));
        test_sort(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_sort((vfs, tmpdir): (Vfs, PathBuf))
    {
        let zdir1 = tmpdir.mash("zdir1");
        let dir1file1 = zdir1.mash("file1");
        let dir1file2 = zdir1.mash("file2");
        let zdir2 = tmpdir.mash("zdir2");
        let dir2file1 = zdir2.mash("file1");
        let dir2file2 = zdir2.mash("file2");
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");

        assert_vfs_mkdir_p!(vfs, &zdir1);
        assert_vfs_mkdir_p!(vfs, &zdir2);
        assert_vfs_mkfile!(vfs, &dir1file1);
        assert_vfs_mkfile!(vfs, &dir1file2);
        assert_vfs_mkfile!(vfs, &dir2file1);
        assert_vfs_mkfile!(vfs, &dir2file2);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);

        // Without sorting
        let iter = vfs.entries(&tmpdir).unwrap().into_iter();
        assert_iter_eq(iter, vec![
            &tmpdir, &file2, &zdir1, &dir1file2, &dir1file1, &file1, &zdir2, &dir2file2, &dir2file1,
        ]);

        // with manual sorting on name
        let mut iter = vfs.entries(&tmpdir).unwrap().sort(|x, y| x.file_name().cmp(&y.file_name())).into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2file2);
        assert!(iter.next().is_none());

        // with sort by name default
        let mut iter = vfs.entries(&tmpdir).unwrap().sort_by_name().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2file2);
        assert!(iter.next().is_none());

        // sort dirs first
        let zdir3 = zdir1.mash("zdir3");
        let dir3file1 = zdir3.mash("file1");
        let dir3file2 = zdir3.mash("file2");
        assert_vfs_mkdir_p!(vfs, &zdir3);
        assert_vfs_mkfile!(vfs, &dir3file1);
        assert_vfs_mkfile!(vfs, &dir3file2);

        let mut iter = vfs.entries(&tmpdir).unwrap().dirs_first().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir3);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir3file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir3file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), file2);
        assert!(iter.next().is_none());

        // sort files first
        let mut iter = vfs.entries(&tmpdir).unwrap().files_first().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir3);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir3file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir3file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2file2);
        assert!(iter.next().is_none());

        // sort files first but in reverse aphabetic order
        let mut iter =
            vfs.entries(&tmpdir).unwrap().files_first().sort(|x, y| y.file_name().cmp(&x.file_name())).into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert_eq!(iter.next().unwrap().unwrap().path(), file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), zdir3);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir3file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir3file1);
        assert!(iter.next().is_none());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_max_descriptors()
    {
        test_max_descriptors(assert_vfs_setup!(Vfs::memfs()));
        test_max_descriptors(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_max_descriptors((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");
        let dir3 = dir2.mash("dir3");
        let file3 = dir3.mash("file3");

        assert_vfs_mkdir_p!(vfs, &dir3);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_vfs_mkfile!(vfs, &file3);

        // Without descriptor cap
        let iter = vfs.entries(&tmpdir).unwrap().into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &dir1, &dir2, &file2, &dir3, &file3, &file1]);

        // with descritor cap - should have the same pattern
        let mut paths = vfs.entries(&tmpdir).unwrap();
        paths.max_descriptors = 1;
        let iter = paths.into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &dir1, &dir2, &file2, &dir3, &file3, &file1]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    // #[test]
    // fn test_vfs_loop_detection()
    // {
    //     test_loop_detection(assert_vfs_setup!(Vfs::memfs()));
    //     test_loop_detection(assert_vfs_setup!(Vfs::stdfs()));
    // }
    // fn test_loop_detection((vfs, tmpdir): (Vfs, PathBuf))
    // {
    //     let dir1 = tmpdir.mash("dir1");
    //     let dir2 = dir1.mash("dir2");
    //     let link1 = dir2.mash("link1");

    //     assert_vfs_mkdir_p!(vfs, &dir2);
    //     assert_vfs_symlink!(vfs, &link1, &dir1);

    //     // Non follow should be fine
    //     let iter = vfs.entries(&tmpdir).unwrap().into_iter();
    //     assert_iter_eq(iter, vec![&tmpdir, &dir1, &dir2, &link1]);

    //     // Follow link will loop
    //     let mut iter = vfs.entries(&tmpdir).unwrap().follow(true).into_iter();
    //     assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2);
    //     assert_eq!(iter.next().unwrap().unwrap_err().to_string(),
    // PathError::link_looping(dir1).to_string());     assert!(iter.next().is_none());

    //     assert_vfs_remove_all!(vfs, &tmpdir);
    // }

    #[test]
    fn test_vfs_filter()
    {
        test_filter(assert_vfs_setup!(Vfs::memfs()));
        test_filter(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_filter((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");
        let file3 = tmpdir.mash("file3");
        let link1 = tmpdir.mash("link1");
        let link2 = tmpdir.mash("link2");

        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_vfs_mkfile!(vfs, &file3);
        assert_vfs_symlink!(vfs, &link2, &dir2);
        assert_vfs_symlink!(vfs, &link1, &file1);

        // Files only
        let iter = vfs.entries(&tmpdir).unwrap().files().into_iter();
        assert_iter_eq(iter, vec![&link1, &file3, &file2, &file1]);

        // Dirs only
        let iter = vfs.entries(&tmpdir).unwrap().dirs().into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &link2, &dir1, &dir2]);

        // Custom links only
        let iter = vfs.entries(&tmpdir).unwrap().into_iter().filter_p(|x| x.is_symlink());
        assert_iter_eq(iter, vec![&link1, &link2]);

        // Custom name
        let iter = vfs.entries(&tmpdir).unwrap().into_iter().filter_p(|x| x.path().has_suffix("1"));
        assert_iter_eq(iter, vec![&link1, &dir1, &file1]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_multiple()
    {
        test_multiple(assert_vfs_setup!(Vfs::memfs()));
        test_multiple(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_multiple((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");
        let file3 = tmpdir.mash("file3");
        let link1 = tmpdir.mash("link1");

        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_vfs_mkfile!(vfs, &file3);
        assert_vfs_symlink!(vfs, &link1, &file3);

        let iter = vfs.entries(&tmpdir).unwrap().into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &file3, &dir1, &dir2, &file2, &file1, &link1]);
        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_single()
    {
        test_single(assert_vfs_setup!(Vfs::memfs()));
        test_single(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_single((vfs, tmpdir): (Vfs, PathBuf))
    {
        let file1 = tmpdir.mash("file1");

        // Single directory
        let mut iter = vfs.entries(&tmpdir).unwrap().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert!(iter.next().is_none());

        // Single file
        assert_vfs_mkfile!(vfs, &file1);
        let mut iter = vfs.entries(&file1).unwrap().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert!(iter.next().is_none());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }
}

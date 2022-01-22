//! # Entries
//! Provides a builder pattern for constructing iterators for travsersing VFS filesystems. Inspired
//! by WalkDir, Entries provides a similar feature set in a simplified manner that is Virtual File
//! System (VFS) friendly.
//!
//! ## Features
//! * Support for Rivia VFS
//! * Recursive directory traversal with depth control
//! * Symbolic link following
//! * Automatic link path reading
//! * Directory entries `.` and `..` are ommitted
//!
//! ## Construction
//! Use the VFS builder functions to construct an instance e.g. sys::entries or Stdfs::entries.
//!
//! ## Traversal
//! Entries is a depth first algorithm by default with directories yielded before their contents.
//! However this behavior can be changed by setting the `contents_first` options to direct Entries
//! to yield the contents of directories first before the directory its self which is useful for
//! operations like chmod that revoke permissions to read.
//!
//! ## File Descriptors
//! Considering that most unix type systems have a limit of 1024 file descriptors, Paths is careful
//! not to exhaust this resource by limiting its internal consumption to no more than 50 at a time.
//! Anything beyond that will be read into memory and iterated from there internally rather than
//! holding more than 50 open file descriptors.

use std::{cmp::Ordering, fmt, path::Path};

use crate::{
    errors::*,
    sys::{Entry, EntryIter, VfsEntry},
    trying,
};

pub(crate) const DEFAULT_MAX_DESCRIPTORS: u16 = 50;

/// Entries provides a builder pattern for traversing VFS filesystems.
///
/// Use the VFS builder functions to construct an instance e.g. sys::entries or Stdfs::entries.
///
/// ### Examples
/// ```
/// use fungus::prelude::*;
///
/// let tmpdir = sys::abs("tests/temp/entries_doc_entries").unwrap();
/// assert!(sys::remove_all(&tmpdir).is_ok());
/// assert!(sys::mkdir(&tmpdir).is_ok());
/// let file1 = tmpdir.mash("file1");
/// assert_eq!(sys::mkfile(&file1).unwrap(), file1);
/// let mut iter = sys::entries(&tmpdir).unwrap().into_iter();
/// assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
/// assert_eq!(iter.next().unwrap().unwrap().path(), file1);
/// assert!(iter.next().is_none());
/// assert!(sys::remove_all(tmpdir).is_ok());
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
    /// Filter entries down to just directories.
    ///
    /// Default: false
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn dirs(mut self) -> Self
    {
        self.dirs = true;
        self.files = false;
        self
    }

    /// Filter entries down to just files.
    ///
    /// Default: false
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn files(mut self) -> Self
    {
        self.dirs = false;
        self.files = true;
        self
    }

    /// Set `follow` to follow links that point to directories and iterate over the contents
    /// of the linked directory as allowd by `depth`.
    ///
    /// Default: false
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn follow(mut self, yes: bool) -> Self
    {
        self.follow = yes;
        self
    }

    /// Set the min depth that Entries should traverse. The given path is considered depth 0.
    /// To only include that path and not recurse set `max_depth(0)`. By default recusrion i.e.
    /// `max_depth` is effectively unbounded.
    ///
    /// Note setting `min_depth` first will autocorrect later calls to `max_depth` to be consistent
    /// in relation to `min_depth`. The inverse would be true if `max_depth` was called first.
    ///
    /// Default: 0
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn min_depth(mut self, min: usize) -> Self
    {
        self.min_depth = min;
        if self.min_depth > self.max_depth {
            self.min_depth = self.max_depth;
        }
        self
    }

    /// Set the max depth that Entries should traverse exclusive. The given path is considered depth
    /// 0, while its children would be considered to be at depth 1. So a max of 0 would include the
    /// given path only and exclude its children. To include the given path an its children only
    /// you'd set `max_depth(1)`. By default recusrion i.e. `max_depth` is effectively unbounded.
    ///
    /// Note setting `max_depth` first will autocorrect later calls to `min_depth` to be consistent
    /// in relation to `max_depth`. The inverse would be true if `min_depth` was called first.
    ///
    /// Default: std::usize::MAX
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn max_depth(mut self, max: usize) -> Self
    {
        self.max_depth = max;
        if self.max_depth < self.min_depth {
            self.max_depth = self.min_depth;
        }
        self
    }

    /// Set the pre-operation function to run over each directory before processing. This will
    /// happen before reading the filesystem traversal and is useful for things like changing
    /// permissions or ownership to allow for recusion.
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn pre_op(mut self, op: impl FnMut(&VfsEntry) -> RvResult<()>+Send+Sync+'static) -> Self
    {
        self.pre_op = Some(Box::new(op));
        self
    }

    /// Set the default sorter to be directories first by name. This will have the affect of caching
    /// all directory entries and iterating from memory as we traverse to enforce ordering.
    ///
    /// Default: false
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn dirs_first(mut self) -> Self
    {
        self.dirs_first = true;
        self.sort(|x, y| x.file_name().cmp(&y.file_name()))
    }

    /// Set the default sorter to be files first by name. This will have the affect of caching
    /// all directory entries and iterating from memory as we traverse to enforce ordering.
    /// Default: false
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn files_first(mut self) -> Self
    {
        self.files_first = true;
        self.sort(|x, y| x.file_name().cmp(&y.file_name()))
    }

    /// Return the contents of directories before the directory itself. This is useful for
    /// operations like chmod that revoke permission on the way out.
    /// Default: false
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn contents_first(mut self) -> Self
    {
        self.contents_first = true;
        self
    }

    /// Set the default sorter to be by name. This will have the affect of caching all directory
    /// entries and iterating from memory as we traverse to enforce ordering. Default: false
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn sort_by_name(mut self) -> Self
    {
        self.sort_by_name = true;
        self.sort(|x, y| x.file_name().cmp(&y.file_name()))
    }

    /// Set a function for sorting entries.
    /// Default: None
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
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

/// Iterator for traversing VFS filesystems.
///
/// Use the VFS builder functions to construct an instance e.g. sys::entries or Stdfs::entries.
///
/// ### Examples
/// ```
/// use fungus::prelude::*;
///
/// let tmpdir = sys::abs("tests/temp/entries_doc_entriesiter").unwrap();
/// assert!(sys::remove_all(&tmpdir).is_ok());
/// assert!(sys::mkdir(&tmpdir).is_ok());
/// let file1 = tmpdir.mash("file1");
/// assert_eq!(sys::mkfile(&file1).unwrap(), file1);
/// let mut iter = sys::entries(&tmpdir).unwrap().into_iter();
/// assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
/// assert_eq!(iter.next().unwrap().unwrap().path(), file1);
/// assert!(iter.next().is_none());
/// assert!(sys::remove_all(tmpdir).is_ok());
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
    /// use fungus::prelude::*;
    ///
    /// let tmpdir = sys::abs("tests/temp/entries_doc_filter_p").unwrap();
    /// assert!(sys::remove_all(&tmpdir).is_ok());
    /// assert!(sys::mkdir(&tmpdir).is_ok());
    /// let file1 = tmpdir.mash("file1");
    /// assert_eq!(sys::mkfile(&file1).unwrap(), file1);
    /// let mut iter =
    ///     sys::entries(&tmpdir).unwrap().into_iter().filter_p(|x| x.path().has_suffix("1"));
    /// assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    /// assert!(iter.next().is_none());
    /// assert!(sys::remove_all(tmpdir).is_ok());
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
    assert_stdfs_setup_func!();

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
    fn test_stdfs_contents_first()
    {
        let tmpdir = assert_stdfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");
        let file3 = tmpdir.mash("file3");
        let link1 = tmpdir.mash("link1");

        assert_stdfs_mkdir_p!(&dir2);
        assert_stdfs_mkfile!(&file1);
        assert_stdfs_mkfile!(&file2);
        assert_stdfs_mkfile!(&file3);
        assert_eq!(Stdfs::symlink(&file3, &link1).unwrap(), link1);

        // contents first un-sorted
        let iter = Stdfs::entries(&tmpdir).unwrap().contents_first().into_iter();
        assert_iter_eq(iter, vec![&link1, &file3, &file2, &dir2, &file1, &dir1, &tmpdir]);

        // contents first sorted
        let mut iter = Stdfs::entries(&tmpdir).unwrap().contents_first().dirs_first().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2);
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), file3);
        assert_eq!(iter.next().unwrap().unwrap().path(), link1);
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert!(iter.next().is_none());

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_stdfs_sort()
    {
        let tmpdir = assert_stdfs_setup!();
        let zdir1 = tmpdir.mash("zdir1");
        let dir1file1 = zdir1.mash("file1");
        let dir1file2 = zdir1.mash("file2");
        let zdir2 = tmpdir.mash("zdir2");
        let dir2file1 = zdir2.mash("file1");
        let dir2file2 = zdir2.mash("file2");
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");

        assert_stdfs_mkdir_p!(&zdir1);
        assert_stdfs_mkdir_p!(&zdir2);
        assert_stdfs_mkfile!(&dir1file1);
        assert_stdfs_mkfile!(&dir1file2);
        assert_stdfs_mkfile!(&dir2file1);
        assert_stdfs_mkfile!(&dir2file2);
        assert_stdfs_mkfile!(&file1);
        assert_stdfs_mkfile!(&file2);

        // Without sorting
        let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
        assert_iter_eq(iter, vec![
            &tmpdir, &file2, &zdir1, &dir1file2, &dir1file1, &file1, &zdir2, &dir2file2, &dir2file1,
        ]);

        // with sorting on name
        let mut iter = Stdfs::entries(&tmpdir).unwrap().sort(|x, y| x.file_name().cmp(&y.file_name())).into_iter();
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

        // with sort default set
        let mut iter = Stdfs::entries(&tmpdir).unwrap().sort_by_name().into_iter();
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
        assert_stdfs_mkdir_p!(&zdir3);
        assert_stdfs_mkfile!(&dir3file1);
        assert_stdfs_mkfile!(&dir3file2);

        let mut iter = Stdfs::entries(&tmpdir).unwrap().dirs_first().into_iter();
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
        let mut iter = Stdfs::entries(&tmpdir).unwrap().files_first().into_iter();
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
        let mut iter = Stdfs::entries(&tmpdir)
            .unwrap()
            .files_first()
            .sort(|x, y| y.file_name().cmp(&x.file_name()))
            .into_iter();
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

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_stdfs_max_descriptors()
    {
        let tmpdir = assert_stdfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");
        let dir3 = dir2.mash("dir3");
        let file3 = dir3.mash("file3");

        assert_stdfs_mkdir_p!(&dir3);
        assert_stdfs_mkfile!(&file1);
        assert_stdfs_mkfile!(&file2);
        assert_stdfs_mkfile!(&file3);

        // Without descriptor cap
        let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &dir1, &dir2, &file2, &dir3, &file3, &file1]);

        // with descritor cap - should have the same pattern
        let mut paths = Stdfs::entries(&tmpdir).unwrap();
        paths.max_descriptors = 1;
        let iter = paths.into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &dir1, &dir2, &file2, &dir3, &file3, &file1]);

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_stdfs_loop_detection()
    {
        let tmpdir = assert_stdfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        let dir2 = dir1.mash("dir2");
        let link1 = dir2.mash("link1");

        assert_stdfs_mkdir_p!(&dir2);
        assert_eq!(Stdfs::symlink(&dir1, &link1).unwrap(), link1);

        // Non follow should be fine
        let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &dir1, &dir2, &link1]);

        // Follow link will loop
        let mut iter = Stdfs::entries(&tmpdir).unwrap().follow(true).into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2);
        assert_eq!(iter.next().unwrap().unwrap_err().to_string(), PathError::link_looping(dir1).to_string());
        assert!(iter.next().is_none());

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_stdfs_filter()
    {
        let tmpdir = assert_stdfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");
        let file3 = tmpdir.mash("file3");
        let link1 = tmpdir.mash("link1");
        let link2 = tmpdir.mash("link2");

        assert_stdfs_mkdir_p!(&dir2);
        assert_stdfs_mkfile!(&file1);
        assert_stdfs_mkfile!(&file2);
        assert_stdfs_mkfile!(&file3);
        assert_eq!(Stdfs::symlink(&dir2, &link2).unwrap(), link2);
        assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);

        // Files only
        let iter = Stdfs::entries(&tmpdir).unwrap().files().into_iter();
        assert_iter_eq(iter, vec![&link1, &file3, &file2, &file1]);

        // Dirs only
        let iter = Stdfs::entries(&tmpdir).unwrap().dirs().into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &link2, &dir1, &dir2]);

        // Custom links only
        let mut iter = Stdfs::entries(&tmpdir).unwrap().into_iter().filter_p(|x| x.is_symlink());
        assert_iter_eq(iter, vec![&link1, &link2]);

        // Custom name
        let iter = Stdfs::entries(&tmpdir).unwrap().into_iter().filter_p(|x| x.path().has_suffix("1"));
        assert_iter_eq(iter, vec![&link1, &dir1, &file1]);

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_stdfs_follow()
    {
        let tmpdir = assert_stdfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");
        let file3 = tmpdir.mash("file3");
        let link1 = tmpdir.mash("link1");

        assert_stdfs_mkdir_p!(&dir2);
        assert_stdfs_mkfile!(&file1);
        assert_stdfs_mkfile!(&file2);
        assert_stdfs_mkfile!(&file3);
        assert_eq!(Stdfs::symlink(&dir2, &link1).unwrap(), link1);

        // Follow off
        let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &link1, &file3, &dir1, &dir2, &file2, &file1]);

        // Follow on
        let iter = Stdfs::entries(&tmpdir).unwrap().follow(true).into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &dir2, &file2, &file3, &dir1, &dir2, &file2, &file1]);

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_stdfs_depth()
    {
        let tmpdir = assert_stdfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        let dir1file1 = dir1.mash("file1");
        let file1 = tmpdir.mash("file1");
        let dir2 = dir1.mash("dir2");
        let dir2file1 = dir2.mash("file1");

        assert_stdfs_mkdir_p!(&dir2);
        assert_stdfs_mkfile!(&dir1file1);
        assert_stdfs_mkfile!(&dir2file1);
        assert_stdfs_mkfile!(&file1);

        // Min: 0, Max: 0 = only root
        let mut iter = Stdfs::entries(&tmpdir).unwrap().max_depth(0).into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert!(iter.next().is_none());

        // Min: 0, Max: 1 = root and immediate children
        let iter = Stdfs::entries(&tmpdir).unwrap().max_depth(1).into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &file1, &dir1]);

        // Min: 0, Max: 2 = root, its immediate children and their immediate children
        let iter = Stdfs::entries(&tmpdir).unwrap().max_depth(2).into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &file1, &dir1, &dir2, &dir1file1]);

        // Min: 1, Max: max = skip root, all rest
        let iter = Stdfs::entries(&tmpdir).unwrap().min_depth(1).into_iter();
        assert_iter_eq(iter, vec![&file1, &dir1, &dir2, &dir1file1, &dir2file1]);

        // Min: 1, Max: 1 = skip root, hit root's children only
        let iter = Stdfs::entries(&tmpdir).unwrap().min_depth(1).max_depth(1).into_iter();
        assert_iter_eq(iter, vec![&file1, &dir1]);

        // Min: 1, Max: 2 = skip root, hit root's chilren and theirs only
        let iter = Stdfs::entries(&tmpdir).unwrap().min_depth(1).max_depth(2).into_iter();
        assert_iter_eq(iter, vec![&file1, &dir1, &dir2, &dir1file1]);

        // Min: 2, Max: 1 - max should get corrected to 2 because of ordering
        let iter = Stdfs::entries(&tmpdir).unwrap().min_depth(2).max_depth(1).into_iter();
        assert_eq!(iter.opts.min_depth, 2);
        assert_eq!(iter.opts.max_depth, 2);
        assert_iter_eq(iter, vec![&dir2, &dir1file1]);

        // Min: 2, Max: 1 - min should get corrected to 1 because of ordering
        let iter = Stdfs::entries(&tmpdir).unwrap().max_depth(1).min_depth(2).into_iter();
        assert_eq!(iter.opts.min_depth, 1);
        assert_eq!(iter.opts.max_depth, 1);
        assert_iter_eq(iter, vec![&file1, &dir1]);

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_stdfs_multiple()
    {
        let tmpdir = assert_stdfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");
        let file3 = tmpdir.mash("file3");
        let link1 = tmpdir.mash("link1");

        assert_stdfs_mkdir_p!(&dir2);
        assert_stdfs_mkfile!(&file1);
        assert_stdfs_mkfile!(&file2);
        assert_stdfs_mkfile!(&file3);
        assert_eq!(Stdfs::symlink(&file3, &link1).unwrap(), link1);

        let iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
        assert_iter_eq(iter, vec![&tmpdir, &file3, &dir1, &dir2, &file2, &file1, &link1]);
        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_memfs_single()
    {
        // Single directory
        let memfs = Memfs::new();
        assert_eq!(memfs.mkdir_p("dir1").unwrap(), PathBuf::from("/dir1"));
        let mut iter = memfs.entries("dir1").unwrap().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/dir1"));
        assert!(iter.next().is_none());

        // Single file
        assert_eq!(memfs.mkfile("dir1/file1").unwrap(), PathBuf::from("/dir1/file1"));
        let mut iter = memfs.entries("/dir1/file1").unwrap().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), PathBuf::from("/dir1/file1"));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_stdfs_single()
    {
        let tmpdir = assert_stdfs_setup!();
        let file1 = tmpdir.mash("file1");

        // Single directory
        let mut iter = Stdfs::entries(&tmpdir).unwrap().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), tmpdir);
        assert!(iter.next().is_none());

        // Single file
        assert!(Stdfs::mkfile(&file1).is_ok());
        let mut iter = Stdfs::entries(&file1).unwrap().into_iter();
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert!(iter.next().is_none());

        assert_stdfs_remove_all!(&tmpdir);
    }
}

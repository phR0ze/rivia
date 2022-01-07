//! # Entry provides a trait for a single extrapolated filesystem item.
//!
//! Entry can be implemented by multiple filesystem backend providers to provide an extensible
//! way to reuse algorithms related to file systems.
//! 
//! ## Switch backend providers
//!
//! ### Example
/// ```
/// use rivia_core::*;
/// ```
use crate::errors::*;

use std::{
    cmp::Ordering,
    ffi::OsStr,
    fmt::Debug,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

/// Entry provides a virtual filesystem trait for a single filesystem item.
pub trait Entry: Debug+Send+Sync+'static {
    /// `path` reports the actual file or directory path when `is_symlink` reports false. When
    /// `is_symlink` reports true and `follow` reports true `path` will report the actual file
    /// or directory that the link points to and `alt` will report the link's path. When
    /// `is_symlink` reports true and `follow` reports false `path` will report the link's path
    /// and `alt` will report the actual file or directory the link points to.
    fn path(&self) -> &Path;

    /// Move the `path` value out of this struct as an owned value
    fn path_buf(self) -> PathBuf;

    /// `alt` will be empty unless `is_symlink` reports true. When `is_symlink` reports true and
    /// `follow` reports true `alt` will report the path to the link and `path` will report the
    /// path to the actual file or directory the link points to. When `is_symlink` reports true
    /// and `follow` reports false `alt` will report the actual file or directory the link points
    /// to and `path` will report the link path.
    fn alt(&self) -> &Path;

    /// Move the `link` value out of this struct as an owned value
    fn alt_buf(self) -> PathBuf;

    /// File name of the entry
    fn file_name(&self) -> Option<&OsStr> {
        self.path().file_name()
    }

    /// Switch the `path` and `alt` values if `is_symlink` reports true.
    fn follow(self, follow: bool) -> dyn Entry;

    /// Return the current following state
    fn following(&self) -> bool;

    /// Regular directories and symlinks that point to directories will report true.
    fn is_dir(&self) -> bool;

    /// Regular files and symlinks that point to files will report true.
    fn is_file(&self) -> bool;

    /// Links will report true
    fn is_symlink(&self) -> bool;

    /// Link to a directory will report true meaning that the original path given refers to a
    /// link and the path pointed to by the link refers to a directory.
    fn is_symlink_dir(&self) -> bool {
        self.is_symlink() && self.is_dir()
    }

    /// Link to a file will report true meaning that the original path given refers to a
    /// link and the path pointed to by the link refers to a file.
    fn is_symlink_file(&self) -> bool {
        self.is_symlink() && self.is_file()
    }

    /// Reports the mode of the path
    fn mode(&self) -> u32;

    // /// Create an iterator from the given `path` to iterate over just the contents
    // /// of this path non-recursively.
    // fn iter(&self) -> RvResult<EntryIter>;

    /// Up cast the trait type to the enum wrapper
    fn upcast(self) -> Self;
}

/// `EntryIter` provides iteration over a directory in a filesystem agnostic way.
///
/// ### Cached
/// Optionally all entries can be read into memory from the underlying filesystem and yielded
/// from there by invoking the `cache` method. In this way the number of open file descriptors
/// can be controlled at the cost of memory consumption.
pub struct EntryIter {
    path: PathBuf,
    cached: bool,
    following: bool,
    iter: Box<dyn Iterator<Item=RvResult<dyn Entry>>>,
}

impl EntryIter {
    /// Return a reference to the internal path being iterated over
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads the remaining portion of the VFS backend iterator into memory then creates a new
    /// EntryIter that will iterate over the new cached entries.
    pub fn cache(&mut self) {
        if !self.cached {
            self.cached = true;
            self.iter = Box::new(self.collect::<Vec<_>>().into_iter());
        }
    }

    /// Return the current cached state
    pub fn cached(&self) -> bool {
        self.cached
    }

    /// Sort directories first than files according to the given sort function
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn dirs_first(&mut self, cmp: impl Fn(&Entry, &Entry) -> Ordering) {
        self.cached = true;
        let (mut dirs, mut files) = self._split();
        self._sort(&mut dirs, &cmp);
        self._sort(&mut files, cmp);
        self.iter = Box::new(dirs.into_iter().chain(files.into_iter()));
    }

    /// Sort files first than directories according to the given sort function
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn files_first(&mut self, cmp: impl Fn(&Entry, &Entry) -> Ordering) {
        self.cached = true;
        let (mut dirs, mut files) = self._split();
        self._sort(&mut dirs, &cmp);
        self._sort(&mut files, cmp);
        self.iter = Box::new(files.into_iter().chain(dirs.into_iter()));
    }

    /// When `true` iterating results will have their `path` and `alt` values switched if
    /// their `is_symlink` reports true.
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn follow(mut self, follow: bool) -> Self {
        self.following = follow;
        self
    }

    /// Return the current following state
    pub fn following(&self) -> bool {
        self.following
    }

    /// Sort the entries according to the given sort function
    ///
    /// ### Examples
    /// ```
    /// use fungus::prelude::*;
    /// ```
    pub fn sort(&mut self, cmp: impl Fn(&Entry, &Entry) -> Ordering) {
        self.cached = true;
        let mut entries = self.collect::<Vec<_>>();
        self._sort(&mut entries, cmp);
        self.iter = Box::new(entries.into_iter());
    }

    /// Sort the given entries with the given sorter function
    fn _sort(
        &mut self, entries: &mut Vec<RvResult<Entry>>, cmp: impl Fn(&Entry, &Entry) -> Ordering,
    ) {
        entries.sort_by(|x, y| match (x, y) {
            (&Ok(ref x), &Ok(ref y)) => cmp(x, y),
            (&Err(_), &Err(_)) => Ordering::Equal,
            (&Ok(_), &Err(_)) => Ordering::Greater,
            (&Err(_), &Ok(_)) => Ordering::Less,
        });
    }

    /// Split the files and directories out
    fn _split(&mut self) -> (Vec<RvResult<Entry>>, Vec<RvResult<Entry>>) {
        let mut dirs: Vec<RvResult<Entry>> = vec![];
        let mut files: Vec<RvResult<Entry>> = vec![];
        for x in self.collect::<Vec<_>>() {
            if let Ok(entry) = x {
                if entry.is_dir() {
                    dirs.push(Ok(entry));
                } else {
                    files.push(Ok(entry));
                }
            } else {
                // push errors on the dirs iterator to trigger errors at the client level
                dirs.push(x);
            }
        }
        (dirs, files)
    }
}

impl Iterator for EntryIter {
    type Item = RvResult<Entry>;

    fn next(&mut self) -> Option<RvResult<Entry>> {
        match self.iter.next() {
            Some(x) => Some(match x {
                Ok(y) => Ok(if self.following {
                    // Switch path and alt if is_link
                    y.follow(self.following)
                } else {
                    y
                }),
                Err(e) => Err(e),
            }),
            None => None,
        }
    }
}

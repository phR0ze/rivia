use std::{
    cmp::{self, Ordering},
    collections::{HashMap, HashSet},
    fmt, fs,
    hash::{Hash, Hasher},
    io,
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
};

use itertools::Itertools;

use super::MemfsEntries;
use crate::{
    errors::*,
    exts::*,
    sys::{self, Entry, EntryIter, PathExt, VfsEntry},
    trying,
};

// MemfsEntryOpts implements the builder pattern to provide advanced options for creating
// MemfsEntry instances
#[derive(Debug)]
pub(crate) struct MemfsEntryOpts
{
    path: PathBuf, // path of the entry
    alt: PathBuf,  // alternate path for the entry, used with links
    dir: bool,     // is this entry a dir
    file: bool,    // is this entry a file
    link: bool,    // is this entry a link
    mode: u32,     // permission mode of the entry
}

impl MemfsEntryOpts
{
    // Create a MemfsEntry instance from the MemfsEntryOpts instance
    pub(crate) fn new(self) -> MemfsEntry
    {
        MemfsEntry {
            files: if self.dir { Some(HashSet::new()) } else { None },
            path: self.path,
            alt: self.alt,
            dir: self.dir,
            file: self.file,
            link: self.link,
            mode: self.mode,
            follow: false,
            cached: false,
        }
    }

    pub(crate) fn alt<T: Into<PathBuf>>(mut self, path: T) -> Self
    {
        self.alt = path.into();
        self
    }

    pub(crate) fn dir(mut self) -> Self
    {
        self.dir = true;
        self.file = false;
        self
    }

    pub(crate) fn file(mut self) -> Self
    {
        self.file = true;
        self.dir = false;
        self
    }

    pub(crate) fn link(mut self) -> Self
    {
        self.link = true;
        self
    }

    pub(crate) fn mode(mut self, mode: u32) -> Self
    {
        self.mode = mode;
        self
    }
}

/// MemfsEntry is an implementation of a single entry in a virtual filesystem.
///
/// ### Example
/// ```
/// use rivia::prelude::*;
/// ```
#[derive(Debug)]
pub struct MemfsEntry
{
    pub(crate) path: PathBuf,                  // entry path
    pub(crate) alt: PathBuf,                   // alternate entry path, used with links
    pub(crate) dir: bool,                      // is this entry a dir
    pub(crate) file: bool,                     // is this entry a file
    pub(crate) link: bool,                     // is this entry a link
    pub(crate) mode: u32,                      // permission mode of the entry
    pub(crate) follow: bool,                   // tracks if the path and alt have been switched
    pub(crate) cached: bool,                   // tracks if properties have been cached
    pub(crate) files: Option<HashSet<String>>, // file or directory names
}

impl MemfsEntry
{
    /// Create a new MemfsEntryOpts to allow for more advanced Memfs creation
    ///
    /// # Arguments
    /// * `abs` - target path expected to already be in absolute form
    pub(crate) fn opts<T: Into<PathBuf>>(path: T) -> MemfsEntryOpts
    {
        MemfsEntryOpts {
            path: path.into(),
            alt: PathBuf::new(),
            dir: true, // directory by default
            file: false,
            link: false,
            mode: 0,
        }
    }

    // Add an entry to this directory
    //
    // # Arguments
    // * `entry` - the entry to add to this directory
    //
    // # Errors
    // * PathError::IsNotDir(PathBuf) when this entry is not a directory.
    // * PathError::ExistsAlready(PathBuf) when the given entry already exists.
    // entry's path
    pub(crate) fn add<T: Into<String>>(&mut self, entry: T) -> RvResult<()>
    {
        let name = entry.into();

        // Ensure this is a valid directory
        if !self.dir {
            return Err(PathError::is_not_dir(&self.path).into());
        }

        // Insert the new entry or error out if already exists
        if let Some(ref mut files) = self.files {
            if !files.insert(name.clone()) {
                let path = self.path.mash(name);
                return Err(PathError::exists_already(path).into());
            }
        } else {
            let mut files = HashSet::new();
            files.insert(name);
            self.files = Some(files);
        }

        Ok(())
    }

    // Remove an entry from this directory. Returns true on success or false if there was no file to
    // remove.
    //
    // # Arguments
    // * `entry` - the entry to remove from this directory
    //
    // # Errors
    // * PathError::IsNotDir(PathBuf) when this entry is not a directory.
    // * PathError::ExistsAlready(PathBuf) when the given entry already exists.
    // entry's path
    pub(crate) fn remove<T: Into<String>>(&mut self, entry: T) -> RvResult<()>
    {
        let name = entry.into();

        // Ensure this is a valid directory
        if !self.dir {
            return Err(PathError::is_not_dir(&self.path).into());
        }

        // Remove the entry
        if let Some(ref mut files) = self.files {
            files.remove(&name);
        }

        Ok(())
    }

    /// Switch the `path` and `alt` values if `is_link` reports true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub(crate) fn follow(mut self, follow: bool) -> Self
    {
        if follow && !self.follow {
            self.follow = true;
            if self.link {
                let path = self.path;
                self.path = self.alt;
                self.alt = path;
            }
        }
        self
    }
}

impl Entry for MemfsEntry
{
    /// `path` reports the actual file or directory when `is_symlink` reports false. When
    /// `is_symlink` reports true and `follow` reports true `path` will report the actual file or
    /// directory that the link points to and `alt` will report the link's path. When `is_symlink`
    /// reports true and `follow` reports false `path` will report the link's path and `alt` will
    /// report the actual file or directory the link points to.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn path(&self) -> &Path
    {
        &self.path
    }

    /// Move the `path` value out of this struct as an owned value
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn path_buf(self) -> PathBuf
    {
        self.path
    }

    /// `alt` will be empty unless `is_symlink` reports true. When `is_symlink` reports true and
    /// `follow` reports true `alt` will report the path to the link and `path` will report the
    /// path to the actual file or directory the link points to. When `is_symlink` reports trueand
    /// `follow` reports false `alt` will report the actual file or directory the link points to
    /// and `path` will report the link path.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn alt(&self) -> &Path
    {
        &self.alt
    }

    /// Move the `link` value out of this struct as an owned value
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn alt_buf(self) -> PathBuf
    {
        self.alt
    }

    /// Switch the `path` and `alt` values if `is_symlink` reports true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn follow(self, follow: bool) -> VfsEntry
    {
        VfsEntry::Memfs(self.follow(follow))
    }

    /// Return the current following state
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn following(&self) -> bool
    {
        self.follow
    }

    /// Regular directories and symlinks that point to directories will report
    /// true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_dir(&self) -> bool
    {
        self.dir
    }

    /// Regular files and symlinks that point to files will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_file(&self) -> bool
    {
        self.file
    }

    /// Links will report true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_symlink(&self) -> bool
    {
        self.link
    }

    /// Reports the mode of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn mode(&self) -> u32
    {
        self.mode
    }

    // /// Create an iterator from the given path to iterate over just the contents of this path
    // /// non-recursively.
    // ///
    // /// ### Examples
    // /// ```
    // /// use rivia::prelude::*;
    // /// ```
    // fn iter(&self) -> RvResult<EntryIter>
    // {
    //     // self.entries(false);
    //     // MemfsEntry::iter(&self.path, false)
    // }

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn upcast(self) -> VfsEntry
    {
        VfsEntry::Memfs(self)
    }
}

impl Clone for MemfsEntry
{
    fn clone(&self) -> Self
    {
        Self {
            path: self.path.clone(),
            alt: self.alt.clone(),
            dir: self.dir,
            file: self.file,
            link: self.link,
            mode: self.mode,
            follow: self.follow,
            cached: self.cached,
            files: self.files.clone(),
        }
    }
}

pub(crate) struct MemfsEntryIter
{
    iter: Box<dyn Iterator<Item=PathBuf>>,
    entries: Arc<MemfsEntries>,
}

impl MemfsEntryIter
{
    /// Create a new memfs iterator for the given directory only
    ///
    /// # Arguments
    /// * `entry` - target entry to read the directory from
    /// * `memfs` - shared copy of the memory filessystem
    pub(crate) fn new<T: AsRef<Path>>(path: T, entries: Arc<MemfsEntries>) -> RvResult<Self>
    {
        let path = path.as_ref();
        if let Some(entry) = entries.get(path) {
            // Create an iterator over Vec<PathBuf>
            let mut items = vec![];
            if let Some(ref files) = entry.files {
                for name in files.iter() {
                    items.push(path.mash(name));
                }
            }
            Ok(MemfsEntryIter {
                iter: Box::new(items.into_iter()),
                entries,
            })
        } else {
            Err(PathError::does_not_exist(path).into())
        }
    }
}

impl Iterator for MemfsEntryIter
{
    type Item = RvResult<VfsEntry>;

    fn next(&mut self) -> Option<RvResult<VfsEntry>>
    {
        if let Some(value) = self.iter.next() {
            if let Some(x) = self.entries.get(&value).clone() {
                return Some(Ok(x.clone().upcast()));
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

    #[test]
    fn test_iter()
    {
        // MemfsEntryIter::new();
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

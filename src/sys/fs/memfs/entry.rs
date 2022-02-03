use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use super::MemfsEntries;
use crate::{
    errors::*,
    sys::{Entry, PathExt, VfsEntry},
};

// MemfsEntryOpts implements the builder pattern to provide advanced options for creating
// MemfsEntry instances
#[derive(Debug)]
pub(crate) struct MemfsEntryOpts
{
    path: PathBuf, // path of the entry
    alt: PathBuf,  // abs path to target link is pointing to
    rel: PathBuf,  // relative path to target link is pointing to
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
            rel: self.rel,
            dir: self.dir,
            file: self.file,
            link: self.link,
            mode: self.mode,
            follow: false,
            cached: false,
        }
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

    pub(crate) fn link_to<T: Into<PathBuf>>(mut self, path: T) -> RvResult<Self>
    {
        self.link = true;
        self.alt = path.into();
        self.rel = self.alt.relative(self.path.dir()?)?;
        Ok(self)
    }

    pub(crate) fn mode(mut self, mode: u32) -> Self
    {
        self.mode = mode;
        self
    }
}

/// Provides a Vfs backend [`Entry`] implementation for Memfs
///
/// ### Example
/// ```
/// use rivia::prelude::*;
/// ```
#[derive(Debug)]
pub struct MemfsEntry
{
    pub(crate) path: PathBuf,                  // abs path
    pub(crate) alt: PathBuf,                   // abs path link is pointing to
    pub(crate) rel: PathBuf,                   // relative path link is pointing to
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
    /// * `abs` - target path expected to already be in absolute form
    pub(crate) fn opts<T: Into<PathBuf>>(path: T) -> MemfsEntryOpts
    {
        MemfsEntryOpts {
            path: path.into(),
            alt: PathBuf::new(),
            rel: PathBuf::new(),
            dir: true, // directory by default
            file: false,
            link: false,
            mode: 0,
        }
    }

    // Add an entry to this directory
    //
    // * `entry` - the entry to add to this directory
    //
    // ### Errors
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
    /// Returns the actual file or directory path when `is_symlink` reports false
    ///
    /// * When `is_symlink` returns true and `following` returns true `path` will return the actual
    ///   file or directory that the link points to and `alt` will report the link's path
    /// * When `is_symlink` returns true and `following` returns false `path` will report the link's
    ///   path and `alt` will report the actual file or directory the link points to.
    ///
    /// ### Examples
    /// ```ignore
    /// use rivia::prelude::*;
    ///
    /// let entry = MemfsEntry::opts("foo").new();
    /// entry.path();
    /// ```
    fn path(&self) -> &Path
    {
        &self.path
    }

    /// Returns a PathBuf of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn path_buf(&self) -> PathBuf
    {
        self.path.clone()
    }

    /// Returns the path the link is pointing to if `is_symlink` reports true
    ///
    /// * When `is_symlink` returns true and `following` returns true `path` will return the actual
    ///   file or directory that the link points to and `alt` will report the link's path
    /// * When `is_symlink` returns true and `following` returns false `path` will report the link's
    ///   path and `alt` will report the actual file or directory the link points to.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn alt(&self) -> &Path
    {
        &self.alt
    }

    /// Returns a PathBuf of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn alt_buf(&self) -> PathBuf
    {
        self.alt.clone()
    }

    /// Returns the path the link is pointing to in relative form if `is_symlink` reports true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn rel(&self) -> &Path
    {
        &self.rel
    }

    /// Retunrns a PathBuf of the relative path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn rel_buf(&self) -> PathBuf
    {
        self.rel.clone()
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
            rel: self.rel.clone(),
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
    fn test_follow()
    {
        let memfs = Memfs::new();

        // Check that follow switchs the path and alt path
        let path = memfs.root().mash("link");
        let target = memfs.root().mash("target");
        let entry = MemfsEntry::opts(&path).link_to(&target).unwrap().new();
        assert_eq!(entry.path(), &path);
        assert_eq!(entry.alt(), &target);
        assert_eq!(entry.rel(), Path::new("target"));
        let entry = entry.follow(true);
        assert_eq!(entry.path(), &target);
        assert_eq!(entry.alt(), &path);
        assert_eq!(entry.rel(), Path::new("target"));
    }
}

use crate::{
    errors::*,
    fs::{VfsEntry, EntryIter, Entry},
    trying,
};

use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

/// MemfsEntry is an implementation a virtual filesystem trait for a single filesystem item. It is implemented
///
/// ### Example
/// ```
/// use rivia::prelude::*;
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct MemfsEntry {
    data: Vec<u8>, // memory file data
    path: PathBuf, // path of the entry
    alt: PathBuf,  // alternate path for the entry, used with links
    dir: bool,     // is this entry a dir
    file: bool,    // is this entry a file
    link: bool,    // is this entry a link
    mode: u32,     // permission mode of the entry
    follow: bool,  // tracks if the path and alt have been switched
    cached: bool,  // tracks if properties have been cached
}

impl Default for MemfsEntry {

    /// Defaults to an empty directory
    fn default() -> Self
    {
        Self {
            data: vec![],
            path: PathBuf::new(),
            alt: PathBuf::new(),
            dir: true,              // Set directory to true by default
            file: false,
            link: false,
            mode: 0,
            follow: false,
            cached: false,
        }
    }
}

impl Clone for MemfsEntry {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            path: self.path.clone(),
            alt: self.alt.clone(),
            dir: self.dir,
            file: self.file,
            link: self.link,
            mode: self.mode,
            follow: self.follow,
            cached: self.cached,
        }
    }
}

impl MemfsEntry {
    /// Create a Memfs entry for the given path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub(crate) fn new<T: Into<PathBuf>>(path: T) -> Self {
        MemfsEntry
        {
            data: vec![],
            path: path.into(),
            ..Default::default()
        }
    }

    /// Set the entry to be a directory. Will automatically set file and link to false.
    /// In order to have a link that points to a directory you need to call link() after this call.
    pub fn dir(mut self) -> Self {
        self.file = false;
        self.link = false;
        self.dir = true;
        self
    }

    /// Set the entry to be a file. Will automatically set dir and link to false.
    /// In order to have a link that points to a file you need to call link() after this call.
    pub fn file(mut self) -> Self {
        self.dir = false;
        self.link = false;
        self.file = true;
        self
    }

    /// Set the entry to be a link
    pub fn link(mut self) -> Self {
        self.link = true;
        self
    }

    /// Create an iterator from the given path to iterate over just the contents
    /// of this path non-recursively.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn iter(path: &Path, follow: bool) -> RvResult<EntryIter>
    {
        Ok(EntryIter {
            path: path.to_path_buf(),
            cached: false,
            following: follow,
            iter: Box::new(MemfsEntryIter(fs::read_dir(path)?)),
        })
    }

    /// Switch the `path` and `alt` values if `is_symlink` reports true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn follow(mut self, follow: bool) -> Self
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
    /// `is_symlink` reports true and `follow` reports true `path` will report the actual file
    /// or directory that the link points to and `alt` will report the link's path. When
    /// `is_symlink` reports true and `follow` reports false `path` will report the link's path
    /// and `alt` will report the actual file or directory the link points to.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn path(&self) -> &Path {
        &self.path
    }

    /// Move the `path` value out of this struct as an owned value
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn path_buf(self) -> PathBuf {
        self.path
    }

    /// `alt` will be empty unless `is_symlink` reports true. When `is_symlink` reports true and
    /// `follow` reports true `alt` will report the path to the link and `path` will report the
    /// path to the actual file or directory the link points to. When `is_symlink` reports true
    /// and `follow` reports false `alt` will report the actual file or directory the link points
    /// to and `path` will report the link path.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn alt(&self) -> &Path {
        &self.alt
    }

    /// Move the `link` value out of this struct as an owned value
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn alt_buf(self) -> PathBuf {
        self.alt
    }

    /// Switch the `path` and `alt` values if `is_symlink` reports true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn follow(self, follow: bool) -> VfsEntry {
        VfsEntry::Memfs(self.follow(follow))
    }

    /// Return the current following state
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn following(&self) -> bool {
        self.follow
    }

    /// Regular directories and symlinks that point to directories will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_dir(&self) -> bool {
        self.dir
    }

    /// Regular files and symlinks that point to files will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_file(&self) -> bool {
        self.file
    }

    /// Links will report true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_symlink(&self) -> bool {
        self.link
    }

    /// Reports the mode of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn mode(&self) -> u32 {
        self.mode
    }

    /// Create an iterator from the given path to iterate over just the contents
    /// of this path non-recursively.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn iter(&self) -> RvResult<EntryIter> {
        MemfsEntry::iter(&self.path, false)
    }

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn upcast(self) -> VfsEntry {
        VfsEntry::Memfs(self)
    }
}

#[derive(Debug)]
struct MemfsEntryIter(fs::ReadDir);
impl Iterator for MemfsEntryIter {
    type Item = RvResult<VfsEntry>;

    fn next(&mut self) -> Option<RvResult<VfsEntry>> {
        // if let Some(value) = self.0.next() {
        //     return Some(match MemfsEntry::from(&trying!(value).path()) {
        //         Ok(x) => Ok(x.upcast()),
        //         Err(e) => Err(e),
        //     });
        // }
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
    fn test_root_dir() -> RvResult<()> {
        let memfs = Memfs::new();
        let data = memfs.read_all(Path::new("/"))?;
        Ok(())
    }
}
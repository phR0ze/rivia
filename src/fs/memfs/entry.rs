use crate::{
    errors::*,
    fs::{VfsEntry, EntryIter, Entry, Stdfs},
    trying,
};

use std::{
    cmp::Ordering,
    ffi::OsStr,
    fmt::Debug,
    fs,
    os::unix::fs::PermissionsExt,
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
    path: PathBuf, // path of the entry
    alt: PathBuf,  // alternate path for the entry, used with links
    dir: bool,     // is this entry a dir
    file: bool,    // is this entry a file
    link: bool,    // is this entry a link
    mode: u32,     // permission mode of the entry
    follow: bool,  // tracks if the path and alt have been switched
    cached: bool,  // tracsk if properties have been cached
}

impl Default for MemfsEntry {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            alt: PathBuf::new(),
            dir: false,
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
    /// Create a Memfs entry using the given properties
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    pub(crate) fn new<T: Into<PathBuf>>(
        path: T, alt: T, dir: bool, file: bool, link: bool, mode: u32, follow: bool, cached: bool,
    ) -> Self {
        MemfsEntry { path: path.into(), alt: alt.into(), dir, file, link, mode, follow, cached }
    }

    /// Create a Memfs entry from the given path. The path is always expanded, cleaned and
    /// turned into an absolute value. Additionally filesystem properties are cached.
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    pub fn from<T: AsRef<Path>>(path: T) -> RvResult<Self> {
        let path = Stdfs::abs(path)?;
        let mut link = false;
        let mut alt = PathBuf::new();
        let mut meta = fs::symlink_metadata(&path)?;

        // Load link information for links
        if meta.file_type().is_symlink() {
            link = true;
            let src = fs::read_link(&path)?;

            // Ensure src is rooted properly
            let rooted = if !src.is_absolute() {
                Stdfs::mash(Stdfs::dir(&path)?, src)
            } else {
                src
            };

            // Set the link's source
            alt = Stdfs::abs(rooted)?;

            // Switch to the link's source metadata
            meta = fs::metadata(&path)?;
        }

        Ok(MemfsEntry {
            path,
            alt,
            dir: meta.is_dir(),
            file: meta.is_file(),
            link,
            mode: meta.permissions().mode(),
            follow: false,
            cached: true,
        })
    }

    /// Create an iterator from the given path to iterate over just the contents
    /// of this path non-recursively.
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    pub fn iter(path: &Path, follow: bool) -> RvResult<EntryIter> {
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
    /// use rivia::*;
    /// ```
    pub fn follow(mut self, follow: bool) -> Self {
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

impl Entry for MemfsEntry {
    /// `path` reports the actual file or directory when `is_symlink` reports false. When
    /// `is_symlink` reports true and `follow` reports true `path` will report the actual file
    /// or directory that the link points to and `alt` will report the link's path. When
    /// `is_symlink` reports true and `follow` reports false `path` will report the link's path
    /// and `alt` will report the actual file or directory the link points to.
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    fn path(&self) -> &Path {
        &self.path
    }

    /// Move the `path` value out of this struct as an owned value
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
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
    /// use rivia::*;
    /// ```
    fn alt(&self) -> &Path {
        &self.alt
    }

    /// Move the `link` value out of this struct as an owned value
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    fn alt_buf(self) -> PathBuf {
        self.alt
    }

    /// Switch the `path` and `alt` values if `is_symlink` reports true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    fn follow(self, follow: bool) -> VfsEntry {
        VfsEntry::Memfs(self.follow(follow))
    }

    /// Return the current following state
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    fn following(&self) -> bool {
        self.follow
    }

    /// Regular directories and symlinks that point to directories will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    fn is_dir(&self) -> bool {
        self.dir
    }

    /// Regular files and symlinks that point to files will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    fn is_file(&self) -> bool {
        self.file
    }

    /// Links will report true
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    fn is_symlink(&self) -> bool {
        self.link
    }

    /// Reports the mode of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    fn mode(&self) -> u32 {
        self.mode
    }

    /// Create an iterator from the given path to iterate over just the contents
    /// of this path non-recursively.
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
    /// ```
    fn iter(&self) -> RvResult<EntryIter> {
        MemfsEntry::iter(&self.path, false)
    }

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::*;
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
        if let Some(value) = self.0.next() {
            return Some(match MemfsEntry::from(&trying!(value).path()) {
                Ok(x) => Ok(x.upcast()),
                Err(e) => Err(e),
            });
        }
        None
    }
}

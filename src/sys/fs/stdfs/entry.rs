use std::{
    fmt::Debug,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use crate::{
    errors::*,
    sys::{Entry, PathExt, Stdfs, VfsEntry},
    trying,
};

/// Provides a Vfs backend [`Entry`] implementation for Stdfs
///
/// ## Features
/// * Caching of filesystem properties for cheap access
/// * One time path expansion, cleaning and absolute value resolution
/// * Simplified link behavior
///
/// ### Performance
/// New entries created with Stdfs::from will automatically have filesystem properties cached for
/// cheap access as well as a one time path expansion/cleaning and absolute value resolution.
/// Further access will use the cached values reducing the overhead of constant absolute path
/// checking. Refreshing the cached properties can be done by creating a new Entry with Stdfs::from.
///
/// ### Link behavior
/// Although patterned after std::fs::DirEntry's behavior Entry deviates in that `is_dir`, `is_file`
/// and `is_symlink` are not mutually exclusive. `is_dir` and `is_file` will always follow links to
/// report on the actual file or directory. Thus it is possible for `is_symlink` to report as true
/// when we have a link and `is_dir` to report as true if following the link we have a directory.
/// The same is true for the file side. `path` will report the actual file or directory when
/// `is_symlink` reports false and `alt` will be empty. When `is_symlink` reports true and `follow`
/// reports true `path` will report the actual file or directory that the link points to and `alt`
/// will report the link's path. When `is_symlink` reports true and `follow` reports false `path`
/// will report the link's path and `alt` will report the actual file or directory the link points
/// to. With Paths controlling this behavior Entry should behave intuitiveely. However if different
/// behavior is desired checking the `follow` and `is_
#[derive(Debug, PartialEq, Eq)]
pub struct StdfsEntry
{
    pub(crate) path: PathBuf, // abs path
    pub(crate) alt: PathBuf,  // abs path link is pointing to
    pub(crate) rel: PathBuf,  // relative path link is pointing to
    pub(crate) dir: bool,     // is this entry a dir
    pub(crate) file: bool,    // is this entry a file
    pub(crate) link: bool,    // is this entry a link
    pub(crate) mode: u32,     // permission mode of the entry
    pub(crate) follow: bool,  // tracks if the path and alt have been switched
    pub(crate) cached: bool,  // tracsk if properties have been cached
}

impl Default for StdfsEntry
{
    fn default() -> Self
    {
        Self {
            path: PathBuf::new(),
            alt: PathBuf::new(),
            rel: PathBuf::new(),
            dir: false,
            file: false,
            link: false,
            mode: 0,
            follow: false,
            cached: false,
        }
    }
}

impl Clone for StdfsEntry
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
        }
    }
}

impl StdfsEntry
{
    /// Create a Stdfs entry from the given path
    ///
    /// * Handles path expansion and absolute path resolution
    /// * Filesystem properties are cached during load
    pub(crate) fn from<T: AsRef<Path>>(path: T) -> RvResult<Self>
    {
        let path = Stdfs::abs(path)?;
        if !Stdfs::exists(&path) {
            return Err(PathError::does_not_exist(&path).into());
        }
        let mut link = false;
        let mut alt = PathBuf::new();
        let mut rel = PathBuf::new();
        let mut meta = fs::symlink_metadata(&path)?;

        // Load link information for links
        if meta.file_type().is_symlink() {
            link = true;
            let target = fs::read_link(&path)?;

            // Ensure target is an absolute path
            alt = Stdfs::abs(if !target.is_absolute() { path.dir()?.mash(target) } else { target })?;

            // Get the target path relative to the link path if possible
            rel = alt.relative(path.dir()?)?;

            // Switch to the link's source metadata
            meta = fs::metadata(&path)?;
        }

        Ok(StdfsEntry {
            path,
            alt,
            rel,
            dir: meta.is_dir(),
            file: meta.is_file(),
            link,
            mode: meta.permissions().mode(),
            follow: false,
            cached: true,
        })
    }
}

impl Entry for StdfsEntry
{
    /// Returns the actual file or directory path when `is_symlink` reports false
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
    fn follow(mut self, follow: bool) -> VfsEntry
    {
        if follow && self.link && !self.follow {
            self.follow = true;
            let path = self.path;
            self.path = self.alt;
            self.alt = path;
        }
        self.upcast()
    }

    /// Return the current following state. Only applies to symlinks
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn following(&self) -> bool
    {
        self.follow
    }

    /// Regular directories and symlinks that point to directories will report true.
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
        VfsEntry::Stdfs(self)
    }
}

#[derive(Debug)]
pub(crate) struct StdfsEntryIter
{
    pub(crate) dir: fs::ReadDir,
}
impl Iterator for StdfsEntryIter
{
    type Item = RvResult<VfsEntry>;

    fn next(&mut self) -> Option<RvResult<VfsEntry>>
    {
        if let Some(value) = self.dir.next() {
            return Some(match StdfsEntry::from(&trying!(value).path()) {
                Ok(x) => Ok(x.upcast()),
                Err(e) => Err(e),
            });
        }
        None
    }
}

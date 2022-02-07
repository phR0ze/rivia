use std::{
    cmp::Ordering,
    ffi::OsStr,
    fmt::Debug,
    path::{Path, PathBuf},
};

use crate::{
    errors::*,
    sys::{MemfsEntry, StdfsEntry},
};

/// Defines a virtual file system entry that can be used generically across all Vfs provider
/// backends
///
/// * [`StdfsEntry`] and [`MemfsEntry`] provide the fundamental implementations
///
/// ### Example
/// ```
/// use rivia::prelude::*;
/// ```
pub trait Entry: Debug+Send+Sync+'static
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
    fn path(&self) -> &Path;

    /// Returns a PathBuf of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn path_buf(&self) -> PathBuf;

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
    fn alt(&self) -> &Path;

    /// Returns a PathBuf of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn alt_buf(&self) -> PathBuf;

    /// Returns the path the link is pointing to in relative form if `is_symlink` reports true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn rel(&self) -> &Path;

    /// Retunrns a PathBuf of the relative path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn rel_buf(&self) -> PathBuf;

    /// File name of the entry
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn file_name(&self) -> Option<&OsStr>
    {
        self.path().file_name()
    }

    /// Switch the `path` and `alt` values if `is_symlink` reports true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn follow(self, follow: bool) -> VfsEntry;

    /// Return the current following state
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn following(&self) -> bool;

    /// Returns true if this path is executable
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_exec(&self) -> bool
    {
        self.mode() & 0o111 != 0
    }

    /// Regular directories and symlinks that point to directories will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_dir(&self) -> bool;

    /// Regular files and symlinks that point to files will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_file(&self) -> bool;

    /// Returns true if this path is readonly
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_readonly(&self) -> bool
    {
        self.mode() & 0o222 == 0
    }

    /// Links will report true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_symlink(&self) -> bool;

    /// Link to a directory will report true meaning that the original path given refers to a
    /// link and the path pointed to by the link refers to a directory.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_symlink_dir(&self) -> bool
    {
        self.is_symlink() && self.is_dir()
    }

    /// Link to a file will report true meaning that the original path given refers to a
    /// link and the path pointed to by the link refers to a file.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_symlink_file(&self) -> bool
    {
        self.is_symlink() && self.is_file()
    }

    /// Reports the mode of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn mode(&self) -> u32;

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn upcast(self) -> VfsEntry;
}

/// Provides an ergonomic encapsulation of the underlying Vfs [`Entry`] backend implementations
#[derive(Debug)]
pub enum VfsEntry
{
    Stdfs(StdfsEntry),
    Memfs(MemfsEntry),
}

impl Clone for VfsEntry
{
    fn clone(&self) -> Self
    {
        match self {
            VfsEntry::Stdfs(x) => VfsEntry::Stdfs(x.clone()),
            VfsEntry::Memfs(x) => VfsEntry::Memfs(x.clone()),
        }
    }
}

impl Entry for VfsEntry
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
        match self {
            VfsEntry::Stdfs(x) => x.path(),
            VfsEntry::Memfs(x) => x.path(),
        }
    }

    /// Returns a PathBuf of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn path_buf(&self) -> PathBuf
    {
        match self {
            VfsEntry::Stdfs(x) => x.path_buf(),
            VfsEntry::Memfs(x) => x.path_buf(),
        }
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
        match self {
            VfsEntry::Stdfs(x) => x.alt(),
            VfsEntry::Memfs(x) => x.alt(),
        }
    }

    /// Returns a PathBuf of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn alt_buf(&self) -> PathBuf
    {
        match self {
            VfsEntry::Stdfs(x) => x.alt_buf(),
            VfsEntry::Memfs(x) => x.alt_buf(),
        }
    }

    /// Returns the path the link is pointing to in relative form if `is_symlink` reports true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn rel(&self) -> &Path
    {
        match self {
            VfsEntry::Stdfs(x) => x.rel(),
            VfsEntry::Memfs(x) => x.rel(),
        }
    }

    /// Retunrns a PathBuf of the relative path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn rel_buf(&self) -> PathBuf
    {
        match self {
            VfsEntry::Stdfs(x) => x.rel_buf(),
            VfsEntry::Memfs(x) => x.rel_buf(),
        }
    }

    /// Switch the `path` and `alt` values if `is_symlink` reports true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn follow(self, follow: bool) -> VfsEntry
    {
        match self {
            VfsEntry::Stdfs(x) => x.follow(follow).upcast(),
            VfsEntry::Memfs(x) => x.follow(follow).upcast(),
        }
    }

    /// Return the current following state
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn following(&self) -> bool
    {
        match self {
            VfsEntry::Stdfs(x) => x.following(),
            VfsEntry::Memfs(x) => x.following(),
        }
    }

    /// Regular directories and symlinks that point to directories will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_dir(&self) -> bool
    {
        match self {
            VfsEntry::Stdfs(x) => x.is_dir(),
            VfsEntry::Memfs(x) => x.is_dir(),
        }
    }

    /// Regular files and symlinks that point to files will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_file(&self) -> bool
    {
        match self {
            VfsEntry::Stdfs(x) => x.is_file(),
            VfsEntry::Memfs(x) => x.is_file(),
        }
    }

    /// Returns true if this path is readonly
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_readonly(&self) -> bool
    {
        match self {
            VfsEntry::Stdfs(x) => x.is_readonly(),
            VfsEntry::Memfs(x) => x.is_readonly(),
        }
    }

    /// Links will report true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_symlink(&self) -> bool
    {
        match self {
            VfsEntry::Stdfs(x) => x.is_symlink(),
            VfsEntry::Memfs(x) => x.is_symlink(),
        }
    }

    /// Reports the mode of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn mode(&self) -> u32
    {
        match self {
            VfsEntry::Stdfs(x) => x.mode(),
            VfsEntry::Memfs(x) => x.mode(),
        }
    }

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn upcast(self) -> VfsEntry
    {
        match self {
            VfsEntry::Stdfs(x) => x.upcast(),
            VfsEntry::Memfs(x) => x.upcast(),
        }
    }
}

/// Provides iteration over a single directory in a VFS filesystem.
///
/// ### Cached
/// Optionally all entries can be read into memory from the underlying VFS and yielded from there
/// by invoking the `cache` method. In this way the number of open file descriptors can be
/// controlled at the cost of memory consumption.
pub(crate) struct EntryIter
{
    pub(crate) path: PathBuf,
    pub(crate) cached: bool,
    pub(crate) following: bool,
    pub(crate) iter: Box<dyn Iterator<Item=RvResult<VfsEntry>>>,
}

impl EntryIter
{
    /// Return a reference to the internal path being iterated over
    pub fn path(&self) -> &Path
    {
        &self.path
    }

    /// Reads the remaining portion of the VFS backend iterator into memory then creates a new
    /// EntryIter that will iterate over the new cached entries.
    pub fn cache(&mut self)
    {
        if !self.cached {
            self.cached = true;
            self.iter = Box::new(self.collect::<Vec<_>>().into_iter());
        }
    }

    /// Return the current cached state
    pub fn cached(&self) -> bool
    {
        self.cached
    }

    /// Sort directories first than files according to the given sort function
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn dirs_first(&mut self, cmp: impl Fn(&VfsEntry, &VfsEntry) -> Ordering)
    {
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
    /// use rivia::prelude::*;
    /// ```
    pub fn files_first(&mut self, cmp: impl Fn(&VfsEntry, &VfsEntry) -> Ordering)
    {
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
    /// use rivia::prelude::*;
    /// ```
    pub fn follow(mut self, follow: bool) -> Self
    {
        self.following = follow;
        self
    }

    /// Return the current following state
    pub fn following(&self) -> bool
    {
        self.following
    }

    /// Sort the entries according to the given sort function
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn sort(&mut self, cmp: impl Fn(&VfsEntry, &VfsEntry) -> Ordering)
    {
        self.cached = true;
        let mut entries = self.collect::<Vec<_>>();
        self._sort(&mut entries, cmp);
        self.iter = Box::new(entries.into_iter());
    }

    /// Sort the given entries with the given sorter function
    fn _sort(&mut self, entries: &mut Vec<RvResult<VfsEntry>>, cmp: impl Fn(&VfsEntry, &VfsEntry) -> Ordering)
    {
        entries.sort_by(|x, y| match (x, y) {
            (&Ok(ref x), &Ok(ref y)) => cmp(x, y),
            (&Err(_), &Err(_)) => Ordering::Equal,
            (&Ok(_), &Err(_)) => Ordering::Greater,
            (&Err(_), &Ok(_)) => Ordering::Less,
        });
    }

    /// Split the files and directories out
    fn _split(&mut self) -> (Vec<RvResult<VfsEntry>>, Vec<RvResult<VfsEntry>>)
    {
        let mut dirs: Vec<RvResult<VfsEntry>> = vec![];
        let mut files: Vec<RvResult<VfsEntry>> = vec![];
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

impl Iterator for EntryIter
{
    type Item = RvResult<VfsEntry>;

    fn next(&mut self) -> Option<RvResult<VfsEntry>>
    {
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

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[test]
    fn test_vfs_entry_alt_rel()
    {
        test_entry_alt_rel(assert_vfs_setup!(Vfs::memfs()));
        test_entry_alt_rel(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_entry_alt_rel((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let link1 = tmpdir.mash("link1");

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_symlink!(vfs, &link1, &dir1);
        let entry = vfs.entry(&link1).unwrap();
        assert_eq!(entry.alt(), &dir1);
        assert_eq!(entry.alt_buf(), dir1);
        assert_eq!(entry.rel(), Path::new("dir1"));
        assert_eq!(entry.rel_buf(), PathBuf::from("dir1"));

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    // #[test]
    // fn test_vfs_dirs_first_files_first()
    // {
    //     let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
    //     let dir1 = sys::mash(&tmpdir, "dir1");
    //     let dir2 = sys::mash(&tmpdir, "dir2");
    //     let file1 = sys::mash(&tmpdir, "file1");
    //     let file2 = sys::mash(&tmpdir, "file2");

    //     assert_vfs_mkdir_p!(vfs, &dir1);
    //     assert_vfs_mkdir_p!(vfs, &dir2);
    //     assert_vfs_mkfile!(vfs, &file1);
    //     assert_vfs_mkfile!(vfs, &file2);

    //     // dirs first
    //     let mut iter = StdfsEntry::from(&tmpdir).unwrap().iter().unwrap();
    //     iter.dirs_first(|x, y| x.file_name().cmp(&y.file_name()));
    //     assert_eq!(iter.cached(), true);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert!(iter.next().is_none());

    //     // files first
    //     let mut iter = StdfsEntry::from(&tmpdir).unwrap().iter().unwrap();
    //     iter.files_first(|x, y| x.file_name().cmp(&y.file_name()));
    //     assert_eq!(iter.cached(), true);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), dir2);
    //     assert!(iter.next().is_none());

    //     assert_vfs_remove_all!(vfs, &tmpdir);
    // }

    // #[test]
    // fn test_stdfs_entry_sort()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let file1 = sys::mash(&tmpdir, "file1");
    //     let file2 = sys::mash(&tmpdir, "file2");

    //     assert_stdfs_touch!(&file1);
    //     assert_stdfs_touch!(&file2);

    //     // custom sort for files
    //     let mut iter = StdfsEntry::from(&tmpdir).unwrap().iter().unwrap();
    //     iter.sort(|x, y| x.file_name().cmp(&y.file_name()));
    //     assert_eq!(iter.cached(), true);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert!(iter.next().is_none());

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_entry_iter()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let file1 = sys::mash(&tmpdir, "file1");
    //     let file2 = sys::mash(&tmpdir, "file2");

    //     assert_stdfs_touch!(&file1);
    //     assert_stdfs_touch!(&file2);

    //     // open file descriptors
    //     let mut iter = StdfsEntry::from(&tmpdir).unwrap().iter().unwrap();
    //     assert_eq!(iter.cached, false);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert!(iter.next().is_none());

    //     // caching in memory
    //     let mut iter = StdfsEntry::from(&tmpdir).unwrap().iter().unwrap();
    //     iter.cache();
    //     assert_eq!(iter.cached(), true);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file2);
    //     assert_eq!(iter.next().unwrap().unwrap().path(), file1);
    //     assert!(iter.next().is_none());

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_entry_is_dir()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = sys::mash(&tmpdir, "dir1");
    //     let file1 = sys::mash(&tmpdir, "file1");
    //     let link1 = sys::mash(&tmpdir, "link1");
    //     let link2 = sys::mash(&tmpdir, "dir2");

    //     // regular directory
    //     assert_stdfs_mkdir_p!(&dir1);
    //     assert_eq!(StdfsEntry::from(&dir1).unwrap().is_dir(), true);

    //     // Current dir
    //     assert_eq!(StdfsEntry::from(&PathBuf::from(".")).unwrap().is_dir(), true);

    //     // file is not a directory
    //     assert_stdfs_touch!(&file1);
    //     assert_eq!(StdfsEntry::from(&file1).unwrap().is_dir(), false);

    //     // file link is not a directory
    //     assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);
    //     assert_eq!(StdfsEntry::from(&link1).unwrap().is_dir(), false);
    //     assert_eq!(StdfsEntry::from(&link1).unwrap().is_symlink_file(), true);

    //     // dir link is a directory
    //     assert_eq!(Stdfs::symlink(&dir1, &link2).unwrap(), link2);
    //     assert_eq!(StdfsEntry::from(&link2).unwrap().is_dir(), true);
    //     assert_eq!(StdfsEntry::from(&link2).unwrap().is_symlink_dir(), true);

    //     // invalid directory
    //     assert_eq!(StdfsEntry::from(&PathBuf::from("/foobar")).unwrap_err().to_string(), "No such
    // file or directory (os error 2)");

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_entry_is_file()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = sys::mash(&tmpdir, "dir1");
    //     let file1 = sys::mash(&tmpdir, "file1");
    //     let link1 = sys::mash(&tmpdir, "link1");
    //     let link2 = sys::mash(&tmpdir, "dir2");

    //     // regular directory is not a file
    //     assert_stdfs_mkdir_p!(&dir1);
    //     assert_eq!(StdfsEntry::from(&dir1).unwrap().is_file(), false);

    //     // Current dir is not a file
    //     assert_eq!(StdfsEntry::from(&PathBuf::from(".")).unwrap().is_file(), false);

    //     // regular file is true
    //     assert_stdfs_touch!(&file1);
    //     assert_eq!(StdfsEntry::from(&file1).unwrap().is_file(), true);

    //     // file link is not a regular file ist a symlink_file
    //     assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);
    //     assert_eq!(StdfsEntry::from(&link1).unwrap().is_file(), true);
    //     assert_eq!(StdfsEntry::from(&link1).unwrap().is_symlink_file(), true);

    //     // dir link is not a directory
    //     assert_eq!(Stdfs::symlink(&dir1, &link2).unwrap(), link2);
    //     assert_eq!(StdfsEntry::from(&link2).unwrap().is_file(), false);
    //     assert_eq!(StdfsEntry::from(&link2).unwrap().is_symlink_dir(), true);

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_stdfs_entry_is_symlink()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = sys::mash(&tmpdir, "dir1");
    //     let file1 = sys::mash(&tmpdir, "file1");
    //     let link1 = sys::mash(&tmpdir, "link1");
    //     let link2 = sys::mash(&tmpdir, "link2");

    //     // invalid
    //     assert!(StdfsEntry::from(&PathBuf::from("")).is_err());

    //     // non-existing file or dir is not a symlink
    //     assert_eq!(StdfsEntry::from(&link1).unwrap_err().to_string(), "No such file or directory
    // (os error 2)");     assert_eq!(StdfsEntry::from(&dir1).unwrap_err().to_string(), "No such
    // file or directory (os error 2)");

    //     // regular file is not a symlink
    //     assert!(Stdfs::touch(&file1).is_ok());
    //     assert_eq!(StdfsEntry::from(&file1).unwrap().is_symlink(), false);

    //     // symlink file is a symlink
    //     assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);
    //     assert_eq!(StdfsEntry::from(&link1).unwrap().is_symlink(), true);

    //     // regular dir is not a symlink
    //     assert_stdfs_mkdir_p!(&dir1);
    //     assert_eq!(StdfsEntry::from(&dir1).unwrap().is_symlink(), false);

    //     // symlink dir is a symlink
    //     assert_eq!(Stdfs::symlink(&dir1, &link2).unwrap(), link2);
    //     assert_eq!(StdfsEntry::from(&link2).unwrap().is_symlink(), true);

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    #[test]
    fn test_vfs_entry_is_symlink_dir()
    {
        test_entry_is_symlink_dir(assert_vfs_setup!(Vfs::memfs()));
        test_entry_is_symlink_dir(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_entry_is_symlink_dir((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let link1 = tmpdir.mash("link1");

        // regular dir is not a symlink dir
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_eq!(vfs.entry(&dir1).unwrap().is_symlink_dir(), false);

        // test absolute
        assert_vfs_symlink!(vfs, &link1, &dir1);
        assert_eq!(vfs.entry(&link1).unwrap().is_symlink_dir(), true);
        assert_eq!(vfs.entry(&link1).unwrap().is_symlink_file(), false);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_entry_is_symlink_file()
    {
        test_entry_is_symlink_file(assert_vfs_setup!(Vfs::memfs()));
        test_entry_is_symlink_file(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_entry_is_symlink_file((vfs, tmpdir): (Vfs, PathBuf))
    {
        let file1 = tmpdir.mash("file1");
        let link1 = tmpdir.mash("link1");

        // regular dir is not a symlink dir
        assert_vfs_mkfile!(vfs, &file1);
        assert_eq!(vfs.entry(&file1).unwrap().is_symlink_file(), false);

        // test absolute
        assert_vfs_symlink!(vfs, &link1, &file1);
        assert_eq!(vfs.entry(&link1).unwrap().is_symlink_dir(), false);
        assert_eq!(vfs.entry(&link1).unwrap().is_symlink_file(), true);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    // #[test]
    // fn test_stdfs_entry_readlink_abs()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let file1 = sys::mash(&tmpdir, "file1");
    //     let link1 = sys::mash(&tmpdir, "link1");
    //     let dir1 = sys::mash(&tmpdir, "dir1");
    //     let link2 = sys::mash(&dir1, "link2");
    //     let link3 = sys::mash(&dir1, "link3");
    //     let link4 = sys::mash(&dir1, "link4");

    //     // link at the same level
    //     assert!(Stdfs::touch(&file1).is_ok());
    //     assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);
    //     assert_eq!(StdfsEntry::from(&link1).unwrap().path(), link1);
    //     assert_eq!(StdfsEntry::from(&link1).unwrap().alt(), file1);
    //     assert_eq!(StdfsEntry::from(&link1).unwrap().follow(true).path(), file1);
    //     assert_eq!(StdfsEntry::from(&link1).unwrap().follow(true).alt(), link1);
    //     assert_eq!(Stdfs::readlink_abs(&link1).unwrap(), file1);

    //     // link nested one deeper
    //     assert!(Stdfs::mkdir_p(&dir1).is_ok());
    //     assert_eq!(Stdfs::symlink(&file1, &link2).unwrap(), link2);
    //     assert_eq!(StdfsEntry::from(&link2).unwrap().path(), link2);
    //     assert_eq!(StdfsEntry::from(&link2).unwrap().alt(), file1);
    //     assert_eq!(StdfsEntry::from(&link2).unwrap().follow(true).path(), file1);
    //     assert_eq!(StdfsEntry::from(&link2).unwrap().follow(true).alt(), link2);
    //     assert_eq!(Stdfs::readlink_abs(&link2).unwrap(), file1);

    //     // absolute
    //     assert!(std::os::unix::fs::symlink(&file1, &link3).is_ok());
    //     assert_eq!(StdfsEntry::from(&link3).unwrap().path(), link3);
    //     assert_eq!(StdfsEntry::from(&link3).unwrap().alt(), file1);
    //     assert_eq!(StdfsEntry::from(&link3).unwrap().follow(true).path(), file1);
    //     assert_eq!(StdfsEntry::from(&link3).unwrap().follow(true).alt(), link3);
    //     assert_eq!(Stdfs::readlink_abs(&link3).unwrap(), file1);

    //     // absolute path with symbols
    //     assert!(std::os::unix::fs::symlink(sys::mash(&dir1, "../file1"), &link4).is_ok());
    //     assert_eq!(StdfsEntry::from(&link4).unwrap().path(), link4);
    //     assert_eq!(StdfsEntry::from(&link4).unwrap().alt(), file1);
    //     assert_eq!(StdfsEntry::from(&link4).unwrap().follow(true).path(), file1);
    //     assert_eq!(StdfsEntry::from(&link4).unwrap().follow(true).alt(), link4);
    //     assert_eq!(Stdfs::readlink_abs(&link4).unwrap(), file1);

    //     assert_stdfs_remove_all!(&tmpdir);
    // }
}

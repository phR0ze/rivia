use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use crate::{
    errors::*,
    sys::{Entry, VfsEntry},
};

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
    #[allow(dead_code)]
    pub fn follow(mut self, follow: bool) -> Self
    {
        self.following = follow;
        self
    }

    /// Return the current following state
    #[allow(dead_code)]
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
    fn test_entry_iter_error()
    {
        let vfs = Memfs::new();
        let tmpdir = PathBuf::new();
        let guard = vfs.read_guard();
        if let Err(e) = vfs._entry_iter(&guard, &tmpdir) {
            assert_eq!(e.to_string(), PathError::Empty.to_string());
        }
    }

    #[test]
    fn test_entry_iter_dirs_first()
    {
        let vfs = Memfs::new();
        let tmpdir = vfs.root().mash("tmpdir");
        let dir1 = tmpdir.mash("dir1");
        let dir2 = tmpdir.mash("dir2");
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);

        let guard = vfs.read_guard();

        // dirs first
        let mut iter = vfs._entry_iter(&guard, &tmpdir).unwrap()(&tmpdir, false).unwrap();
        iter.dirs_first(|x, y| x.file_name().cmp(&y.file_name()));
        assert_eq!(iter.cached(), true);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2);
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), file2);
        assert!(iter.next().is_none());

        // files first
        let mut iter = vfs._entry_iter(&guard, &tmpdir).unwrap()(&tmpdir, false).unwrap();
        iter.files_first(|x, y| x.file_name().cmp(&y.file_name()));
        assert_eq!(iter.cached(), true);
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), file2);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir1);
        assert_eq!(iter.next().unwrap().unwrap().path(), dir2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_entry_sort()
    {
        let vfs = Memfs::new();
        let tmpdir = vfs.root().mash("tmpdir");
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");

        assert_vfs_mkdir_p!(vfs, &tmpdir);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);

        // custom sort for files
        let guard = vfs.read_guard();
        let mut iter = vfs._entry_iter(&guard, &tmpdir).unwrap()(&tmpdir, false).unwrap();
        iter.sort(|x, y| x.file_name().cmp(&y.file_name()));
        assert_eq!(iter.cached(), true);
        assert_eq!(iter.next().unwrap().unwrap().path(), file1);
        assert_eq!(iter.next().unwrap().unwrap().path(), file2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_entry_follow()
    {
        let vfs = Memfs::new();
        let tmpdir = vfs.root().mash("tmpdir");
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");
        let file3 = vfs.root().mash("file3");
        let link1 = tmpdir.mash("link");

        assert_vfs_mkdir_p!(vfs, &tmpdir);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file2);
        assert_vfs_mkfile!(vfs, &file3);
        assert_vfs_symlink!(vfs, &link1, &file3);

        let guard = vfs.read_guard();

        // custom sort for files
        let iter = vfs._entry_iter(&guard, &tmpdir).unwrap()(&tmpdir, false).unwrap();
        assert_eq!(iter.following(), false);
        let mut iter = iter.follow(true);
        assert_eq!(iter.following(), true);
        iter.sort(|x, y| x.file_name().cmp(&y.file_name()));
        assert_eq!(iter.cached(), true);

        // because we sort on the path and we have follow set which switches the path and alt
        // sort order will be based on the file name not the link name
        let item1 = iter.next().unwrap().unwrap();
        assert_eq!(item1.following(), false);
        assert_eq!(item1.path(), &file1);

        let item2 = iter.next().unwrap().unwrap();
        assert_eq!(item2.following(), false);
        assert_eq!(item2.path(), &file2);

        let item3 = iter.next().unwrap().unwrap();
        assert_eq!(item3.following(), true);
        assert_eq!(item3.path(), &file3);
        assert_eq!(item3.alt(), &link1);

        assert!(iter.next().is_none());
    }
}

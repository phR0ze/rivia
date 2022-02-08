use std::{
    ffi::OsStr,
    fmt::Debug,
    path::{Path, PathBuf},
};

use crate::sys::{MemfsEntry, StdfsEntry};

/// Defines a virtual file system entry that can be used generically across all Vfs provider
/// backends
///
/// * [`StdfsEntry`] and [`MemfsEntry`] provide the fundamental implementations
///
/// ### Example
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Memfs::new();
/// let file = vfs.root().mash("file");
/// assert_vfs_mkfile!(vfs, &file);
/// let entry = vfs.entry(&file).unwrap();
/// assert_eq!(entry.path(), &file);
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
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap();
    /// assert_eq!(entry.path(), &file);
    /// ```
    fn path(&self) -> &Path;

    /// Returns a PathBuf of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap();
    /// assert_eq!(entry.path_buf(), file);
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
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// let entry = vfs.entry(&link).unwrap();
    /// assert_eq!(entry.alt(), &file);
    /// ```
    fn alt(&self) -> &Path;

    /// Returns a PathBuf of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// let entry = vfs.entry(&link).unwrap();
    /// assert_eq!(entry.alt_buf(), file);
    /// ```
    fn alt_buf(&self) -> PathBuf;

    /// Returns the path the link is pointing to in relative form if `is_symlink` reports true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// let entry = vfs.entry(&link).unwrap();
    /// assert_eq!(entry.rel(), Path::new("file"));
    /// ```
    fn rel(&self) -> &Path;

    /// Retunrns a PathBuf of the relative path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// let entry = vfs.entry(&link).unwrap();
    /// assert_eq!(entry.rel_buf(), PathBuf::from("file"));
    /// ```
    fn rel_buf(&self) -> PathBuf;

    /// File name of the entry
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// let entry = vfs.entry(&link).unwrap();
    /// assert_eq!(entry.file_name().unwrap(), "link");
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
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// let entry = vfs.entry(&link).unwrap();
    /// let entry = entry.follow(false);
    /// ```
    fn follow(self, follow: bool) -> VfsEntry;

    /// Return the current following state
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// let entry = vfs.entry(&link).unwrap();
    /// assert_eq!(entry.following(), false);
    /// let entry = entry.follow(true);
    /// assert_eq!(entry.following(), true);
    /// ```
    fn following(&self) -> bool;

    /// Returns true if this path is executable
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap();
    /// assert_eq!(entry.is_exec(), false);
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
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap();
    /// assert_eq!(entry.is_dir(), false);
    /// ```
    fn is_dir(&self) -> bool;

    /// Regular files and symlinks that point to files will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap();
    /// assert_eq!(entry.is_file(), true);
    /// ```
    fn is_file(&self) -> bool;

    /// Returns true if this path is readonly
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap();
    /// assert_eq!(entry.is_readonly(), false);
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
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap();
    /// assert_eq!(entry.is_symlink(), false);
    /// ```
    fn is_symlink(&self) -> bool;

    /// Link to a directory will report true meaning that the original path given refers to a
    /// link and the path pointed to by the link refers to a directory.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap();
    /// assert_eq!(entry.is_symlink_dir(), false);
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
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap();
    /// assert_eq!(entry.is_symlink_file(), false);
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
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap();
    /// assert_ne!(entry.mode(), 0o40644);
    /// ```
    fn mode(&self) -> u32;

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// assert_vfs_mkfile!(vfs, &file);
    /// let entry = vfs.entry(&file).unwrap().upcast();
    /// assert_eq!(entry.is_file(), true);
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

    /// Return the current following state. Only applies to symlinks
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

    #[test]
    fn test_vfs_entry_is_dir()
    {
        test_entry_is_dir(assert_vfs_setup!(Vfs::memfs()));
        test_entry_is_dir(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_entry_is_dir((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let file1 = tmpdir.mash("file1");

        assert_vfs_mkfile!(vfs, &file1);
        assert_eq!(vfs.entry(&file1).unwrap().is_dir(), false);
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_eq!(vfs.entry(&dir1).unwrap().is_dir(), true);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_entry_is_file()
    {
        test_entry_is_file(assert_vfs_setup!(Vfs::memfs()));
        test_entry_is_file(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_entry_is_file((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let file1 = tmpdir.mash("file1");

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_eq!(vfs.entry(&dir1).unwrap().is_file(), false);
        assert_vfs_mkfile!(vfs, &file1);
        assert_eq!(vfs.entry(&file1).unwrap().is_file(), true);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_entry_is_readonly()
    {
        test_entry_is_readonly(assert_vfs_setup!(Vfs::memfs()));
        test_entry_is_readonly(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_entry_is_readonly((vfs, tmpdir): (Vfs, PathBuf))
    {
        let file1 = tmpdir.mash("file1");

        assert_vfs_mkfile!(vfs, &file1);
        assert_eq!(vfs.entry(&file1).unwrap().is_readonly(), false);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_entry_is_symlink()
    {
        test_entry_is_symlink(assert_vfs_setup!(Vfs::memfs()));
        test_entry_is_symlink(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_entry_is_symlink((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let link1 = tmpdir.mash("link1");

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_eq!(vfs.entry(&dir1).unwrap().is_symlink(), false);
        assert_vfs_symlink!(vfs, &link1, &dir1);
        assert_eq!(vfs.entry(&link1).unwrap().is_symlink(), true);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

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

    #[test]
    fn test_vfs_entry_follow()
    {
        test_entry_follow(assert_vfs_setup!(Vfs::memfs()));
        test_entry_follow(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_entry_follow((vfs, tmpdir): (Vfs, PathBuf))
    {
        let file1 = tmpdir.mash("file1");
        let link1 = tmpdir.mash("link1");

        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_symlink!(vfs, &link1, &file1);
        let entry = vfs.entry(&link1).unwrap();
        assert_eq!(entry.path(), &link1);
        assert_eq!(entry.following(), false);
        let entry = entry.follow(true);
        assert_eq!(entry.path(), &file1);
        assert_eq!(entry.following(), true);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_entry_upcast()
    {
        test_entry_upcast(assert_vfs_setup!(Vfs::memfs()));
        test_entry_upcast(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_entry_upcast((vfs, tmpdir): (Vfs, PathBuf))
    {
        let file1 = tmpdir.mash("file1");

        assert_vfs_mkfile!(vfs, &file1);
        assert_eq!(vfs.entry(&file1).unwrap().upcast().path(), &file1);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }
}

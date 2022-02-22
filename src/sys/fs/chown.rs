use std::path::PathBuf;

use crate::errors::RvResult;

/// Provides a builder pattern for flexibly changing file ownership
///
/// Use the Vfs functions `chown_b` to create a new instance followed by one or more options and
/// complete the operation by calling `exec`.
///
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Memfs::new();
/// let file1 = vfs.root().mash("file1");
/// let file2 = vfs.root().mash("file2");
/// //assert_vfs_write_all!(vfs, &file1, "this is a test");
/// //assert!(vfs.copy_b(&file1, &file2).unwrap().exec().is_ok());
/// //assert_eq!(vfs.read_all(&file2).unwrap(), "this is a test");
/// ```
pub struct Chown
{
    pub(crate) opts: ChownOpts,
    pub(crate) exec: Box<dyn Fn(ChownOpts) -> RvResult<()>>, // provider callback
}

// Internal type used to encapsulate just the options. This separates the provider implementation
// from the options allowing for sharing options between different vfs providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChownOpts
{
    pub(crate) path: PathBuf,    // path to chown
    pub(crate) uid: Option<u32>, // uid to use
    pub(crate) gid: Option<u32>, // uid to use
    pub(crate) follow: bool,     // follow links
    pub(crate) recursive: bool,  // chown recursiveily
}

impl Chown
{
    /// Set user id to use for ownership for the given path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file1 = vfs.root().mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert!(vfs.chown_b(&file1).unwrap().uid(5).exec().is_ok());
    /// assert_eq!(vfs.uid(&file1).unwrap(), 5);
    /// assert_eq!(vfs.gid(&file1).unwrap(), 1000);
    /// ```
    pub fn uid(mut self, uid: u32) -> Self
    {
        self.opts.uid = Some(uid);
        self
    }

    /// Set group id to use for ownership for the given path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file1 = vfs.root().mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert!(vfs.chown_b(&file1).unwrap().gid(5).exec().is_ok());
    /// assert_eq!(vfs.uid(&file1).unwrap(), 1000);
    /// assert_eq!(vfs.gid(&file1).unwrap(), 5);
    /// ```
    pub fn gid(mut self, gid: u32) -> Self
    {
        self.opts.gid = Some(gid);
        self
    }

    /// Set user id and group id to use for ownership for the given path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file1 = vfs.root().mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert!(vfs.chown_b(&file1).unwrap().owner(5, 5).exec().is_ok());
    /// assert_eq!(vfs.uid(&file1).unwrap(), 5);
    /// assert_eq!(vfs.gid(&file1).unwrap(), 5);
    /// ```
    pub fn owner(mut self, uid: u32, gid: u32) -> Self
    {
        self.opts.uid = Some(uid);
        self.opts.gid = Some(gid);
        self
    }

    /// Follow links so that the path they point to are also affected
    ///
    /// * Default: false
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
    /// assert_eq!(vfs.uid(&file).unwrap(), 1000);
    /// assert_eq!(vfs.uid(&link).unwrap(), 1000);
    /// assert!(vfs.chown_b(&link).unwrap().uid(5).follow().exec().is_ok());
    /// assert_eq!(vfs.uid(&file).unwrap(), 5);
    /// assert_eq!(vfs.uid(&link).unwrap(), 1000);
    /// ```
    pub fn follow(mut self) -> Self
    {
        self.opts.follow = true;
        self
    }

    /// Follow paths recursively when set to true
    ///
    /// * Default: true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_eq!(vfs.uid(&file).unwrap(), 1000);
    /// assert_eq!(vfs.uid(&dir).unwrap(), 1000);
    /// assert!(vfs.chown_b(&dir).unwrap().uid(5).recurse(false).exec().is_ok());
    /// assert_eq!(vfs.uid(&dir).unwrap(), 5);
    /// assert_eq!(vfs.uid(&file).unwrap(), 1000);
    /// ```
    pub fn recurse(mut self, yes: bool) -> Self
    {
        self.opts.recursive = yes;
        self
    }

    /// Execute the [`Chown`] options against the path provided during construction with the Vfs
    /// `chown_b` functions.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file1 = vfs.root().mash("file1");
    /// assert_vfs_mkfile!(vfs, &file1);
    /// assert!(vfs.chown_b(&file1).unwrap().owner(5, 5).exec().is_ok());
    /// ```
    pub fn exec(&self) -> RvResult<()>
    {
        (self.exec)(self.opts.clone())
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[test]
    fn test_vfs_chown()
    {
        test_chown(assert_vfs_setup!(Vfs::memfs()));
        test_chown(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_chown((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let file1 = tmpdir.mash("file1");
        let dir1file1 = dir1.mash("dir1file1");

        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &dir1file1);
        let (uid, gid) = vfs.owner(&file1).unwrap();

        // chown single file
        assert!(vfs.chown(&file1, uid, gid).is_ok());
        assert_eq!(vfs.uid(&file1).unwrap(), uid);
        assert_eq!(vfs.gid(&file1).unwrap(), gid);

        // recurse
        assert!(vfs.chown(&dir1, uid, gid).is_ok());
        assert_eq!(vfs.owner(&dir1).unwrap(), (uid, gid));
        assert_eq!(vfs.owner(&dir1file1).unwrap(), (uid, gid));

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_chown_follow()
    {
        test_chown_follow(assert_vfs_setup!(Vfs::memfs()));
        test_chown_follow(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_chown_follow((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let dir1file1 = dir1.mash("dir1file1");
        let link1 = tmpdir.mash("link1");

        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &dir1file1);
        assert_vfs_symlink!(vfs, &link1, &dir1);
        let (uid, gid) = vfs.owner(&dir1file1).unwrap();

        // no follow
        assert!(vfs.chown_b(&link1).unwrap().owner(uid, gid).exec().is_ok());
        assert_eq!(vfs.owner(&dir1).unwrap(), (uid, gid));
        assert_eq!(vfs.owner(&dir1file1).unwrap(), (uid, gid));

        // follow
        assert!(vfs.chown_b(&link1).unwrap().owner(uid, gid).exec().is_ok());
        assert_eq!(vfs.owner(&dir1).unwrap(), (uid, gid));
        assert_eq!(vfs.owner(&dir1file1).unwrap(), (uid, gid));

        assert_vfs_remove_all!(vfs, &tmpdir);
    }
}

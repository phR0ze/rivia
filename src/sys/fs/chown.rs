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
    pub(crate) path: PathBuf,   // path to chown
    pub(crate) uid: u32,        // uid to use
    pub(crate) gid: u32,        // uid to use
    pub(crate) follow: bool,    // follow links
    pub(crate) recursive: bool, // chown recursiveily
}

impl Chown
{
    /// Set user id and group id to use for ownership for the given path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir1 = vfs.root().mash("dir1");
    /// let dir1file1 = dir1.mash("dir1file1");
    /// let link1 = vfs.root().mash("link1");
    /// assert_vfs_mkdir_p!(vfs, &dir1);
    /// assert_vfs_mkfile!(vfs, &dir1file1);
    /// assert_vfs_symlink!(vfs, &link1, &dir1);
    /// //let uid = user::getuid();
    /// //let gid = user::getgid();
    /// //assert!(vfs.chown_b(&link1, uid, gid).unwrap().set(uid, gid).exec().is_ok());
    /// //assert_eq!(vfs.uid(&dir1).unwrap(), uid);
    /// //assert_eq!(vfs.gid(&dir1).unwrap(), gid);
    /// //assert_eq!(vfs.uid(&dir1file1).unwrap(), uid);
    /// //assert_eq!(vfs.gid(&dir1file1).unwrap(), gid);
    /// ```
    pub fn set(mut self, uid: u32, gid: u32) -> Self
    {
        self.opts.uid = uid;
        self.opts.gid = gid;
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
    /// let dir1 = vfs.root().mash("dir1");
    /// let dir1file1 = dir1.mash("dir1file1");
    /// let link1 = vfs.root().mash("link1");
    /// //assert_vfs_mkdir_p!(vfs, &dir1);
    /// //assert_vfs_mkfile!(vfs, &dir1file1);
    /// //assert_vfs_symlink!(vfs, &link1, &dir1);
    /// //let uid = user::getuid();
    /// //let gid = user::getgid();
    /// //assert!(vfs.chown_b(&link1, uid, gid).unwrap().exec().is_ok());
    /// //assert_eq!(vfs.uid(&dir1).unwrap(), uid);
    /// //assert_eq!(vfs.gid(&dir1).unwrap(), gid);
    /// //assert_eq!(vfs.uid(&dir1file1).unwrap(), uid);
    /// //assert_eq!(vfs.gid(&dir1file1).unwrap(), gid);
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
    /// let dir1 = vfs.root().mash("dir1");
    /// let dir1file1 = dir1.mash("dir1file1");
    /// let link1 = vfs.root().mash("link1");
    /// //assert_vfs_mkdir_p!(vfs, &dir1);
    /// //assert_vfs_mkfile!(vfs, &dir1file1);
    /// //assert_vfs_symlink!(vfs, &link1, &dir1);
    /// //let uid = user::getuid();
    /// //let gid = user::getgid();
    /// //assert!(vfs.chown_b(&link1, uid, gid).unwrap().exec().is_ok());
    /// //assert_eq!(vfs.uid(&dir1).unwrap(), uid);
    /// //assert_eq!(vfs.gid(&dir1).unwrap(), gid);
    /// //assert_eq!(vfs.uid(&dir1file1).unwrap(), uid);
    /// //assert_eq!(vfs.gid(&dir1file1).unwrap(), gid);
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
    /// let dir1 = vfs.root().mash("dir1");
    /// let dir1file1 = dir1.mash("dir1file1");
    /// let link1 = vfs.root().mash("link1");
    /// //assert_vfs_mkdir_p!(vfs, &dir1);
    /// //assert_vfs_mkfile!(vfs, &dir1file1);
    /// //assert_vfs_symlink!(vfs, &link1, &dir1);
    /// //let uid = user::getuid();
    /// //let gid = user::getgid();
    /// //assert!(vfs.chown_b(&link1, uid, gid).unwrap().exec().is_ok());
    /// //assert_eq!(vfs.uid(&dir1).unwrap(), uid);
    /// //assert_eq!(vfs.gid(&dir1).unwrap(), gid);
    /// //assert_eq!(vfs.uid(&dir1file1).unwrap(), uid);
    /// //assert_eq!(vfs.gid(&dir1file1).unwrap(), gid);
    /// ```
    pub fn exec(&self) -> RvResult<()>
    {
        (self.exec)(self.opts.clone())
    }
}

// // Unit tests
// // -------------------------------------------------------------------------------------------------
// #[cfg(test)]
// mod tests
// {
//     use crate::prelude::*;
//     assert_vfs_setup_func!();

//     #[test]
//     fn test_vfs_vfs_chown()
//     {
//         let vfs.root() = assert_vfs_setup!();
//         let dir1 = vfs.root().mash("dir1");
//         let file1 = vfs.root().mash("file1");
//         let dir1file1 = dir1.mash("dir1file1");

//         assert_eq!(vfs::mkfile(&file1).unwrap(), file1);
//         assert_eq!(vfs::mkdir(&dir1).unwrap(), dir1);
//         assert_eq!(vfs::mkfile(&dir1file1).unwrap(), dir1file1);
//         let uid = user::getuid();
//         let gid = user::getgid();

//         // chown single file
//         assert_eq!(vfs::uid(&file1).unwrap(), uid);
//         assert_eq!(vfs::gid(&file1).unwrap(), gid);
//         assert!(vfs::chown(&file1, uid, gid).is_ok());
//         assert_eq!(vfs::uid(&file1).unwrap(), uid);
//         assert_eq!(vfs::gid(&file1).unwrap(), gid);

//         // recurse
//         assert!(vfs::chown(&dir1, uid, gid).is_ok());
//         assert_eq!(vfs::uid(&dir1).unwrap(), uid);
//         assert_eq!(vfs::gid(&dir1).unwrap(), gid);
//         assert_eq!(vfs::uid(&dir1file1).unwrap(), uid);
//         assert_eq!(vfs::gid(&dir1file1).unwrap(), gid);
//     }

//     #[test]
//     fn test_vfs_vfs_chown_follow()
//     {
//         let vfs.root() = assert_vfs_setup!();
//         let dir1 = vfs.root().mash("dir1");
//         let dir1file1 = dir1.mash("dir1file1");
//         let link1 = vfs.root().mash("link1");

//         assert_eq!(vfs::mkdir(&dir1).unwrap(), dir1);
//         assert_eq!(vfs::mkfile(&dir1file1).unwrap(), dir1file1);
//         assert_eq!(vfs::symlink(&dir1, &link1).unwrap(), link1);

//         let uid = user::getuid();
//         let gid = user::getgid();

//         // no follow
//         assert!(vfs::chown_b(&link1, uid, gid).unwrap().exec().is_ok());
//         assert_eq!(vfs::uid(&dir1).unwrap(), uid);
//         assert_eq!(vfs::gid(&dir1).unwrap(), gid);
//         assert_eq!(vfs::uid(&dir1file1).unwrap(), uid);
//         assert_eq!(vfs::gid(&dir1file1).unwrap(), gid);

//         // follow
//         assert!(vfs::chown_b(&link1, uid, gid).unwrap().exec().is_ok());
//         assert_eq!(vfs::uid(&dir1).unwrap(), uid);
//         assert_eq!(vfs::gid(&dir1).unwrap(), gid);
//         assert_eq!(vfs::uid(&dir1file1).unwrap(), uid);
//         assert_eq!(vfs::gid(&dir1file1).unwrap(), gid);
//     }
// }

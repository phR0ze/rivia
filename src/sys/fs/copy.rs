use std::path::PathBuf;

use crate::errors::RvResult;

/// Provides a builder pattern for flexibly copying files
///
/// Use the Vfs functions `copy_b` to create a new instance followed by one or more options and
/// complete the operation by calling `exec`.
///
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Memfs::new();
/// let file1 = vfs.root().mash("file1");
/// let file2 = vfs.root().mash("file2");
/// assert_vfs_write_all!(vfs, &file1, "this is a test");
/// assert!(vfs.copy_b(&file1, &file2).unwrap().exec().is_ok());
/// assert_eq!(vfs.read_all(&file2).unwrap(), "this is a test");
/// ```
pub struct Copier
{
    pub(crate) opts: CopyOpts,
    pub(crate) exec: Box<dyn Fn(CopyOpts) -> RvResult<()>>, // provider callback
}

// Internal type used to encapsulate just the options. This separates the provider implementation
// from the options allowing for sharing options between different vfs providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopyOpts
{
    pub(crate) src: PathBuf,      // source file
    pub(crate) dst: PathBuf,      // destination path
    pub(crate) mode: Option<u32>, // mode to use
    pub(crate) cdirs: bool,       // chmod only dirs when true
    pub(crate) cfiles: bool,      // chmod only files when true
    pub(crate) follow: bool,      // follow links when copying files
}

impl Copier
{
    /// Apply chmod to all files and directories
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file1 = vfs.root().mash("file1");
    /// let file2 = vfs.root().mash("file2");
    /// let dir1 = vfs.root().mash("dir1");
    /// let dir2 = vfs.root().mash("dir2");
    /// assert!(vfs.mkfile_m(&file1, 0o600).is_ok());
    /// assert!(vfs.mkdir_m(&dir1, 0o777).is_ok());
    /// assert!(vfs.copy_b(&file1, &file2).unwrap().chmod_all(0o655).exec().is_ok());
    /// assert_eq!(vfs.mode(&file2).unwrap(), 0o100655);
    /// assert!(vfs.copy_b(&dir1, &dir2).unwrap().chmod_all(0o755).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir2).unwrap(), 0o40755);
    /// ```
    pub fn chmod_all(mut self, mode: u32) -> Self
    {
        self.opts.cdirs = false;
        self.opts.cfiles = false;
        self.opts.mode = Some(mode);
        self
    }

    /// Apply chmod to all directories only
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file1 = vfs.root().mash("file1");
    /// let file2 = vfs.root().mash("file2");
    /// let dir1 = vfs.root().mash("dir1");
    /// let dir2 = vfs.root().mash("dir2");
    /// assert!(vfs.mkfile_m(&file1, 0o600).is_ok());
    /// assert!(vfs.mkdir_m(&dir1, 0o777).is_ok());
    /// assert_eq!(vfs.mode(&dir1).unwrap(), 0o40777);
    /// assert!(vfs.copy_b(&file1, &file2).unwrap().chmod_dirs(0o655).exec().is_ok());
    /// assert_eq!(vfs.mode(&file2).unwrap(), 0o100600);
    /// assert!(vfs.copy_b(&dir1, &dir2).unwrap().chmod_dirs(0o755).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir2).unwrap(), 0o40755);
    /// ```
    pub fn chmod_dirs(mut self, mode: u32) -> Self
    {
        self.opts.cdirs = true;
        self.opts.cfiles = false;
        self.opts.mode = Some(mode);
        self
    }

    /// Apply chmod to only files
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file1 = vfs.root().mash("file1");
    /// let file2 = vfs.root().mash("file2");
    /// let dir1 = vfs.root().mash("dir1");
    /// let dir2 = vfs.root().mash("dir2");
    /// assert!(vfs.mkfile_m(&file1, 0o600).is_ok());
    /// assert!(vfs.mkdir_m(&dir1, 0o777).is_ok());
    /// assert_eq!(vfs.mode(&dir1).unwrap(), 0o40777);
    /// assert!(vfs.copy_b(&file1, &file2).unwrap().chmod_files(0o655).exec().is_ok());
    /// assert_eq!(vfs.mode(&file2).unwrap(), 0o100655);
    /// assert!(vfs.copy_b(&dir1, &dir2).unwrap().chmod_files(0o755).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir2).unwrap(), 0o40777);
    /// ```
    pub fn chmod_files(mut self, mode: u32) -> Self
    {
        self.opts.cdirs = false;
        self.opts.cfiles = true;
        self.opts.mode = Some(mode);
        self
    }

    /// Update the `follow` option
    ///
    /// * Default: false
    /// * When `true` links are followed i.e. the file pointed to will be copied not the link
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file1 = vfs.root().mash("file1");
    /// let link1 = vfs.root().mash("link1");
    /// let file2 = vfs.root().mash("file2");
    /// assert_vfs_write_all!(vfs, &file1, "file1");
    /// assert_vfs_symlink!(vfs, &link1, &file1);
    /// assert!(vfs.copy_b(&link1, &file2).unwrap().follow(true).exec().is_ok());
    /// assert_vfs_no_symlink!(vfs, &file2);
    /// assert_vfs_read_all!(vfs, &file2, "file1");
    /// ```
    pub fn follow(mut self, yes: bool) -> Self
    {
        self.opts.follow = yes;
        self
    }

    /// Execute the [`Copier`] builder current options.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file1 = vfs.root().mash("file1");
    /// let file2 = vfs.root().mash("file2");
    /// assert_vfs_write_all!(vfs, &file1, "this is a test");
    /// assert!(vfs.copy_b(&file1, &file2).unwrap().exec().is_ok());
    /// assert_vfs_read_all!(vfs, &file2, "this is a test");
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
    fn test_vfs_copy_errors()
    {
        test_copy_errors(assert_vfs_setup!(Vfs::memfs()));
        test_copy_errors(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_copy_errors((vfs, tmpdir): (Vfs, PathBuf))
    {
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");

        // source same as destination
        assert!(vfs.copy(&file1, &file1).is_ok());
        assert_vfs_no_exists!(vfs, &file1);

        // source empty
        assert_eq!(vfs.copy("", &file1).unwrap_err().downcast_ref::<PathError>(), Some(&PathError::Empty));
        assert_vfs_no_exists!(vfs, &file1);

        // destination empty
        assert_eq!(vfs.copy(&file1, "").unwrap_err().downcast_ref::<PathError>(), Some(&PathError::Empty));
        assert_vfs_no_exists!(vfs, &file1);

        // source doesn't exist
        assert_eq!(
            vfs.copy(&file1, &file2).unwrap_err().downcast_ref::<PathError>(),
            Some(&PathError::does_not_exist(&file1))
        );
        assert_vfs_no_exists!(vfs, &file2);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_copy_file()
    {
        test_copy_file(assert_vfs_setup!(Vfs::memfs()));
        test_copy_file(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_copy_file((vfs, tmpdir): (Vfs, PathBuf))
    {
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");
        let link1 = tmpdir.mash("link1");
        let link2 = tmpdir.mash("link2");
        let dir1 = tmpdir.mash("dir1");
        let dir1file3 = dir1.mash("file3");
        let dir2 = tmpdir.mash("dir2");
        let dir2file1 = dir2.mash("file1");

        // file copy i.e. copy with diff name
        assert!(vfs.mkfile_m(&file1, 0o600).is_ok());
        assert!(vfs.copy(&file1, &file2).is_ok());
        assert_eq!(vfs.mode(&file2).unwrap(), 0o100600);
        assert_iter_eq(vfs.all_paths(&tmpdir).unwrap(), vec![file1.clone(), file2.clone()]);

        // file copy, i.e. copy with diff name, to dir that doesn't exist
        assert_vfs_no_exists!(vfs, &dir1);
        assert_vfs_no_exists!(vfs, &dir1file3);
        assert!(vfs.copy(&file1, &dir1file3).is_ok());
        assert_eq!(vfs.mode(&dir1file3).unwrap(), 0o100600);
        assert_vfs_exists!(vfs, &dir1file3);
        assert_iter_eq(vfs.all_paths(&tmpdir).unwrap(), vec![
            dir1.clone(),
            dir1file3.clone(),
            file1.clone(),
            file2.clone(),
        ]);

        // link copy, i.e. copy with diff name
        assert_vfs_symlink!(vfs, &link1, &file1);
        assert_vfs_no_exists!(vfs, &link2);
        assert!(vfs.copy(&link1, &link2).is_ok());
        assert_vfs_exists!(vfs, &link2);
        assert_iter_eq(vfs.all_paths(&tmpdir).unwrap(), vec![
            dir1.clone(),
            dir1file3.clone(),
            file1.clone(),
            file2.clone(),
            link1.clone(),
            link2.clone(),
        ]);

        // file clone, i.e. keep original name, to dir that doesn't exist
        assert_vfs_no_exists!(vfs, &dir2file1);
        assert!(vfs.copy(&file1, &dir2file1).is_ok());
        assert_eq!(vfs.mode(&dir2file1).unwrap(), 0o100600);
        assert_vfs_exists!(vfs, &dir2file1);
        assert_iter_eq(vfs.all_paths(&tmpdir).unwrap(), vec![
            dir1.clone(),
            dir1file3.clone(),
            dir2.clone(),
            dir2file1.clone(),
            file1.clone(),
            file2.clone(),
            link1.clone(),
            link2.clone(),
        ]);

        // file clone, i.e. keep original name, to dir that already exist
        assert_vfs_remove_all!(vfs, &dir2);
        assert_vfs_mkdir_p!(vfs, &dir2);
        assert_vfs_no_exists!(vfs, &dir2file1);
        assert!(vfs.copy(&file1, &dir2).is_ok());
        assert_eq!(vfs.mode(&dir2file1).unwrap(), 0o100600);
        assert_iter_eq(vfs.all_paths(&tmpdir).unwrap(), vec![
            dir1.clone(),
            dir1file3.clone(),
            dir2.clone(),
            dir2file1.clone(),
            file1.clone(),
            file2.clone(),
            link1.clone(),
            link2.clone(),
        ]);

        // link clone, i.e. keep original name
        let link4 = dir2.mash("link1");
        assert_vfs_no_exists!(vfs, &link4);
        assert!(vfs.copy(&link1, &dir2).is_ok());
        assert_eq!(vfs.readlink(&link4).unwrap(), PathBuf::from("..").mash("file1"));
        assert_vfs_exists!(vfs, &link4);
        assert_iter_eq(vfs.all_paths(&tmpdir).unwrap(), vec![
            dir1.clone(),
            dir1file3.clone(),
            dir2.clone(),
            dir2file1.clone(),
            link4.clone(),
            file1.clone(),
            file2.clone(),
            link1.clone(),
            link2.clone(),
        ]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_copy_chmod()
    {
        test_copy_chmod(assert_vfs_setup!(Vfs::memfs()));
        test_copy_chmod(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_copy_chmod((vfs, tmpdir): (Vfs, PathBuf))
    {
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");
        let file3 = tmpdir.mash("file3");
        let file4 = tmpdir.mash("file4");
        let dir1 = tmpdir.mash("dir1");
        let dir2 = tmpdir.mash("dir2");
        let dir3 = tmpdir.mash("dir3");
        let dir4 = tmpdir.mash("dir4");

        // Set file mode but not dir mode
        assert!(vfs.mkfile_m(&file1, 0o600).is_ok());
        assert!(vfs.mkdir_m(&dir1, 0o777).is_ok());
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40777);

        assert!(vfs.copy_b(&file1, &file2).unwrap().chmod_files(0o655).exec().is_ok());
        assert_eq!(vfs.mode(&file2).unwrap(), 0o100655);

        assert!(vfs.copy_b(&dir1, &dir2).unwrap().chmod_files(0o755).exec().is_ok());
        assert_eq!(vfs.mode(&dir2).unwrap(), 0o40777);

        // Set dir mode but not file mode
        assert!(vfs.copy_b(&file1, &file3).unwrap().chmod_dirs(0o655).exec().is_ok());
        assert_eq!(vfs.mode(&file3).unwrap(), 0o100600);

        assert!(vfs.copy_b(&dir1, &dir3).unwrap().chmod_dirs(0o755).exec().is_ok());
        assert_eq!(vfs.mode(&dir3).unwrap(), 0o40755);

        // Set dir and file mode
        assert!(vfs.copy_b(&file1, &file4).unwrap().chmod_all(0o655).exec().is_ok());
        assert_eq!(vfs.mode(&file4).unwrap(), 0o100655);

        assert!(vfs.copy_b(&dir1, &dir4).unwrap().chmod_all(0o755).exec().is_ok());
        assert_eq!(vfs.mode(&dir4).unwrap(), 0o40755);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_copy_dir()
    {
        test_copy_dir(assert_vfs_setup!(Vfs::memfs()));
        test_copy_dir(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_copy_dir((vfs, tmpdir): (Vfs, PathBuf))
    {
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = tmpdir.mash("dir2");
        let file2 = dir2.mash("file1");
        let dir3 = dir2.mash("dir1");
        let file3 = dir3.mash("file1");
        let link1 = tmpdir.mash("link1");
        let link2 = tmpdir.mash("link2");

        // clone i.e. copy with diff name
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);
        assert!(vfs.copy(&dir1, &dir2).is_ok());
        assert_iter_eq(vfs.all_paths(&tmpdir).unwrap(), vec![
            dir1.clone(),
            file1.clone(),
            dir2.clone(),
            file2.clone(),
        ]);

        // clone i.e. copy to different location same name
        assert!(vfs.copy(&dir1, &dir2).is_ok());
        assert_iter_eq(vfs.all_paths(&tmpdir).unwrap(), vec![
            dir1.clone(),
            file1.clone(),
            dir2.clone(),
            dir3.clone(),
            file3.clone(),
            file2.clone(),
        ]);

        // copy dir symlink
        assert_vfs_symlink!(vfs, &link1, &dir1);
        assert!(vfs.copy(&link1, &link2).is_ok());
        assert_vfs_readlink_abs!(vfs, &link1, &dir1);
        assert_vfs_readlink_abs!(vfs, &link2, &dir1);

        // clone link1 into dir2
        let link3 = dir2.mash("link1");
        assert!(vfs.copy(&link1, &dir2).is_ok());
        assert_vfs_readlink!(vfs, &link3, PathBuf::from("../dir1"));
        assert_iter_eq(vfs.all_paths(&tmpdir).unwrap(), vec![
            dir1.clone(),
            file1.clone(),
            dir2.clone(),
            dir3.clone(),
            file3.clone(),
            file2.clone(),
            link3.clone(),
            link1.clone(),
            link2.clone(),
        ]);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_copy_follow()
    {
        test_copy_follow(assert_vfs_setup!(Vfs::memfs()));
        test_copy_follow(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_copy_follow((vfs, tmpdir): (Vfs, PathBuf))
    {
        // don't follow file link - copy
        let file1 = tmpdir.mash("file1");
        let link1 = tmpdir.mash("link1");
        let link2 = tmpdir.mash("link2");
        assert_vfs_write_all!(vfs, &file1, "file1");
        assert_vfs_symlink!(vfs, &link1, &file1);
        assert!(vfs.copy_b(&link1, &link2).unwrap().exec().is_ok());
        assert_vfs_readlink_abs!(vfs, &link1, &file1);
        assert_vfs_readlink!(vfs, &link1, PathBuf::from("file1"));
        assert_eq!(vfs.readlink(&link1).unwrap(), vfs.readlink(&link2).unwrap());
        assert_eq!(vfs.readlink_abs(&link1).unwrap(), vfs.readlink_abs(&link2).unwrap());

        // follow file link - copy
        let file2 = tmpdir.mash("file2");
        assert!(vfs.copy_b(&link1, &file2).unwrap().follow(true).exec().is_ok());
        assert_vfs_no_symlink!(vfs, &file2);
        assert_vfs_is_file!(vfs, &file2);
        assert_vfs_read_all!(vfs, &file2, "file1");

        // don't follow dir link - copy
        let dir1 = tmpdir.mash("dir1");
        let dir1file = dir1.mash("dir1file");
        let dir1link1 = tmpdir.mash("dir1link1");
        let dir1link2 = tmpdir.mash("dir1link2");
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_write_all!(vfs, &dir1file, "dir1file");
        assert_vfs_symlink!(vfs, &dir1link1, &dir1);
        assert!(vfs.copy_b(&dir1link1, &dir1link2).unwrap().exec().is_ok());
        assert_vfs_readlink_abs!(vfs, &dir1link1, &dir1);

        // follow dir link - copy
        let dir2 = tmpdir.mash("dir2");
        let dir2file = dir2.mash("dir1file");
        assert!(vfs.copy_b(&dir1link1, &dir2).unwrap().follow(true).exec().is_ok());
        assert_vfs_read_all!(vfs, &dir2file, "dir1file");
        assert_vfs_no_symlink!(vfs, &dir2file);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }
}

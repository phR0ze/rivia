use std::path::PathBuf;

use crate::{
    errors::{RvResult, VfsError},
    sys::{Entry, VfsEntry},
};

/// Provides a builder pattern for flexibly copying files
///
/// Use the Vfs functions `copy_b` to create a new instance followed by one or more options and
/// complete the operation by calling `exec`.
///
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Memfs::new();
/// let file = vfs.root().mash("file");
/// assert_vfs_mkfile!(vfs, &file);
/// assert_eq!(vfs.is_exec(&file), false);
/// assert!(vfs.chmod_b(&file).unwrap().sym("f:a+x").exec().is_ok());
/// assert_eq!(vfs.is_exec(&file), true);
/// ```
pub struct Copy
{
    pub(crate) cp: CopyInner,
    pub(crate) exec: Box<dyn Fn(CopyInner) -> RvResult<()>>, // vfs backend to use
}

// Internal clonable type used to encapsulate just the values
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopyInner
{
    pub(crate) src: PathBuf,      // source file
    pub(crate) dst: PathBuf,      // destination path
    pub(crate) mode: Option<u32>, // mode to use
    pub(crate) cdirs: bool,       // chmod only dirs when true
    pub(crate) cfiles: bool,      // chmod only files when true
    pub(crate) follow: bool,      // follow links when copying files
}

impl Copy
{
    /// Apply chmod to all files and directories.
    ///
    /// Default: false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// //assert!(sys::mkfile_m(&file1, 0o600).is_ok());
    /// //assert!(sys::mkdir_m(&dir1, 0o777).is_ok());
    /// //assert!(sys::copy_b(&file1, &file2).unwrap().chmod_all(0o655).exec().is_ok());
    /// //assert_eq!(vfs.mode(&file2).unwrap(), 0o100655);
    /// //assert!(vfs.copy_b(&dir1, &dir2).unwrap().chmod_all(0o755).exec().is_ok());
    /// //assert_eq!(vfs.mode(&dir2).unwrap(), 0o40755);
    /// ```
    pub fn chmod_all(mut self, mode: u32) -> Self
    {
        self.cp.cdirs = false;
        self.cp.cfiles = false;
        self.cp.mode = Some(mode);
        self
    }

    /// Apply chmod to only directories.
    ///
    /// Default: false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// //assert!(vfs.mkfile_m(&file1, 0o600).is_ok());
    /// //assert!(vfs.mkdir_m(&dir1, 0o777).is_ok());
    /// //assert_eq!(vfs.mode(&dir1).unwrap(), 0o40777);
    /// //assert!(vfs.copy_b(&file1, &file2).unwrap().chmod_dirs(0o655).exec().is_ok());
    /// //assert_eq!(vfs.mode(&file2).unwrap(), 0o100600);
    /// //assert!(vfs.copy_b(&dir1, &dir2).unwrap().chmod_dirs(0o755).exec().is_ok());
    /// //assert_eq!(vfs.mode(&dir2).unwrap(), 0o40755);
    /// ```
    pub fn chmod_dirs(mut self, mode: u32) -> Self
    {
        self.cp.cdirs = true;
        self.cp.cfiles = false;
        self.cp.mode = Some(mode);
        self
    }

    /// Apply chmod to only files.
    ///
    /// Default: false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// let dir1 = tmpdir.mash("dir1");
    /// let dir2 = tmpdir.mash("dir2");
    /// //assert!(vfs.mkfile_m(&file1, 0o600).is_ok());
    /// //assert!(vfs.mkdir_m(&dir1, 0o777).is_ok());
    /// //assert_eq!(vfs.mode(&dir1).unwrap(), 0o40777);
    /// //assert!(vfs.copy_b(&file1, &file2).unwrap().chmod_files(0o655).exec().is_ok());
    /// //assert_eq!(vfs.mode(&file2).unwrap(), 0o100655);
    /// //assert!(vfs.copy_b(&dir1, &dir2).unwrap().chmod_files(0o755).exec().is_ok());
    /// //assert_eq!(vfs.mode(&dir2).unwrap(), 0o40777);
    /// ```
    pub fn chmod_files(mut self, mode: u32) -> Self
    {
        self.cp.cdirs = false;
        self.cp.cfiles = true;
        self.cp.mode = Some(mode);
        self
    }

    /// Update the `follow` option. When `yes` is `true`, links are followed i.e. the file pointed
    /// to will be copied not the link.
    ///
    /// Default: false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let file1 = tmpdir.mash("file1");
    /// let link1 = tmpdir.mash("link1");
    /// let file2 = tmpdir.mash("file2");
    /// //assert_write!(&file1, "file1");
    /// //assert_eq!(vfs.symlink(&file1, &link1).unwrap(), link1);
    /// //assert!(vfs.copy_b(&link1, &file2).unwrap().follow(true).exec().is_ok());
    /// //assert!(vfs.readlink(&file2).is_err());
    /// //assert_eq!(vfs.read(&file2).unwrap(), "file1");
    /// ```
    pub fn follow(mut self, yes: bool) -> Self
    {
        self.cp.follow = yes;
        self
    }

    /// Execute the [`Copy`] builder current options.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let tmpdir = vfs.root().mash("tmpdir");
    /// let file1 = tmpdir.mash("file1");
    /// let file2 = tmpdir.mash("file2");
    /// //assert_write!(&file1, "this is a test");
    /// //assert!(vfs.copy_b(&file1, &file2).unwrap().exec().is_ok());
    /// //assert_eq!(vfs.read(&file2).unwrap(), "this is a test");
    /// ```
    pub fn exec(&self) -> RvResult<()>
    {
        (self.exec)(self.cp.clone())
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    // #[test]
    // fn test_vfs_stdfs_copy_error_cases()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let file1 = tmpdir.mash("file1");
    //     let file2 = tmpdir.mash("file2");

    //     // Direct calls
    //     {
    //         // source same as destination
    //         assert!(Stdfs::copy(&file1, &file1).is_ok());
    //         assert_eq!(Stdfs::exists(&file1), false);

    //         // source empty
    //         assert_eq!(Stdfs::copy("", &file1).unwrap_err().to_string().as_str(), "path empty");
    //         assert_eq!(Stdfs::exists(&file1), false);

    //         // destination empty
    //         assert_eq!(Stdfs::copy(&file1, "").unwrap_err().to_string().as_str(), "path empty");
    //         assert_eq!(Stdfs::exists(&file1), false);

    //         // source doesn't exist
    //         assert_eq!(
    //             Stdfs::copy(&file1, &file2).unwrap_err().to_string().as_str(),
    //             format!("path does not exist: {}", &file1.to_string().unwrap())
    //         );
    //         assert_eq!(Stdfs::exists(&file2), false);
    //     }

    //     // Method calls
    //     {
    //         let stdfs = Stdfs::new();

    //         // source same as destination
    //         assert!(stdfs.copy(&file1, &file1).is_ok());
    //         assert_eq!(stdfs.exists(&file1), false);

    //         // source empty
    //         assert_eq!(stdfs.copy(&PathBuf::new(), &file1).unwrap_err().to_string().as_str(),
    // "path empty");         assert_eq!(stdfs.exists(&file1), false);

    //         // destination empty
    //         assert_eq!(stdfs.copy(&file1, &PathBuf::new()).unwrap_err().to_string().as_str(),
    // "path empty");         assert_eq!(stdfs.exists(&file1), false);

    //         // source doesn't exist
    //         assert_eq!(
    //             stdfs.copy(&file1, &file2).unwrap_err().to_string().as_str(),
    //             format!("path does not exist: {}", &file1.to_string().unwrap())
    //         );
    //         assert_eq!(stdfs.exists(&file2), false);
    //     }

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_vfs_stdfs_copy_file()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let file1 = tmpdir.mash("file1");
    //     let file2 = tmpdir.mash("file2");
    //     let link1 = tmpdir.mash("link1");
    //     let link2 = tmpdir.mash("link2");
    //     let dir1 = tmpdir.mash("dir1");
    //     let file3 = dir1.mash("file3");
    //     let dir2 = tmpdir.mash("dir2");
    //     let file4 = dir2.mash("file1");

    //     // file copy i.e. copy with diff name
    //     assert!(Stdfs::mkfile_m(&file1, 0o600).is_ok());
    //     assert_eq!(Stdfs::exists(&file1), true);
    //     assert_eq!(Stdfs::exists(&file2), false);
    //     assert!(Stdfs::copy(&file1, &file2).is_ok());
    //     assert_eq!(Stdfs::mode(&file2).unwrap(), 0o100600);
    //     assert_iter_eq(Stdfs::all_paths(&tmpdir).unwrap(), vec![file1.clone(), file2.clone()]);

    //     // file copy, i.e. copy with diff name, to dir that doesn't exist
    //     assert_eq!(Stdfs::exists(&file3), false);
    //     assert!(Stdfs::copy(&file1, &file3).is_ok());
    //     assert_eq!(Stdfs::mode(&file3).unwrap(), 0o100600);
    //     assert_eq!(Stdfs::exists(&file3), true);
    //     assert_iter_eq(Stdfs::all_paths(&tmpdir).unwrap(), vec![
    //         dir1.clone(),
    //         file3.clone(),
    //         file1.clone(),
    //         file2.clone(),
    //     ]);

    //     // link copy, i.e. copy with diff name
    //     assert!(Stdfs::symlink(&file1, &link1).is_ok());
    //     assert_eq!(Stdfs::exists(&link2), false);
    //     assert!(Stdfs::copy(&link1, &link2).is_ok());
    //     assert_eq!(Stdfs::exists(&link2), true);
    //     assert_iter_eq(Stdfs::all_paths(&tmpdir).unwrap(), vec![
    //         dir1.clone(),
    //         file3.clone(),
    //         file1.clone(),
    //         file2.clone(),
    //         link1.clone(),
    //         link2.clone(),
    //     ]);

    //     // file clone, i.e. keep original name, to dir that doesn't exist
    //     assert_eq!(Stdfs::exists(&file4), false);
    //     assert!(Stdfs::copy(&file1, &file4).is_ok());
    //     assert_eq!(Stdfs::mode(&file4).unwrap(), 0o100600);
    //     assert_eq!(Stdfs::exists(&file4), true);
    //     assert_iter_eq(Stdfs::all_paths(&tmpdir).unwrap(), vec![
    //         dir1.clone(),
    //         file3.clone(),
    //         dir2.clone(),
    //         file4.clone(),
    //         file1.clone(),
    //         file2.clone(),
    //         link1.clone(),
    //         link2.clone(),
    //     ]);

    //     // file clone, i.e. keep original name, to dir that already exist
    //     assert!(Stdfs::remove_all(&dir2).is_ok());
    //     assert!(Stdfs::mkdir(&dir2).is_ok());
    //     assert_eq!(Stdfs::exists(&file4), false);
    //     assert!(Stdfs::copy(&file1, &dir2).is_ok());
    //     assert_eq!(Stdfs::mode(&file4).unwrap(), 0o100600);
    //     assert_iter_eq(Stdfs::all_paths(&tmpdir).unwrap(), vec![
    //         dir1.clone(),
    //         file3.clone(),
    //         dir2.clone(),
    //         file4.clone(),
    //         file1.clone(),
    //         file2.clone(),
    //         link1.clone(),
    //         link2.clone(),
    //     ]);

    //     // link clone, i.e. keep original name
    //     let link4 = dir2.mash("link1");
    //     assert_eq!(Stdfs::exists(&link4), false);
    //     assert!(Stdfs::copy(&link1, &dir2).is_ok());
    //     assert_eq!(Stdfs::readlink(&link4).unwrap(), PathBuf::from("../file1"));
    //     assert_eq!(Stdfs::exists(&link4), true);
    //     assert_iter_eq(Stdfs::all_paths(&tmpdir).unwrap(), vec![
    //         dir1.clone(),
    //         file3.clone(),
    //         dir2.clone(),
    //         file4.clone(),
    //         link4.clone(),
    //         file1.clone(),
    //         file2.clone(),
    //         link1.clone(),
    //         link2.clone(),
    //     ]);

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_vfs_stdfs_copy_chmod()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let file1 = tmpdir.mash("file1");
    //     let file2 = tmpdir.mash("file2");
    //     let file3 = tmpdir.mash("file3");
    //     let file4 = tmpdir.mash("file4");
    //     let dir1 = tmpdir.mash("dir1");
    //     let dir2 = tmpdir.mash("dir2");
    //     let dir3 = tmpdir.mash("dir3");
    //     let dir4 = tmpdir.mash("dir4");

    //     // Set file mode but not dir mode
    //     assert!(Stdfs::mkfile_m(&file1, 0o600).is_ok());
    //     assert!(Stdfs::mkdir_m(&dir1, 0o777).is_ok());
    //     assert_eq!(Stdfs::mode(&dir1).unwrap(), 0o40777);

    //     assert!(Stdfs::copy_b(&file1, &file2).chmod_files(0o655).exec().is_ok());
    //     assert_eq!(Stdfs::mode(&file2).unwrap(), 0o100655);

    //     assert!(Stdfs::copy_b(&dir1, &dir2).chmod_files(0o755).exec().is_ok());
    //     assert_eq!(Stdfs::mode(&dir2).unwrap(), 0o40777);

    //     // Set dir mode but not file mode
    //     assert!(Stdfs::copy_b(&file1, &file3).chmod_dirs(0o655).exec().is_ok());
    //     assert_eq!(Stdfs::mode(&file3).unwrap(), 0o100600);

    //     assert!(Stdfs::copy_b(&dir1, &dir3).chmod_dirs(0o755).exec().is_ok());
    //     assert_eq!(Stdfs::mode(&dir3).unwrap(), 0o40755);

    //     // Set dir and file mode
    //     assert!(Stdfs::copy_b(&file1, &file4).chmod_all(0o655).exec().is_ok());
    //     assert_eq!(Stdfs::mode(&file4).unwrap(), 0o100655);

    //     assert!(Stdfs::copy_b(&dir1, &dir4).chmod_all(0o755).exec().is_ok());
    //     assert_eq!(Stdfs::mode(&dir4).unwrap(), 0o40755);

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_vfs_stdfs_copy_dir()
    // {
    //     let tmpdir = assert_stdfs_setup!();
    //     let dir1 = tmpdir.mash("dir1");
    //     let file1 = dir1.mash("file1");
    //     let dir2 = tmpdir.mash("dir2");
    //     let file2 = dir2.mash("file1");
    //     let dir3 = dir2.mash("dir1");
    //     let file3 = dir3.mash("file1");
    //     let link1 = tmpdir.mash("link1");
    //     let link2 = tmpdir.mash("link2");

    //     // clone i.e. copy with diff name
    //     assert!(Stdfs::mkdir(&dir1).is_ok());
    //     assert_stdfs_mkfile!(&file1);
    //     assert!(Stdfs::copy(&dir1, &dir2).is_ok());
    //     assert_iter_eq(Stdfs::all_paths(&tmpdir).unwrap(), vec![
    //         dir1.clone(),
    //         file1.clone(),
    //         dir2.clone(),
    //         file2.clone(),
    //     ]);

    //     // clone i.e. copy to different location same name
    //     assert!(Stdfs::copy(&dir1, &dir2).is_ok());
    //     assert_iter_eq(Stdfs::all_paths(&tmpdir).unwrap(), vec![
    //         dir1.clone(),
    //         file1.clone(),
    //         dir2.clone(),
    //         dir3.clone(),
    //         file3.clone(),
    //         file2.clone(),
    //     ]);

    //     // copy symnlink dir
    //     assert_eq!(Stdfs::symlink(&dir1, &link1).unwrap(), link1);
    //     assert_eq!(Stdfs::readlink(&link1).unwrap(), PathBuf::from("dir1"));
    //     assert!(Stdfs::copy(&link1, &link2).is_ok());
    //     assert_eq!(Stdfs::readlink(&link1).unwrap(), PathBuf::from("dir1"));

    //     // clone link1 into dir2
    //     let link3 = dir2.mash("link1");
    //     assert!(Stdfs::copy(&link1, &dir2).is_ok());
    //     assert_eq!(Stdfs::readlink(&link3).unwrap(), PathBuf::from("../dir1"));
    //     assert_iter_eq(Stdfs::all_paths(&tmpdir).unwrap(), vec![
    //         dir1.clone(),
    //         file1.clone(),
    //         dir2.clone(),
    //         dir3.clone(),
    //         file3.clone(),
    //         file2.clone(),
    //         link3.clone(),
    //         link1.clone(),
    //         link2.clone(),
    //     ]);

    //     assert_stdfs_remove_all!(&tmpdir);
    // }

    // #[test]
    // fn test_vfs_stdfs_copy_follow()
    // {
    //     let tmpdir = assert_stdfs_setup!();

    //     // don't follow file link - copy
    //     let file1 = tmpdir.mash("file1");
    //     let link1 = tmpdir.mash("link1");
    //     let link2 = tmpdir.mash("link2");
    //     assert!(Stdfs::write(&file1, "file1").is_ok());
    //     assert_eq!(Stdfs::symlink(&file1, &link1).unwrap(), link1);
    //     assert!(Stdfs::copy_b(&link1, &link2).exec().is_ok());
    //     assert_eq!(Stdfs::readlink(&link1).unwrap(), PathBuf::from("file1"));

    //     // follow file link - copy
    //     let file2 = tmpdir.mash("file2");
    //     assert!(Stdfs::copy_b(&link1, &file2).follow(true).exec().is_ok());
    //     assert!(Stdfs::readlink(&file2).is_err());
    //     assert_eq!(Stdfs::read(&file2).unwrap(), "file1");

    //     // don't follow dir link - copy
    //     let dir1 = tmpdir.mash("dir1");
    //     let dir1file = dir1.mash("dir1file");
    //     let dir1link1 = tmpdir.mash("dir1link1");
    //     let dir1link2 = tmpdir.mash("dir1link2");
    //     assert!(Stdfs::mkdir(&dir1).is_ok());
    //     assert!(Stdfs::write(&dir1file, "dir1file").is_ok());
    //     assert_eq!(Stdfs::symlink(&dir1, &dir1link1).unwrap(), dir1link1);
    //     assert!(Stdfs::copy_b(&dir1link1, &dir1link2).exec().is_ok());
    //     assert_eq!(Stdfs::readlink(&dir1link1).unwrap(), PathBuf::from("dir1"));

    //     // follow dir link - copy
    //     let dir2 = tmpdir.mash("dir2");
    //     let dir2file = dir2.mash("dir1file");
    //     assert!(Stdfs::copy_b(&dir1link1, &dir2).follow(true).exec().is_ok());
    //     assert_eq!(Stdfs::read(&dir2file).unwrap(), "dir1file");
    //     assert!(Stdfs::readlink(&dir2file).is_err());

    //     assert_stdfs_remove_all!(&tmpdir);
    // }
}

#[allow(unused_imports)]
use super::prelude::*;

/// Setup Vfs testing components using the current provider is
///
/// This provides an abstraction over VirtualFileSystem implementations such that we can easily
/// switch out a Memfs backend for a Stdfs backend without modifying the testing algorithms. Vfs
/// tests will default to using the `testing::TEST_TEMP_DIR` as the root of testing and create a new
/// directory inside that using the derived fully qualified function name or given function name
/// when it can't be derived.
///
/// ### Warning
/// Since doc tests always have a default function name of `rust_out::main` its required to override
/// the `func_name` param to get a unique directory to work with in the Stdfs case as you won't get
/// a unique directory created to work from and could cause testing collisions.
///
/// ### Returns
/// * `tmpdir` - the temp directory that was created for the test function to work in
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// let tmpdir = assert_setup!("unique_func_name");
/// assert_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_setup {
    ($func:expr) => {{
        let (_, tmpdir) = assert_vfs_setup!(vfs::VFS.read().unwrap().clone(), $func);
        tmpdir
    }};
    () => {{
        let (_, tmpdir) = assert_vfs_setup!(vfs::VFS.read().unwrap().clone());
        tmpdir
    }};
}

/// Setup Vfs testing components with Memfs provider
///
/// This provides an abstraction over VirtualFileSystem implementations such that we can easily
/// switch out a Memfs backend for a Stdfs backend without modifying the testing algorithms. Vfs
/// tests will default to using the `testing::TEST_TEMP_DIR` as the root of testing and create a new
/// directory inside that using the derived fully qualified function name or given function name
/// when it can't be derived.
///
/// ### Warning
/// Since doc tests always have a default function name of `rust_out::main` its required to override
/// the `func_name` param to get a unique directory to work with in the Stdfs case as you won't get
/// a unique directory created to work from and could cause testing collisions.
///
/// ### Returns
/// * `tmpdir` - the temp directory that was created for the test function to work in
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// let tmpdir = assert_memfs_setup!("unique_func_name");
/// assert_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_memfs_setup {
    ($func:expr) => {{
        assert!(vfs::set_memfs().is_ok());
        let (_, tmpdir) = assert_vfs_setup!(vfs::VFS.read().unwrap().clone(), $func);
        tmpdir
    }};
    () => {{
        assert!(vfs::set_memfs().is_ok());
        let (_, tmpdir) = assert_vfs_setup!(vfs::VFS.read().unwrap().clone());
        tmpdir
    }};
}

/// Setup Vfs testing components with Stdfs provider
///
/// This provides an abstraction over VirtualFileSystem implementations such that we can easily
/// switch out a Memfs backend for a Stdfs backend without modifying the testing algorithms. Vfs
/// tests will default to using the `testing::TEST_TEMP_DIR` as the root of testing and create a new
/// directory inside that using the derived fully qualified function name or given function name
/// when it can't be derived.
///
/// ### Warning
/// Since doc tests always have a default function name of `rust_out::main` its required to override
/// the `func_name` param to get a unique directory to work with in the Stdfs case as you won't get
/// a unique directory created to work from and could cause testing collisions.
///
/// ### Returns
/// * `tmpdir` - the temp directory that was created for the test function to work in
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// let tmpdir = assert_stdfs_setup!("unique_func_name");
/// assert_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_stdfs_setup {
    ($func:expr) => {{
        assert!(vfs::set_stdfs().is_ok());
        let (_, tmpdir) = assert_vfs_setup!(vfs::VFS.read().unwrap().clone(), $func);
        tmpdir
    }};
    () => {{
        assert!(vfs::set_stdfs().is_ok());
        let (_, tmpdir) = assert_vfs_setup!(vfs::VFS.read().unwrap().clone());
        tmpdir
    }};
}

/// Assert the copy of a file
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// let file1 = vfs::root().mash("file1");
/// let file2 = vfs::root().mash("file2");
/// assert_write_all!(&file1, "this is a test");
/// assert_copyfile!(&file1, &file2);
/// ```
#[macro_export]
macro_rules! assert_copyfile {
    ($from:expr, $to:expr) => {
        assert_vfs_copyfile!(vfs::VFS.read().unwrap().clone(), $from, $to)
    };
}

/// Assert that a file or directory exists
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_exists!(vfs::root());
/// ```
#[macro_export]
macro_rules! assert_exists {
    ($path:expr) => {
        assert_vfs_exists!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert the given path doesn't exist
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_exists!("foo");
/// ```
#[macro_export]
macro_rules! assert_no_exists {
    ($path:expr) => {
        assert_vfs_no_exists!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert that the given path exists and is a directory
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_dir!("foo");
/// assert_mkdir_p!("foo");
/// assert_is_dir!("foo");
/// ```
#[macro_export]
macro_rules! assert_is_dir {
    ($path:expr) => {
        assert_vfs_is_dir!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert that the given path isn't a directory
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_dir!("foo");
/// assert_mkdir_p!("foo");
/// assert_is_dir!("foo");
/// ```
#[macro_export]
macro_rules! assert_no_dir {
    ($path:expr) => {
        assert_vfs_no_dir!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert that the given path exists and is a file
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_file!("foo");
/// assert_mkfile!("foo");
/// assert_is_file!("foo");
/// ```
#[macro_export]
macro_rules! assert_is_file {
    ($path:expr) => {
        assert_vfs_is_file!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert that the given path isn't a file
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_file!("foo");
/// ```
#[macro_export]
macro_rules! assert_no_file {
    ($path:expr) => {
        assert_vfs_no_file!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert that the given path exists and is a symlink
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_symlink!("foo");
/// assert_symlink!("foo", "bar");
/// assert_is_symlink!("foo");
/// ```
#[macro_export]
macro_rules! assert_is_symlink {
    ($path:expr) => {
        assert_vfs_is_symlink!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert that the given path isn't a symlink
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_symlink!("foo");
/// ```
#[macro_export]
macro_rules! assert_no_symlink {
    ($path:expr) => {
        assert_vfs_no_symlink!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert the creation of the given directory with the given mode
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_dir!("foo");
/// assert_mkdir_m!("foo", 0o40777);
/// assert_is_dir!("foo");
/// ```
#[macro_export]
macro_rules! assert_mkdir_m {
    ($path:expr, $mode:expr) => {
        assert_vfs_mkdir_m!(vfs::VFS.read().unwrap().clone(), $path, $mode)
    };
}

/// Assert the creation of the given directory.
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_dir!("foo");
/// assert_mkdir_p!("foo");
/// assert_is_dir!("foo");
/// ```
#[macro_export]
macro_rules! assert_mkdir_p {
    ($path:expr) => {
        assert_vfs_mkdir_p!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert the creation of a file. If the file exists no change is made
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_file!("foo");
/// assert_mkfile!("foo");
/// assert_is_file!("foo");
/// ```
#[macro_export]
macro_rules! assert_mkfile {
    ($path:expr) => {
        assert_vfs_mkfile!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert data read from the file matches the input data
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_file!("foo");
/// assert_write_all!("foo", b"foobar 1");
/// assert_read_all!("foo", "foobar 1".to_string());
/// ```
#[macro_export]
macro_rules! assert_read_all {
    ($path:expr, $data:expr) => {
        assert_vfs_read_all!(vfs::VFS.read().unwrap().clone(), $path, $data)
    };
}

/// Assert the reading of a link's target relative path
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_mkfile!("file");
/// assert_symlink!("link", "file");
/// assert_readlink!("link", PathBuf::from("file"));
/// ```
#[macro_export]
macro_rules! assert_readlink {
    ($path:expr, $target:expr) => {
        assert_vfs_readlink!(vfs::VFS.read().unwrap().clone(), $path, $target)
    };
}

/// Assert the reading of a link's target absolute path
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_mkfile!("file");
/// assert_symlink!("link", "file");
/// assert_readlink_abs!("link", vfs::root().mash("file"));
/// ```
#[macro_export]
macro_rules! assert_readlink_abs {
    ($path:expr, $data:expr) => {
        assert_vfs_readlink_abs!(vfs::VFS.read().unwrap().clone(), $path, $data)
    };
}

/// Assert the removal of the target file or directory
///
/// ### Assertion Failures
/// * Assertion fails if the target is a directory that contains files
/// * Assertion fails if the file exists after `remove` is called
/// * Assertion fails if the `remove` call fails
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_mkfile!("foo");
/// assert_remove!("foo");
/// assert_no_exists!("foo");
/// ```
#[macro_export]
macro_rules! assert_remove {
    ($path:expr) => {
        assert_vfs_remove!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert the removal of the target path
///
/// ### Assertion Failures
/// * Assertion fails if `remove_all` fails
/// * Assertion fails if the target path still exists after the call to `remove_all`
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_mkdir_p!("foo/bar");
/// assert_remove_all!("foo");
/// assert_no_exists!("foo/bar");
/// assert_no_exists!("foo");
/// ```
#[macro_export]
macro_rules! assert_remove_all {
    ($path:expr) => {
        assert_vfs_remove_all!(vfs::VFS.read().unwrap().clone(), $path)
    };
}

/// Assert the creation of a symlink. If the symlink exists no change is made
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_symlink!("foo");
/// assert_symlink!("foo", "bar");
/// assert_is_symlink!("foo");
/// ```
#[macro_export]
macro_rules! assert_symlink {
    ($link:expr, $target:expr) => {
        assert_vfs_symlink!(vfs::VFS.read().unwrap().clone(), $link, $target)
    };
}

/// Assert data is written to the given file
///
/// ### Examples
/// ```
/// use rivia_vfs::prelude::*;
///
/// assert!(vfs::set_memfs().is_ok());
/// assert_no_file!("foo");
/// assert_write_all!("foo", b"foobar");
/// assert_is_file!("foo");
/// ```
#[macro_export]
macro_rules! assert_write_all {
    ($path:expr, $data:expr) => {
        assert_vfs_write_all!(vfs::VFS.read().unwrap().clone(), $path, $data)
    };
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[allow(dead_code)]
    fn dump_memfs()
    {
        if let Vfs::Memfs(ref x) = **vfs::VFS.read().unwrap() {
            println!("{}", x);
        }
    }

    #[test]
    fn test_assert_setup()
    {
        let tmpdir = assert_memfs_setup!();
        let expected =
            vfs::root().mash(testing::TEST_TEMP_DIR).mash("rivia_vfs::assert::tests::test_assert_setup");
        assert_eq!(&tmpdir, &expected);
        assert_exists!(&expected);

        // Try with a function name override
        let tmpdir = assert_memfs_setup!("foobar_setup");
        let expected = vfs::root().mash(testing::TEST_TEMP_DIR).mash("foobar_setup");
        assert_eq!(&tmpdir, &expected);
        assert_exists!(&expected);
    }

    #[test]
    fn test_assert_copyfile()
    {
        let tmpdir = assert_memfs_setup!();
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");
        assert_write_all!(&file1, "this is a test");
        assert_copyfile!(&file1, &file2);
    }

    #[test]
    fn test_assert_exists_and_no_exists()
    {
        let tmpdir = assert_memfs_setup!();

        // Test file exists
        {
            let file = tmpdir.mash("file");
            assert_no_exists!(&file);
            assert_mkfile!(&file);
            assert_exists!(&file);
            assert_remove!(&file);
            assert_no_exists!(&file);
        }

        // Test dir exists
        {
            let dir1 = tmpdir.mash("dir1");
            assert_no_exists!(&dir1);
            assert_mkdir_p!(&dir1);
            assert_exists!(&dir1);
            assert_remove_all!(&dir1);
            assert_no_exists!(&dir1);
        }
    }

    #[test]
    fn test_assert_is_dir_no_dir()
    {
        let tmpdir = assert_memfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        assert_no_dir!(&dir1);
        assert_mkdir_p!(&dir1);
        assert_is_dir!(&dir1);
    }

    #[test]
    fn test_assert_is_file_no_file()
    {
        let tmpdir = assert_memfs_setup!();
        let file1 = tmpdir.mash("file1");
        assert_no_file!(&file1);
        assert_mkfile!(&file1);
        assert_is_file!(&file1);
    }

    #[test]
    fn test_assert_is_symlink_no_symlink()
    {
        let tmpdir = assert_memfs_setup!();
        let file1 = tmpdir.mash("file1");
        let link1 = tmpdir.mash("link1");

        // happy path
        assert_no_symlink!(&file1);
        assert_symlink!(&link1, &file1);
        assert_is_symlink!(&link1);

        // exists and is not a symlink
        assert_mkfile!(&file1);
        assert_no_symlink!(&file1);
    }

    #[test]
    fn test_assert_mkdir_m()
    {
        let tmpdir = assert_memfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        assert_no_dir!(&dir1);
        assert_mkdir_m!(&dir1, 0o40777);
        assert_eq!(vfs::mode(&dir1).unwrap(), 0o40777);
        assert_is_dir!(&dir1);
    }

    #[test]
    fn test_assert_mkdir_p()
    {
        let tmpdir = assert_memfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        assert_no_dir!(&dir1);
        assert_mkdir_p!(&dir1);
        assert_is_dir!(&dir1);
    }

    #[test]
    fn test_assert_mkfile()
    {
        let tmpdir = assert_memfs_setup!();
        let file1 = tmpdir.mash("file1");
        assert_no_file!(&file1);
        assert_mkfile!(&file1);
        assert_is_file!(&file1);
    }

    #[test]
    fn test_assert_read_all()
    {
        let tmpdir = assert_memfs_setup!();
        let file = tmpdir.mash("foo");
        assert_write_all!(&file, b"foobar 1");
        assert_read_all!(&file, "foobar 1".to_string());
    }

    #[test]
    fn test_assert_readlink()
    {
        let tmpdir = assert_memfs_setup!();
        let dir = tmpdir.mash("dir");
        let link = dir.mash("link");
        let file = tmpdir.mash("file");
        assert_mkdir_p!(&dir);
        assert_mkfile!(&file);

        assert_no_symlink!(&link);
        assert_symlink!(&link, &file);
        assert_is_symlink!(&link);
        assert_readlink!(&link, PathBuf::from("..").mash("file"));
    }

    #[test]
    fn test_assert_readlink_abs()
    {
        let tmpdir = assert_memfs_setup!();
        let dir = tmpdir.mash("dir");
        let link = dir.mash("link");
        let file = tmpdir.mash("file");
        assert_mkdir_p!(&dir);
        assert_mkfile!(&file);

        assert_no_symlink!(&link);
        assert_symlink!(&link, &file);
        assert_is_symlink!(&link);
        assert_readlink_abs!(&link, &file);
    }

    #[test]
    fn test_assert_remove()
    {
        let tmpdir = assert_memfs_setup!();
        let file1 = tmpdir.mash("file1");
        assert_remove!(&file1);
        assert_mkfile!(&file1);
        assert_is_file!(&file1);
        assert_remove!(&file1);
        assert_no_file!(&file1);
    }

    #[test]
    fn test_assert_remove_all()
    {
        let tmpdir = assert_memfs_setup!();
        let file1 = tmpdir.mash("file1");
        assert_mkfile!(&file1);
        assert_is_file!(&file1);
        assert_remove_all!(&tmpdir);
        assert_no_dir!(&tmpdir);
    }

    #[test]
    fn test_assert_symlink()
    {
        let tmpdir = assert_memfs_setup!();
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let link1 = tmpdir.mash("link1");
        assert_mkdir_p!(&dir1);
        assert_mkfile!(&file1);

        assert_no_symlink!(&link1);
        assert_symlink!(&link1, &file1);
        assert_is_symlink!(&link1);
    }

    #[test]
    fn test_assert_write_all()
    {
        let tmpdir = assert_memfs_setup!();
        let file = tmpdir.mash("foo");
        assert_write_all!(&file, b"foobar 1");
        assert_read_all!(&file, "foobar 1".to_string());
    }
}

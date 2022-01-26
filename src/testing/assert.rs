use std::path::PathBuf;

use crate::{
    function_fqn, panic_msg,
    sys::{FileSystem, PathExt, Vfs},
};

/// Wrapper around `vfs_setup_p` to automatically resolve the function name if possible
///
/// ### Warning
/// Since doc tests always have a default function name of `rust_out::main` its required to use the
/// `vfs_setup_p` form to pass to avoid testing collisions.
///
/// ### Returns
/// * `vfs` - the vfs instance passed to the function for reference
/// * `tmpdir` - the temp directory that was created for the test function to work in
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// fn test_setup_default() {
///     let (vfs, tmpdir) = testing::vfs_setup!(Vfs::vfs());
///     assert_vfs_remove_all!(vfs, tmpdir);
/// }
/// test_setup_default();
/// ```
pub fn vfs_setup(vfs: Vfs) -> (Vfs, PathBuf)
{
    vfs_setup_p(vfs, None)
}

/// Setup Vfs testing components
///
/// This provides an abstraction over FileSystem implementations such that we can easily switch out
/// a Memfs backend for a Stdfs backend without modifying the testing algorithms. Vfs tests will
/// default to using the `testing::TEST_TEMP_DIR` as the root of testing and create a new directory
/// inside that using the derived fully qualified function name or given function name when it can't
/// be derived.
///
/// ### Warning
/// Since doc tests always have a default function name of `rust_out::main` its required to override
/// the `func_name` param to get a unique directory to work with in the Stdfs case as you won't get
/// a unique directory created to work from and could cause testing collisions.
///
/// ### Returns
/// * `vfs` - the vfs instance passed to the function for reference
/// * `tmpdir` - the temp directory that was created for the test function to work in
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let (vfs, tmpdir) = testing::vfs_setup_p!(Vfs::vfs(), "unique_func_name");
/// assert_vfs_remove_all!(vfs, tmpdir);
/// ```
pub fn vfs_setup_p(vfs: Vfs, func_name: Option<&str>) -> (Vfs, PathBuf)
{
    // Get the absolute path to the tmpdir
    let abs = match vfs.abs(super::TEST_TEMP_DIR) {
        Ok(x) => x,
        _ => panic_msg!("assert_vfs_setup!", "failed to get absolute path", super::TEST_TEMP_DIR),
    };

    // Optionally override the derived function name with the one given
    let tmpdir = abs.mash(match func_name {
        Some(name) => name.as_ref(),
        None => function_fqn!(),
    });
    if tmpdir == abs {
        panic_msg!("assert_vfs_setup!", "function name is empty", &tmpdir);
    }

    // Ensure the tmpdir has been removed
    if vfs.remove_all(&tmpdir).is_err() {
        panic_msg!("assert_vfs_setup!", "failed while removing directory", &tmpdir);
    }

    // Create the tmpdir directory
    match vfs.mkdir_p(&tmpdir) {
        Ok(dir) => dir,
        _ => panic_msg!("assert_vfs_setup!", "failed while creating directory", &tmpdir),
    };

    (vfs, tmpdir)
}

/// Assert that a file or directory exists
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
///  let (vfs, tmpdir) = assert_vfs_setup!(Vfs::vfs());
/// assert_vfs_exists!(&vfs, "/");
/// ```
#[macro_export]
macro_rules! assert_vfs_exists {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_exists!", "failed to get absolute path", $path),
        };
        if !$vfs.exists(&target) {
            panic_msg!("assert_vfs_exists!", "doesn't exist", &target);
        }
    };
}

/// Assert the given path doesn't exist
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::vfs());
/// assert_vfs_no_exists!(&vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_no_exists {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_no_exists!", "failed to get absolute path", $path),
        };
        if $vfs.exists(&target) {
            panic_msg!("assert_vfs_no_exists!", "still exists", &target);
        }
    };
}

/// Assert that the given path exists and is a directory
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::vfs());
/// assert_vfs_no_dir!(&vfs, "foo");
/// assert_vfs_mkdir_p!(&vfs, "foo");
/// assert_vfs_is_dir!(&vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_is_dir {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_is_dir!", "failed to get absolute path", $path),
        };
        if $vfs.exists(&target) {
            if !$vfs.is_dir(&target) {
                panic_msg!("assert_vfs_is_dir!", "exists but is not a directory", &target);
            }
        } else {
            panic_msg!("assert_vfs_is_dir!", "doesn't exist", &target);
        }
    };
}

/// Assert that the given path isn't a directory
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::vfs());
/// assert_vfs_no_dir!(&vfs, "foo");
/// assert_vfs_mkdir_p!(&vfs, "foo");
/// assert_vfs_is_dir!(&vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_no_dir {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_no_dir!", "failed to get absolute path", $path),
        };
        if $vfs.exists(&target) {
            if !$vfs.is_dir(&target) {
                panic_msg!("assert_vfs_no_dir!", "exists and is not a directory", &target);
            } else {
                panic_msg!("assert_vfs_no_dir!", "directory still exists", &target);
            }
        }
    };
}

/// Assert that the given path exists and is a file
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::vfs());
/// assert_vfs_no_file!(&vfs, "foo");
/// assert_vfs_mkfile!(&vfs, "foo");
/// assert_vfs_is_file!(&vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_is_file {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_is_file!", "failed to get absolute path", $path),
        };
        if $vfs.exists(&target) {
            if !$vfs.is_file(&target) {
                panic_msg!("assert_vfs_is_file!", "exists but is not a file", &target);
            }
        } else {
            panic_msg!("assert_vfs_is_file!", "doesn't exist", &target);
        }
    };
}

/// Assert that the given path isn't a file
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::vfs());
/// assert_vfs_no_file!(&vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_no_file {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_no_file!", "failed to get absolute path", $path),
        };
        if $vfs.exists(&target) {
            if !$vfs.is_file(&target) {
                panic_msg!("assert_vfs_no_file!", "exists and is not a file", &target);
            } else {
                panic_msg!("assert_vfs_no_file!", "file still exists", &target);
            }
        }
    };
}

/// Assert the creation of the given directory.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::vfs());
/// assert_vfs_no_dir!(&vfs, "foo");
/// assert_vfs_mkdir_p!(&vfs, "foo");
/// assert_vfs_is_dir!(&vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_mkdir_p {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_mkdir_p!", "failed to get absolute path", $path),
        };
        match $vfs.mkdir_p(&target) {
            Ok(x) => {
                if &x != &target {
                    panic_compare_msg!(
                        "assert_vfs_mkdir_p!",
                        "created directory path doesn't match the target",
                        &x,
                        &target
                    );
                }
            },
            Err(e) => panic!("assert_vfs_mkdir_p!: {}", e.to_string()),
        };
        if !$vfs.is_dir(&target) {
            panic_msg!("assert_vfs_mkdir_p!", "failed to create directory", &target);
        }
    };
}

/// Assert the creation of a file. If the file exists no change is made
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::vfs());
/// assert_vfs_no_file!(&vfs, "foo");
/// assert_vfs_mkfile!(&vfs, "foo");
/// assert_vfs_is_file!(&vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_mkfile {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_mkfile!", "failed to get absolute path", $path),
        };
        if $vfs.exists(&target) {
            if !$vfs.is_file(&target) {
                panic_msg!("assert_vfs_mkfile!", "is not a file", &target);
            }
        } else {
            match $vfs.mkfile(&target) {
                Ok(x) => {
                    if &x != &target {
                        panic_compare_msg!(
                            "assert_vfs_mkfile!",
                            "created file path doesn't match the target",
                            &x,
                            &target
                        );
                    }
                },
                _ => panic_msg!("assert_vfs_mkfile!", "failed while creating file", &target),
            };
            if !$vfs.is_file(&target) {
                panic_msg!("assert_vfs_mkfile!", "file doesn't exist", &target);
            }
        }
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
/// use rivia::prelude::*;
///
/// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::vfs());
/// assert_vfs_mkfile!(&vfs, "foo");
/// assert_vfs_remove!(&vfs, "foo");
/// assert_vfs_no_exists!(&vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_remove {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_remove!", "failed to get absolute path", $path),
        };
        if $vfs.exists(&target) {
            if !$vfs.is_dir(&target) {
                if $vfs.remove(&target).is_err() {
                    panic_msg!("assert_vfs_remove!", "failed removing file", &target);
                }
                if $vfs.exists(&target) {
                    panic_msg!("assert_vfs_remove!", "file still exists", &target);
                }
            } else {
                if $vfs.remove(&target).is_err() {
                    panic_msg!("assert_vfs_remove!", "failed removing directory", &target);
                }
                if $vfs.exists(&target) {
                    panic_msg!("assert_vfs_remove!", "directory still exists", &target);
                }
            }
        }
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
/// use rivia::prelude::*;
///
/// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::vfs());
/// assert_vfs_mkdir_p!(&vfs, "foo/bar");
/// assert_vfs_remove_all!(&vfs, "foo");
/// assert_vfs_no_exists!(&vfs, "foo/bar");
/// assert_vfs_no_exists!(&vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_remove_all {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_remove_all!", "failed to get absolute path", $path),
        };
        if $vfs.remove_all(&target).is_err() {
            panic_msg!("assert_vfs_remove_all!", "failed while removing", &target);
        }
        if $vfs.exists(&target) {
            panic_msg!("assert_vfs_remove_all!", "still exists", &target);
        }
    };
}

/// Helper function for testing to simply panic with the given message in a repeatable formatting.
///
/// ### Examples
/// ```ignore,no_run
/// use rivia::prelude::*;
///
/// panic_msg!("assert_vfs_mkdir_p!", "failed to create directory", PathBuf::from("foo"));
/// ```
#[macro_export]
macro_rules! panic_msg {
    ($name:expr, $msg:expr, $target:expr) => {
        panic!("\n{}: {}\n  target: {}\n", $name, $msg, format!("{:?}", $target))
    };
}

/// Helper function for testing to simply panic with the given message in a repeatable formatting.
///
/// ### Examples
/// ```ignore,no_run
/// use rivia::prelude::*;
///
/// panic_msg!("assert_vfs_mkdir_p!", "failed to create directory", PathBuf::from("foo"), PathBuf::from("foo"));
/// ```
#[macro_export]
macro_rules! panic_compare_msg {
    ($name:expr, $msg:expr, $actual:expr, $target:expr) => {
        panic!(
            "\n{}: {}\n  actual: {}\n  target: {}\n",
            $name,
            $msg,
            format!("{:?}", $actual),
            format!("{:?}", $target)
        )
    };
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[test]
    fn test_assert_vfs_exists_and_no_exists()
    {
        let (vfs, tmpdir) = testing::vfs_setup(Vfs::memfs());

        // Test file exists
        {
            let file = tmpdir.mash("file");
            assert_vfs_no_exists!(&vfs, &file);
            assert!(!vfs.exists(&file));
            assert_vfs_mkfile!(&vfs, &file);
            assert_vfs_exists!(&vfs, &file);
            assert!(vfs.exists(&file));

            assert_vfs_remove!(&vfs, &file);
            assert_vfs_no_exists!(&vfs, &file);
            assert!(!vfs.exists(&file));
        }

        // Test dir exists
        {
            let dir1 = tmpdir.mash("dir1");
            assert_vfs_no_exists!(&vfs, &dir1);
            assert!(!vfs.exists(&dir1));
            assert_vfs_mkdir_p!(&vfs, &dir1);
            assert_vfs_exists!(&vfs, &dir1);
            assert!(vfs.exists(&dir1));

            assert_vfs_remove_all!(&vfs, &dir1);
            assert_vfs_no_exists!(&vfs, &dir1);
            assert!(!vfs.exists(&dir1));
        }

        // exists: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_exists!(&vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_exists!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists: doesn't exist
        let file1 = tmpdir.mash("file1");
        let result = testing::capture_panic(|| {
            assert_vfs_exists!(&vfs, &file1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_exists!: doesn't exist\n  target: {:?}\n", &file1)
        );

        // no exists: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_no_exists!(&vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_no_exists!: failed to get absolute path\n  target: \"\"\n"
        );

        // no exists: does exist
        assert_vfs_mkfile!(&vfs, &file1);
        let result = testing::capture_panic(|| {
            assert_vfs_no_exists!(&vfs, &file1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_no_exists!: still exists\n  target: {:?}\n", &file1)
        );

        assert_vfs_remove_all!(&vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_is_dir_no_dir()
    {
        let (vfs, tmpdir) = testing::vfs_setup(Vfs::memfs());
        let dir1 = tmpdir.mash("dir1");
        let dir2 = tmpdir.mash("dir2");

        // happy path
        assert_vfs_no_dir!(&vfs, &dir1);
        assert!(!vfs.is_dir(&dir1));
        assert_vfs_mkdir_p!(&vfs, &dir1);
        assert_vfs_is_dir!(&vfs, &dir1);
        assert!(vfs.is_dir(&dir1));

        // is_dir: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_is_dir!(&vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_is_dir!: failed to get absolute path\n  target: \"\"\n"
        );

        // is_dir: doesn't exist
        let result = testing::capture_panic(|| {
            assert_vfs_is_dir!(&vfs, &dir2);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_is_dir!: doesn't exist\n  target: {:?}\n", &dir2)
        );

        // no_dir: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_no_dir!(&vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_no_dir!: failed to get absolute path\n  target: \"\"\n"
        );

        // no_dir: does exist
        let result = testing::capture_panic(|| {
            assert_vfs_no_dir!(&vfs, &dir1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_no_dir!: directory still exists\n  target: {:?}\n", &dir1)
        );

        assert_vfs_remove_all!(&vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_is_file_no_file()
    {
        let (vfs, tmpdir) = testing::vfs_setup(Vfs::memfs());
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");

        // happy path
        assert_vfs_no_file!(&vfs, &file1);
        assert!(!vfs.is_file(&file1));
        assert_vfs_mkfile!(&vfs, &file1);
        assert_vfs_is_file!(&vfs, &file1);
        assert!(vfs.is_file(&file1));

        // is_file: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_is_file!(&vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_is_file!: failed to get absolute path\n  target: \"\"\n"
        );

        // is_file: doesn't exist
        let result = testing::capture_panic(|| {
            assert_vfs_is_file!(&vfs, &file2);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_is_file!: doesn't exist\n  target: {:?}\n", &file2)
        );

        // no_file: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_no_file!(&vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_no_file!: failed to get absolute path\n  target: \"\"\n"
        );

        // no_file: does exist
        let result = testing::capture_panic(|| {
            assert_vfs_no_file!(&vfs, &file1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_no_file!: file still exists\n  target: {:?}\n", &file1)
        );

        assert_vfs_remove_all!(&vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_mkdir_p()
    {
        let (vfs, tmpdir) = testing::vfs_setup(Vfs::memfs());
        let file1 = tmpdir.mash("file1");
        let dir1 = tmpdir.mash("dir1");
        assert_vfs_mkfile!(&vfs, &file1);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_mkdir_p!(&vfs, "");
        });

        // fail abs
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_mkdir_p!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a directory
        let result = testing::capture_panic(|| {
            assert_vfs_mkdir_p!(&vfs, &file1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("assert_vfs_mkdir_p!: Target path is not a directory: {}", &file1.display())
        );

        // happy path
        assert_vfs_no_dir!(&vfs, &dir1);
        assert_vfs_mkdir_p!(&vfs, &dir1);
        assert_vfs_is_dir!(&vfs, &dir1);

        assert_vfs_remove_all!(&vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_mkfile()
    {
        let (vfs, tmpdir) = testing::vfs_setup(Vfs::memfs());
        let file1 = tmpdir.mash("file1");
        let dir1 = tmpdir.mash("dir1");
        assert_vfs_mkdir_p!(&vfs, &dir1);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_mkfile!(&vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_mkfile!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a file
        let result = testing::capture_panic(|| {
            assert_vfs_mkfile!(&vfs, &dir1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_mkfile!: is not a file\n  target: \"{}\"\n", dir1.display())
        );

        // happy path
        assert_vfs_no_file!(&vfs, &file1);
        assert_vfs_mkfile!(&vfs, &file1);
        assert_vfs_is_file!(&vfs, &file1);

        assert_vfs_remove_all!(&vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_remove()
    {
        let (vfs, tmpdir) = testing::vfs_setup(Vfs::memfs());
        let file1 = tmpdir.mash("file1");

        // happy path
        assert_vfs_remove!(&vfs, &file1);
        assert_vfs_mkfile!(&vfs, &file1);
        assert_vfs_is_file!(&vfs, &file1);
        assert_vfs_remove!(&vfs, &file1);
        assert_vfs_no_file!(&vfs, &file1);

        // bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_remove!(&vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_remove!: failed to get absolute path\n  target: \"\"\n"
        );

        // is a directory
        let result = testing::capture_panic(|| {
            assert_vfs_remove!(&vfs, &tmpdir);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_remove!: exists and isn't a file\n  target: {:?}\n", &tmpdir)
        );

        assert_vfs_remove_all!(&vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_remove_all()
    {
        let (vfs, tmpdir) = testing::vfs_setup(Vfs::memfs());
        let file1 = tmpdir.mash("file1");

        assert_vfs_mkfile!(&vfs, &file1);
        assert_vfs_is_file!(&vfs, &file1);
        assert_vfs_remove_all!(&vfs, &tmpdir);
        assert_vfs_no_dir!(&vfs, &tmpdir);

        // bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_remove_all!(&vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_remove_all!: failed to get absolute path\n  target: \"\"\n"
        );
    }
}

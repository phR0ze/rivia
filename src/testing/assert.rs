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
/// let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs(), "unique_func_name");
/// assert_vfs_remove_all!(vfs, &tmpdir);
/// ```
#[macro_export]
macro_rules! assert_vfs_setup {
    ($vfs:expr $(, $func:expr )?) => {{
        // Setting this value here as a weird work around to Rust either not fully instantiating
        // the vfs value or to it cleaning up the instance before its used. Either way it won't work
        // with `let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());` syntax unless this is set here.
        let vfs = $vfs;

        // Get the absolute path to the tmpdir
        let abs = match vfs.abs(testing::TEST_TEMP_DIR) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_setup!", "failed to get absolute path", testing::TEST_TEMP_DIR),
        };

        // Optionally override the derived function name with the one given
        #[allow(unused_variables)]
        let func_name: Option<&str> = None;
        $( let func_name = Some($func); )?
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
        assert_vfs_mkdir_p!(vfs, &tmpdir);

        (vfs, tmpdir)
    }};
}

/// Assert the copy of a file
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// let file1 = vfs.root().mash("file1");
/// let file2 = vfs.root().mash("file2");
/// assert_vfs_write_all!(vfs, &file1, "this is a test");
/// assert_vfs_copyfile!(vfs, &file1, &file2);
/// ```
#[macro_export]
macro_rules! assert_vfs_copyfile {
    ($vfs:expr, $from:expr, $to:expr) => {
        let src = match $vfs.abs($from) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_copyfile!", "failed to get absolute src path", $from),
        };
        let dst = match $vfs.abs($to) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_copyfile!", "failed to get absolute dst path", $to),
        };
        if !$vfs.exists(&src) {
            panic_msg!("assert_vfs_copyfile!", "doesn't exist", &src);
        } else if !$vfs.is_file(&src) {
            panic_msg!("assert_vfs_copyfile!", "is not a file", &src);
        } else {
            match $vfs.copy(&src, &dst) {
                Ok(_) => match $vfs.read_all(&src) {
                    Ok(x) => match $vfs.read_all(&dst) {
                        Ok(y) => {
                            if &x != &y {
                                panic_compare_msg!("assert_vfs_copyfile!", "src data doesn't match dst", &x, &y);
                            }
                        },
                        _ => panic_msg!("assert_vfs_copyfile!", "failed reading dst file", &dst),
                    },
                    _ => panic_msg!("assert_vfs_copyfile!", "failed reading src file", &src),
                },
                _ => panic_msg!("assert_vfs_copyfile!", "failed while copying src file", &src),
            };
            if !$vfs.is_file(&dst) {
                panic_msg!("assert_vfs_copyfile!", "dst doesn't exist", &dst);
            }
        }
    };
}

/// Assert that a file or directory exists
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_exists!(vfs, "/");
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
/// let vfs = Vfs::memfs();
/// assert_vfs_no_exists!(vfs, "foo");
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
/// let vfs = Vfs::memfs();
/// assert_vfs_no_dir!(vfs, "foo");
/// assert_vfs_mkdir_p!(vfs, "foo");
/// assert_vfs_is_dir!(vfs, "foo");
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
/// let vfs = Vfs::memfs();
/// assert_vfs_no_dir!(vfs, "foo");
/// assert_vfs_mkdir_p!(vfs, "foo");
/// assert_vfs_is_dir!(vfs, "foo");
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
/// let vfs = Vfs::memfs();
/// assert_vfs_no_file!(vfs, "foo");
/// assert_vfs_mkfile!(vfs, "foo");
/// assert_vfs_is_file!(vfs, "foo");
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
/// let vfs = Vfs::memfs();
/// assert_vfs_no_file!(vfs, "foo");
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

/// Assert that the given path exists and is a symlink
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_no_symlink!(vfs, "foo");
/// assert_vfs_symlink!(vfs, "foo", "bar");
/// assert_vfs_is_symlink!(vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_is_symlink {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_is_symlink!", "failed to get absolute path", $path),
        };
        if $vfs.exists(&target) {
            if !$vfs.is_symlink(&target) {
                panic_msg!("assert_vfs_is_link!", "exists but is not a symlink", &target);
            }
        } else {
            panic_msg!("assert_vfs_is_symlink!", "symlink doesn't exist", &target);
        }
    };
}

/// Assert that the given path isn't a symlink
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_no_symlink!(vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_no_symlink {
    ($vfs:expr, $path:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_no_symlink!", "failed to get absolute path", $path),
        };
        if $vfs.exists(&target) {
            if $vfs.is_symlink(&target) {
                panic_msg!("assert_vfs_no_symlink!", "exists and is a symlink", &target);
            }
        }
    };
}

/// Assert the creation of the given directory with the given mode
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_no_dir!(vfs, "foo");
/// assert_vfs_mkdir_m!(vfs, "foo", 0o40777);
/// assert_vfs_is_dir!(vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_mkdir_m {
    ($vfs:expr, $path:expr, $mode:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_mkdir_m!", "failed to get absolute path", $path),
        };
        match $vfs.mkdir_m(&target, $mode) {
            Ok(x) => {
                if &x != &target {
                    panic_compare_msg!(
                        "assert_vfs_mkdir_m!",
                        "created directory path doesn't match the target",
                        &x,
                        &target
                    );
                }
                match $vfs.mode(&target) {
                    Ok(x) => {
                        if x != $mode {
                            panic_compare_msg!(
                                "assert_vfs_mkdir_m!",
                                "created directory mode doesn't match the target",
                                &x,
                                &target
                            );
                        }
                    },
                    Err(e) => panic!("assert_vfs_mkdir_m!: mode failure for {}", e.to_string()),
                };
            },
            Err(e) => panic!("assert_vfs_mkdir_m!: {}", e.to_string()),
        };
        if !$vfs.is_dir(&target) {
            panic_msg!("assert_vfs_mkdir_m!", "failed to create directory", &target);
        }
    };
}

/// Assert the creation of the given directory.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_no_dir!(vfs, "foo");
/// assert_vfs_mkdir_p!(vfs, "foo");
/// assert_vfs_is_dir!(vfs, "foo");
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
/// let vfs = Vfs::memfs();
/// assert_vfs_no_file!(vfs, "foo");
/// assert_vfs_mkfile!(vfs, "foo");
/// assert_vfs_is_file!(vfs, "foo");
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

/// Assert data read from the file matches the input data
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_no_file!(vfs, "foo");
/// assert_vfs_write_all!(vfs, "foo", b"foobar 1");
/// assert_vfs_read_all!(vfs, "foo", "foobar 1".to_string());
/// ```
#[macro_export]
macro_rules! assert_vfs_read_all {
    ($vfs:expr, $path:expr, $data:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_read_all!", "failed to get absolute path", $path),
        };
        if !$vfs.is_file(&target) {
            panic_msg!("assert_vfs_read_all!", "file doesn't exist or is not a file", &target);
        }
        match $vfs.read_all(&target) {
            Ok(data) => {
                if data != $data {
                    panic_msg!("assert_vfs_read_all!", "read data doesn't equal given data", &target);
                }
            },
            _ => panic_msg!("assert_vfs_read_all!", "failed while reading file", &target),
        };
    };
}

/// Assert the reading of a link's target relative path
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_mkfile!(vfs, "file");
/// assert_vfs_symlink!(vfs, "link", "file");
/// assert_vfs_readlink!(vfs, "link", PathBuf::from("file"));
/// ```
#[macro_export]
macro_rules! assert_vfs_readlink {
    ($vfs:expr, $path:expr, $target:expr) => {
        let link = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_readlink!", "failed to get absolute path", $path),
        };
        if !$vfs.is_symlink(&link) {
            panic_msg!("assert_vfs_readlink!", "file doesn't exist or is not a symlink", &link);
        }
        match $vfs.readlink(&link) {
            Ok(x) => {
                if x.to_string().unwrap() != $target.to_string().unwrap() {
                    panic_msg!("assert_vfs_readlink!", "link target doesn't equal given path", &x);
                }
            },
            _ => panic_msg!("assert_vfs_readlink!", "failed while reading link", &link),
        };
    };
}

/// Assert the reading of a link's target absolute path
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_mkfile!(vfs, "file");
/// assert_vfs_symlink!(vfs, "link", "file");
/// assert_vfs_readlink_abs!(vfs, "link", vfs.root().mash("file"));
/// ```
#[macro_export]
macro_rules! assert_vfs_readlink_abs {
    ($vfs:expr, $path:expr, $data:expr) => {
        let link = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_readlink_abs!", "failed to get absolute path", $path),
        };
        let target = match $vfs.abs($data) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_readlink_abs!", "failed to get absolute path", $data),
        };
        if !$vfs.is_symlink(&link) {
            panic_msg!("assert_vfs_readlink_abs!", "file doesn't exist or is not a symlink", &link);
        }
        match $vfs.readlink_abs(&link) {
            Ok(x) => {
                if !target.has_suffix(&x) {
                    panic_msg!("assert_vfs_readlink_abs!", "link target doesn't equal given path", &x);
                }
            },
            _ => panic_msg!("assert_vfs_readlink_abs!", "failed while reading link", &link),
        };
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
/// let vfs = Vfs::memfs();
/// assert_vfs_mkfile!(vfs, "foo");
/// assert_vfs_remove!(vfs, "foo");
/// assert_vfs_no_exists!(vfs, "foo");
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
/// let vfs = Vfs::memfs();
/// assert_vfs_mkdir_p!(vfs, "foo/bar");
/// assert_vfs_remove_all!(vfs, "foo");
/// assert_vfs_no_exists!(vfs, "foo/bar");
/// assert_vfs_no_exists!(vfs, "foo");
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

/// Assert the creation of a symlink. If the symlink exists no change is made
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_no_symlink!(vfs, "foo");
/// assert_vfs_symlink!(vfs, "foo", "bar");
/// assert_vfs_is_symlink!(vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_symlink {
    ($vfs:expr, $link:expr, $target:expr) => {
        let link = match $vfs.abs($link) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_symlink!", "failed to get absolute path", $link),
        };
        if $vfs.exists(&link) {
            if !$vfs.is_symlink(&link) {
                panic_msg!("assert_vfs_symlink!", "is not a symlink", &link);
            }
        } else {
            match $vfs.symlink(&link, $target) {
                Ok(x) => {
                    if &x != &link {
                        panic_compare_msg!("assert_vfs_symlink!", "created link path doesn't match", &x, &link);
                    }
                },
                _ => panic_msg!("assert_vfs_symlink!", "failed while creating symlink", &link),
            };
            if !$vfs.is_symlink(&link) {
                panic_msg!("assert_vfs_symlink!", "symlink doesn't exist", &link);
            }
        }
    };
}

/// Assert data is written to the given file
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// assert_vfs_no_file!(vfs, "foo");
/// assert_vfs_write_all!(vfs, "foo", b"foobar");
/// assert_vfs_is_file!(vfs, "foo");
/// ```
#[macro_export]
macro_rules! assert_vfs_write_all {
    ($vfs:expr, $path:expr, $data:expr) => {
        let target = match $vfs.abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_vfs_write_all!", "failed to get absolute path", $path),
        };
        if $vfs.exists(&target) {
            if !$vfs.is_file(&target) {
                panic_msg!("assert_vfs_write_all!", "is not a file", &target);
            }
        } else {
            match $vfs.write_all(&target, $data) {
                Ok(_) => {
                    if !$vfs.is_file(&target) {
                        panic_msg!("assert_vfs_write_all!", "is not a file", &target);
                    }
                },
                _ => panic_msg!("assert_vfs_write_all!", "failed while writing file", &target),
            };
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
    fn test_vfs_setup()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let expected =
            vfs.root().mash(testing::TEST_TEMP_DIR).mash("rivia::testing::assert::tests::test_vfs_setup");
        assert_eq!(&tmpdir, &expected);
        assert_vfs_exists!(vfs, &expected);

        // Try with a function name override
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs(), "foobar_vfs_setup");
        let expected = vfs.root().mash(testing::TEST_TEMP_DIR).mash("foobar_vfs_setup");
        assert_eq!(&tmpdir, &expected);
        assert_vfs_exists!(vfs, &expected);
    }

    #[test]
    fn test_assert_vfs_copyfile()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());

        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");
        let dir1 = tmpdir.mash("dir1");
        assert_vfs_mkdir_p!(vfs, &dir1);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_copyfile!(vfs, "", "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_copyfile!: failed to get absolute src path\n  target: \"\"\n"
        );

        let result = testing::capture_panic(|| {
            assert_vfs_copyfile!(vfs, "foo", "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_copyfile!: failed to get absolute dst path\n  target: \"\"\n"
        );

        // src doesn't exist
        let result = testing::capture_panic(|| {
            assert_vfs_copyfile!(vfs, &file1, &file2);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_copyfile!: doesn't exist\n  target: {:?}\n", &file1)
        );

        // exists but not a file
        let result = testing::capture_panic(|| {
            assert_vfs_copyfile!(vfs, &dir1, &file2);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_copyfile!: is not a file\n  target: {:?}\n", &dir1)
        );

        // happy path
        assert_vfs_write_all!(vfs, &file1, "this is a test");
        assert_vfs_copyfile!(vfs, &file1, &file2);
    }

    #[test]
    fn test_assert_vfs_exists_and_no_exists()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());

        // Test file exists
        {
            let file = tmpdir.mash("file");
            assert_vfs_no_exists!(vfs, &file);
            assert!(!vfs.exists(&file));
            assert_vfs_mkfile!(vfs, &file);
            assert_vfs_exists!(vfs, &file);
            assert!(vfs.exists(&file));

            assert_vfs_remove!(vfs, &file);
            assert_vfs_no_exists!(vfs, &file);
            assert!(!vfs.exists(&file));
        }

        // Test dir exists
        {
            let dir1 = tmpdir.mash("dir1");
            assert_vfs_no_exists!(vfs, &dir1);
            assert!(!vfs.exists(&dir1));
            assert_vfs_mkdir_p!(vfs, &dir1);
            assert_vfs_exists!(vfs, &dir1);
            assert!(vfs.exists(&dir1));

            assert_vfs_remove_all!(vfs, &dir1);
            assert_vfs_no_exists!(vfs, &dir1);
            assert!(!vfs.exists(&dir1));
        }

        // exists: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_exists!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_exists!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists: doesn't exist
        let file1 = tmpdir.mash("file1");
        let result = testing::capture_panic(|| {
            assert_vfs_exists!(vfs, &file1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_exists!: doesn't exist\n  target: {:?}\n", &file1)
        );

        // no exists: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_no_exists!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_no_exists!: failed to get absolute path\n  target: \"\"\n"
        );

        // no exists: does exist
        assert_vfs_mkfile!(vfs, &file1);
        let result = testing::capture_panic(|| {
            assert_vfs_no_exists!(vfs, &file1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_no_exists!: still exists\n  target: {:?}\n", &file1)
        );

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_is_dir_no_dir()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let dir1 = tmpdir.mash("dir1");
        let dir2 = tmpdir.mash("dir2");

        // happy path
        assert_vfs_no_dir!(vfs, &dir1);
        assert!(!vfs.is_dir(&dir1));
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_is_dir!(vfs, &dir1);
        assert!(vfs.is_dir(&dir1));

        // is_dir: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_is_dir!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_is_dir!: failed to get absolute path\n  target: \"\"\n"
        );

        // is_dir: doesn't exist
        let result = testing::capture_panic(|| {
            assert_vfs_is_dir!(vfs, &dir2);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_is_dir!: doesn't exist\n  target: {:?}\n", &dir2)
        );

        // no_dir: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_no_dir!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_no_dir!: failed to get absolute path\n  target: \"\"\n"
        );

        // no_dir: does exist
        let result = testing::capture_panic(|| {
            assert_vfs_no_dir!(vfs, &dir1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_no_dir!: directory still exists\n  target: {:?}\n", &dir1)
        );

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_is_file_no_file()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let file1 = tmpdir.mash("file1");
        let file2 = tmpdir.mash("file2");

        // happy path
        assert_vfs_no_file!(vfs, &file1);
        assert!(!vfs.is_file(&file1));
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_is_file!(vfs, &file1);
        assert!(vfs.is_file(&file1));

        // is_file: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_is_file!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_is_file!: failed to get absolute path\n  target: \"\"\n"
        );

        // is_file: doesn't exist
        let result = testing::capture_panic(|| {
            assert_vfs_is_file!(vfs, &file2);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_is_file!: doesn't exist\n  target: {:?}\n", &file2)
        );

        // no_file: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_no_file!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_no_file!: failed to get absolute path\n  target: \"\"\n"
        );

        // no_file: does exist
        let result = testing::capture_panic(|| {
            assert_vfs_no_file!(vfs, &file1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_no_file!: file still exists\n  target: {:?}\n", &file1)
        );

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_is_symlink_no_symlink()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let file1 = tmpdir.mash("file1");
        let link1 = tmpdir.mash("link1");

        // happy path
        assert_vfs_no_symlink!(vfs, &file1);
        assert!(!vfs.is_symlink(&file1));
        assert_vfs_symlink!(vfs, &link1, &file1);
        assert_vfs_is_symlink!(vfs, &link1);
        assert!(vfs.is_symlink(&link1));

        // is_symlink: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_is_symlink!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_is_symlink!: failed to get absolute path\n  target: \"\"\n"
        );

        // is_symlink: doesn't exist
        let result = testing::capture_panic(|| {
            assert_vfs_is_symlink!(vfs, &file1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_is_symlink!: symlink doesn't exist\n  target: {:?}\n", &file1)
        );

        // no_symlink: bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_no_symlink!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_no_symlink!: failed to get absolute path\n  target: \"\"\n"
        );

        // no_symlink: does exist
        let result = testing::capture_panic(|| {
            assert_vfs_no_symlink!(vfs, &link1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_no_symlink!: exists and is a symlink\n  target: {:?}\n", &link1)
        );

        // exists and is not a symlink
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_no_symlink!(vfs, &file1);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_mkdir_m()
    {
        let vfs = Memfs::new();
        let file1 = vfs.root().mash("file1");
        let dir1 = vfs.root().mash("dir1");
        assert_vfs_mkfile!(vfs, &file1);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_mkdir_m!(vfs, "", 0o40777);
        });

        // fail abs
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_mkdir_m!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a directory
        let result = testing::capture_panic(|| {
            assert_vfs_mkdir_m!(vfs, &file1, 0o40777);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("assert_vfs_mkdir_m!: Target path is not a directory: {}", &file1.display())
        );

        // happy path
        assert_vfs_no_dir!(vfs, &dir1);
        assert_vfs_mkdir_m!(vfs, &dir1, 0o40777);
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40777);
        assert_vfs_is_dir!(vfs, &dir1);
    }

    #[test]
    fn test_assert_vfs_mkdir_p()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let file1 = tmpdir.mash("file1");
        let dir1 = tmpdir.mash("dir1");
        assert_vfs_mkfile!(vfs, &file1);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_mkdir_p!(vfs, "");
        });

        // fail abs
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_mkdir_p!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a directory
        let result = testing::capture_panic(|| {
            assert_vfs_mkdir_p!(vfs, &file1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("assert_vfs_mkdir_p!: Target path is not a directory: {}", &file1.display())
        );

        // happy path
        assert_vfs_no_dir!(vfs, &dir1);
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_is_dir!(vfs, &dir1);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_mkfile()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let file1 = tmpdir.mash("file1");
        let dir1 = tmpdir.mash("dir1");
        assert_vfs_mkdir_p!(vfs, &dir1);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_mkfile!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_mkfile!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a file
        let result = testing::capture_panic(|| {
            assert_vfs_mkfile!(vfs, &dir1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_mkfile!: is not a file\n  target: \"{}\"\n", dir1.display())
        );

        // happy path
        assert_vfs_no_file!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_is_file!(vfs, &file1);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_read_all()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_read_all!(vfs, "", "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_read_all!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a file
        let result = testing::capture_panic(|| {
            assert_vfs_read_all!(vfs, &tmpdir, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!(
                "\nassert_vfs_read_all!: file doesn't exist or is not a file\n  target: \"{}\"\n",
                &tmpdir.display()
            )
        );

        let file = tmpdir.mash("foo");
        assert_vfs_write_all!(vfs, &file, b"foobar 1");
        assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_readlink()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let dir = tmpdir.mash("dir");
        let link = dir.mash("link");
        let file = tmpdir.mash("file");
        assert_vfs_mkdir_p!(vfs, &dir);
        assert_vfs_mkfile!(vfs, &file);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_readlink!(vfs, "", &file);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_readlink!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a symlink
        let result = testing::capture_panic(|| {
            assert_vfs_readlink!(vfs, &dir, &file);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!(
                "\nassert_vfs_readlink!: file doesn't exist or is not a symlink\n  target: \"{}\"\n",
                dir.display()
            )
        );

        // happy path
        assert_vfs_no_symlink!(vfs, &link);
        assert_vfs_symlink!(vfs, &link, &file);
        assert_vfs_is_symlink!(vfs, &link);
        assert_vfs_readlink!(vfs, &link, PathBuf::from("..").mash("file"));

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_readlink_abs()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let dir = tmpdir.mash("dir");
        let link = dir.mash("link");
        let file = tmpdir.mash("file");
        assert_vfs_mkdir_p!(vfs, &dir);
        assert_vfs_mkfile!(vfs, &file);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_readlink_abs!(vfs, "", &file);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_readlink_abs!: failed to get absolute path\n  target: \"\"\n"
        );
        let result = testing::capture_panic(|| {
            assert_vfs_readlink_abs!(vfs, &link, PathBuf::new());
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_readlink_abs!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a symlink
        let result = testing::capture_panic(|| {
            assert_vfs_readlink_abs!(vfs, &dir, &file);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!(
                "\nassert_vfs_readlink_abs!: file doesn't exist or is not a symlink\n  target: \"{}\"\n",
                dir.display()
            )
        );

        // happy path
        assert_vfs_no_symlink!(vfs, &link);
        assert_vfs_symlink!(vfs, &link, &file);
        assert_vfs_is_symlink!(vfs, &link);
        assert_vfs_readlink_abs!(vfs, &link, &file);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_remove()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let file1 = tmpdir.mash("file1");

        // happy path
        assert_vfs_remove!(vfs, &file1);
        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_is_file!(vfs, &file1);
        assert_vfs_remove!(vfs, &file1);
        assert_vfs_no_file!(vfs, &file1);

        // bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_remove!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_remove!: failed to get absolute path\n  target: \"\"\n"
        );

        // directory contains files
        assert_vfs_mkfile!(vfs, &file1);
        let result = testing::capture_panic(|| {
            assert_vfs_remove!(vfs, &tmpdir);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_remove!: failed removing directory\n  target: {:?}\n", &tmpdir)
        );

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_remove_all()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let file1 = tmpdir.mash("file1");

        assert_vfs_mkfile!(vfs, &file1);
        assert_vfs_is_file!(vfs, &file1);
        assert_vfs_remove_all!(vfs, &tmpdir);
        assert_vfs_no_dir!(vfs, &tmpdir);

        // bad abs
        let result = testing::capture_panic(|| {
            assert_vfs_remove_all!(vfs, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_remove_all!: failed to get absolute path\n  target: \"\"\n"
        );
    }

    #[test]
    fn test_assert_vfs_symlink()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let link1 = tmpdir.mash("link1");
        assert_vfs_mkdir_p!(vfs, &dir1);
        assert_vfs_mkfile!(vfs, &file1);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_symlink!(vfs, "", "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_symlink!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a symlink
        let result = testing::capture_panic(|| {
            assert_vfs_symlink!(vfs, &dir1, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_symlink!: is not a symlink\n  target: \"{}\"\n", dir1.display())
        );

        // happy path
        assert_vfs_no_symlink!(vfs, &link1);
        assert_vfs_symlink!(vfs, &link1, &file1);
        assert_vfs_is_symlink!(vfs, &link1);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_assert_vfs_write_all()
    {
        let (vfs, tmpdir) = assert_vfs_setup!(Vfs::memfs());

        // fail abs
        let result = testing::capture_panic(|| {
            assert_vfs_write_all!(vfs, "", "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_vfs_write_all!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a file
        let result = testing::capture_panic(|| {
            assert_vfs_write_all!(vfs, &tmpdir, "");
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_vfs_write_all!: is not a file\n  target: \"{}\"\n", &tmpdir.display())
        );

        let file = tmpdir.mash("foo");
        assert_vfs_write_all!(vfs, &file, b"foobar 1");
        assert_vfs_read_all!(vfs, &file, "foobar 1".to_string());

        assert_vfs_remove_all!(vfs, &tmpdir);
    }
}

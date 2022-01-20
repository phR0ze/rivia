use std::{
    panic,
    sync::{Arc, Mutex},
};

use lazy_static::lazy_static;

use crate::errors::*;

// Setup a simple counter to track if a custom panic handler should be used. Mutex is used to ensure
// a single thread is accessing the buffer at a time, but mutex itself is not thread safe so we
// wrap it in an Arc to provide that safety.
lazy_static! {
    static ref USE_PANIC_HANDLER: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
}

/// Capture any unwinding panics in a multi-thread safe way
///
/// Doesn't catch aborts, that may occur while executing the given closure. Any panics captured will
/// be converted into a RvResult with the SimpleError::Msg type returned containing the panic
/// output.
pub fn capture_panic(f: impl FnOnce()+panic::UnwindSafe) -> RvResult<()>
{
    {
        // Lock and increment the panic handler tracker within a block to trigger unlock
        let arc = USE_PANIC_HANDLER.clone();
        let mut count = arc.lock().map_err(|_| CoreError::PanicCaptureFailure)?;
        *count = *count + 1;
        panic::set_hook(Box::new(|_| {}));
    }

    // Run the given closure and capture the result
    let result = panic::catch_unwind(f);

    // Lock and decrement cleaning up the custom panic handler if down to 0
    let arc = USE_PANIC_HANDLER.clone();
    let mut count = arc.lock().map_err(|_| CoreError::PanicCaptureFailure)?;
    if *count != 0 {
        *count = *count - 1;
    }
    if *count == 0 {
        let _ = panic::take_hook();
    }

    // Return captured output
    if let Err(err) = result {
        if let Some(x) = err.downcast_ref::<&str>() {
            return Err(CoreError::panic_capture(x).into());
        } else if let Some(x) = err.downcast_ref::<String>() {
            return Err(CoreError::panic_capture(x).into());
        }
    }
    Ok(())
}

/// Create the test `setup` function to be called in tests to create unique directories to work in
/// for testing.
///
/// This call will modify files on disk. The intent is to provide a thread safe space from which to
/// manipulate files during a test.
///
/// `setup` accepts two arguments `root` and `func_name`. `root` and `func_name` are
/// joined as a path and treated as the directory path that will be created for
/// tests.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_stdfs_setup_func!();
/// assert_stdfs_setup!("tests/temp", "assert_stdfs_setup_func");
/// assert_stdfs_mkdir_p!("tests/temp/assert_stdfs_setup_func");
/// assert_stdfs_remove_all!("tests/temp/assert_stdfs_setup_func");
/// ```
#[macro_export]
macro_rules! assert_stdfs_setup_func {
    () => {
        fn setup<T: AsRef<Path>, U: AsRef<Path>>(root: T, func_name: U) -> PathBuf
        {
            // Validate the root path and function name
            if sys::is_empty(root.as_ref()) {
                panic_msg!("assert_stdfs_setup_func!", "root path is empty", root.as_ref());
            } else if sys::is_empty(func_name.as_ref()) {
                panic_msg!("assert_stdfs_setup_func!", "function name is empty", func_name.as_ref());
            }

            // Resolve absolute path of target
            let target = sys::mash(root.as_ref().to_owned(), func_name.as_ref());
            let target = match Stdfs::abs(&target) {
                Ok(x) => x,
                _ => panic_msg!("assert_stdfs_setup_func!", "failed to get absolute path", &target),
            };

            // Ensure the target has been removed
            if Stdfs::remove_all(&target).is_err() {
                panic_msg!("assert_stdfs_setup_func!", "failed while removing directory", &target);
            }

            // Create the target directory
            match Stdfs::mkdir_p(&target) {
                Ok(dir) => dir,
                _ => panic_msg!("assert_stdfs_setup_func!", "failed while creating directory", &target),
            }
        }
    };
}

/// Call the `setup` function created by `assert_stdfs_setup_func!`
///
/// Calls `assert_stdfs_setup_func!` with default `root` and `func_name` based on the function
/// context the setup function is run from or optionally override those values. `root` will default
/// to `TEST_TEMP_DIR` and `func_name` defaults to the function name using the `function!` macro. If
/// only one override is given it is assumed to be the `func_name` to be passed into the
/// `assert_stdfs_setup_func` function. If two parameters are given the first is assumed to be the
/// `root` and the second to be the `func_name`.
///
/// ### Warning
/// Since doc tests always have a default function name of `rust_out::main` its required to override
/// the `func_name` param to get a unique directory to work in as this is not possible by default.
///
/// ### Examples
/// Using the default `root` and `func_name` is fine if called from a named function
/// ```
/// use rivia::prelude::*;
///
/// assert_stdfs_setup_func!();
/// fn assert_stdfs_setup_default() {
///     let tmpdir = assert_stdfs_setup!();
///     assert_stdfs_mkdir!(&tmpdir);
///     assert_stdfs_eq!(
///         &tmpdir,
///         &PathBuf::from(TEST_TEMP_DIR).abs().unwrap().mash("assert_stdfs_setup_default")
///     );
///     assert_stdfs_remove_all!(&tmpdir);
/// }
/// assert_stdfs_setup_default();
/// ```
///
/// Doc tests don't have a named function and require the `func_name` param be overridden
/// ```
/// use rivia::prelude::*;
///
/// assert_stdfs_setup_func!();
/// let tmpdir = assert_stdfs_setup!("assert_stdfs_setup_custom_func");
/// assert_stdfs_mkdir!(&tmpdir);
/// assert_stdfs_eq!(
///     &tmpdir,
///     &PathBuf::from(TEST_TEMP_DIR).abs().unwrap().mash("assert_stdfs_setup_custom_func")
/// );
/// assert_stdfs_remove_all!(&tmpdir);
/// ```
///
/// `root` is treated as the first and `func_name` as the second when two params are given.
/// ```
/// use rivia::prelude::*;
///
/// assert_stdfs_setup_func!();
/// let tmpdir = assert_stdfs_setup!("tests/temp/assert_stdfs_setup_custom_root", "assert_stdfs_setup_custom_func");
/// assert_stdfs_mkdir!(&tmpdir);
/// assert_stdfs_eq!(
///     &tmpdir,
///     &PathBuf::from("tests/temp/assert_stdfs_setup_custom_root")
///         .abs()
///         .unwrap()
///         .mash("assert_stdfs_setup_custom_func")
/// );
/// assert_stdfs_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_stdfs_setup {
    () => {
        setup(testing::TEST_TEMP_DIR, function!())
    };
    ($func:expr) => {
        setup(testing::TEST_TEMP_DIR, $func)
    };
    ($root:expr, $func:expr) => {
        setup($root, $func)
    };
}

/// Assert that a file or directory exists
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_stdfs_setup_func!();
/// let tmpdir = assert_stdfs_setup!("assert_stdfs_exists");
/// assert_stdfs_exists!(&tmpdir);
/// assert_stdfs_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_stdfs_exists {
    ($path:expr) => {
        let target = match Stdfs::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_stdfs_exists!", "failed to get absolute path", $path),
        };
        if !Stdfs::exists(&target) {
            panic_msg!("assert_stdfs_exists!", "doesn't exist", &target);
        }
    };
}

/// Assert the given path doesn't exist
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_stdfs_no_exists!("tests/temp/assert_stdfs_no_exists");
/// assert_stdfs_no_exists!("tests/temp/assert_stdfs_no_exists/file");
/// ```
#[macro_export]
macro_rules! assert_stdfs_no_exists {
    ($path:expr) => {
        let target = match Stdfs::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_stdfs_no_exists!", "failed to get absolute path", $path),
        };
        if Stdfs::exists(&target) {
            panic_msg!("assert_stdfs_no_exists!", "still exists", &target);
        }
    };
}

/// Assert that the given path exists and is a directory
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_stdfs_setup_func!();
/// let tmpdir = assert_stdfs_setup!("assert_stdfs_is_dir");
/// assert_stdfs_is_dir!(&tmpdir);
/// assert_stdfs_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_stdfs_is_dir {
    ($path:expr) => {
        let target = match Stdfs::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_stdfs_is_dir!", "failed to get absolute path", $path),
        };
        if Stdfs::exists(&target) {
            if !Stdfs::is_dir(&target) {
                panic_msg!("assert_stdfs_is_dir!", "exists but is not a directory", &target);
            }
        } else {
            panic_msg!("assert_stdfs_is_dir!", "doesn't exist", &target);
        }
    };
}

/// Assert that the given path isn't a directory
///
/// ### Examples
/// ```
/// use fungus::prelude::*;
///
/// let tmpdir = PathBuf::from(TEST_TEMP_DIR).mash("assert_stdfs_no_dir");
/// assert_stdfs_no_dir!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_stdfs_no_dir {
    ($path:expr) => {
        let target = match Stdfs::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_stdfs_no_dir!", "failed to get absolute path", $path),
        };
        if Stdfs::exists(&target) {
            if !Stdfs::is_dir(&target) {
                panic_msg!("assert_stdfs_no_dir!", "exists and is not a directory", &target);
            } else {
                panic_msg!("assert_stdfs_no_dir!", "directory still exists", &target);
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
/// assert_stdfs_setup_func!();
/// let tmpdir = assert_stdfs_setup!("assert_stdfs_is_file");
/// let file = tmpdir.mash("file");
/// assert_stdfs_touch!(&file);
/// assert_stdfs_is_file!(&file);
/// assert_stdfs_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_stdfs_is_file {
    ($path:expr) => {
        let target = match Stdfs::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_stdfs_is_file!", "failed to get absolute path", $path),
        };
        if Stdfs::exists(&target) {
            if !Stdfs::is_file(&target) {
                panic_msg!("assert_stdfs_is_file!", "exists but is not a file", &target);
            }
        } else {
            panic_msg!("assert_stdfs_is_file!", "doesn't exist", &target);
        }
    };
}

/// Assert that the given path isn't a file
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_stdfs_no_file!("tests/temp/assert_stdfs_no_file/file");
/// ```
#[macro_export]
macro_rules! assert_stdfs_no_file {
    ($path:expr) => {
        let target = match Stdfs::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_stdfs_no_file!", "failed to get absolute path", $path),
        };
        if Stdfs::exists(&target) {
            if !Stdfs::is_file(&target) {
                panic_msg!("assert_stdfs_no_file!", "exists and is not a file", &target);
            } else {
                panic_msg!("assert_stdfs_no_file!", "file still exists", &target);
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
/// assert_stdfs_setup_func!();
/// let tmpdir = assert_stdfs_setup!("assert_stdfs_mkdir_p");
/// let dir1 = tmpdir.mash("dir1");
/// assert_stdfs_mkdir_p!(&dir1);
/// assert_stdfs_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_stdfs_mkdir_p {
    ($path:expr) => {
        let target = match Stdfs::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_stdfs_mkdir_p!", "failed to get absolute path", $path),
        };
        match Stdfs::mkdir_p(&target) {
            Ok(x) => {
                if &x != &target {
                    panic_compare_msg!("assert_stdfs_mkdir_p!", "created directory path doesn't match the target", &x, &target);
                }
            },
            Err(e) => panic!("assert_stdfs_mkdir_p!: {}", e.to_string()),
        };
        if !Stdfs::is_dir(&target) {
            panic_msg!("assert_stdfs_mkdir_p!", "failed to create directory", &target);
        }
    };
}

/// Assert the creation of a file. If the file exists no change is made.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_stdfs_setup_func!();
/// let tmpdir = assert_stdfs_setup!("assert_stdfs_touch");
/// let file1 = tmpdir.mash("file1");
/// assert_stdfs_no_file!(&file1);
/// assert_stdfs_touch!(&file1);
/// assert_stdfs_is_file!(&file1);
/// assert_stdfs_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_stdfs_touch {
    ($path:expr) => {
        let target = match Stdfs::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_stdfs_touch!", "failed to get absolute path", $path),
        };
        match Stdfs::touch(&target) {
            Ok(x) => {
                if &x != &target {
                    panic_compare_msg!("assert_stdfs_touch!", "created file path doesn't match the target", &x, &target);
                }
            },
            Err(e) => panic!("assert_stdfs_touch!: {}", e.to_string()),
        };
        if !Stdfs::is_file(&target) {
            panic_msg!("assert_stdfs_touch!", "file doesn't exist", &target);
        }
    };
}

/// Assert the removal of the target file
///
/// ### Assertion Failures
/// * Assertion fails if the target isn't a file
/// * Assertion fails if the file exists after `Stdfs::remove` is called
/// * Assertion fails if the `Stdfs::remove` call fails
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_stdfs_setup_func!();
/// let tmpdir = assert_stdfs_setup!("assert_stdfs_remove");
/// let file = tmpdir.mash("file");
/// assert_stdfs_touch!(&file);
/// assert_stdfs_remove!(&file);
/// assert_stdfs_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_stdfs_remove {
    ($path:expr) => {
        let target = match Stdfs::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_stdfs_remove!", "failed to get absolute path", $path),
        };
        if Stdfs::exists(&target) {
            if Stdfs::is_file(&target) {
                if Stdfs::remove(&target).is_err() {
                    panic_msg!("assert_stdfs_remove!", "failed removing file", &target);
                }
                if Stdfs::is_file(&target) {
                    panic_msg!("assert_stdfs_remove!", "file still exists", &target);
                }
            } else {
                panic_msg!("assert_stdfs_remove!", "exists and isn't a file", &target);
            }
        }
    };
}

/// Assert the removal of the target path
///
/// ### Assertion Failures
/// * Assertion fails if `Stdfs::remove_all` fails
/// * Assertion fails if the target path still exists after the call to `Stdfs::remove_all`
///
/// ### Examples
/// ```
/// use fungus::prelude::*;
///
/// assert_stdfs_setup_func!();
/// let tmpdir = assert_stdfs_setup!("assert_stdfs_remove_all");
/// assert_stdfs_exists!(&tmpdir);
/// assert_stdfs_remove_all!(&tmpdir);
/// assert_stdfs_no_exists!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_stdfs_remove_all {
    ($path:expr) => {
        let target = match Stdfs::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_stdfs_remove_all!", "failed to get absolute path", $path),
        };
        if Stdfs::remove_all(&target).is_err() {
            panic_msg!("assert_stdfs_remove_all!", "failed while removing", &target);
        }
        if Stdfs::exists(&target) {
            panic_msg!("assert_stdfs_remove_all!", "still exists", &target);
        }
    };
}

/// Helper function for testing to simply panic with the given message in a repeatable formatting.
///
/// ### Examples
/// ```ignore,no_run
/// use rivia::prelude::*;
///
/// panic_msg!("assert_stdfs_mkdir_p!", "failed to create directory", PathBuf::from("foo"));
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
/// panic_msg!("assert_stdfs_mkdir_p!", "failed to create directory", PathBuf::from("foo"), PathBuf::from("foo"));
/// ```
#[macro_export]
macro_rules! panic_compare_msg {
    ($name:expr, $msg:expr, $actual:expr, $target:expr) => {
        panic!("\n{}: {}\n  actual: {}\n  target: {}\n", $name, $msg, format!("{:?}", $actual), format!("{:?}", $target))
    };
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;
    assert_stdfs_setup_func!();

    #[test]
    fn test_assert_stdfs_exists_and_no_exists()
    {
        let tmpdir = assert_stdfs_setup!();

        // Test file exists
        {
            let file = sys::mash(&tmpdir, "file");
            assert_stdfs_no_exists!(&file);
            assert!(!Stdfs::exists(&file));
            assert_stdfs_touch!(&file);
            assert_stdfs_exists!(&file);
            assert!(Stdfs::exists(&file));

            assert_stdfs_remove!(&file);
            assert_stdfs_no_exists!(&file);
            assert!(!Stdfs::exists(&file));
        }

        // Test dir exists
        {
            let dir1 = sys::mash(&tmpdir, "dir1");
            assert_stdfs_no_exists!(&dir1);
            assert!(!Stdfs::exists(&dir1));
            assert_stdfs_mkdir_p!(&dir1);
            assert_stdfs_exists!(&dir1);
            assert!(Stdfs::exists(&dir1));

            assert_stdfs_remove_all!(&dir1);
            assert_stdfs_no_exists!(&dir1);
            assert!(!Stdfs::exists(&dir1));
        }

        // exists: bad abs
        let result = testing::capture_panic(|| {
            assert_stdfs_exists!("");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_exists!: failed to get absolute path\n  target: \"\"\n");

        // exists: doesn't exist
        let file1 = sys::mash(&tmpdir, "file1");
        let result = testing::capture_panic(|| {
            assert_stdfs_exists!(&file1);
        });
        assert_eq!(result.unwrap_err().to_string(), format!("\nassert_stdfs_exists!: doesn't exist\n  target: {:?}\n", &file1));

        // no exists: bad abs
        let result = testing::capture_panic(|| {
            assert_stdfs_no_exists!("");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_no_exists!: failed to get absolute path\n  target: \"\"\n");

        // no exists: does exist
        assert_stdfs_touch!(&file1);
        let result = testing::capture_panic(|| {
            assert_stdfs_no_exists!(&file1);
        });
        assert_eq!(result.unwrap_err().to_string(), format!("\nassert_stdfs_no_exists!: still exists\n  target: {:?}\n", &file1));

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_assert_stdfs_is_dir_no_dir()
    {
        let tmpdir = assert_stdfs_setup!();
        let dir1 = sys::mash(&tmpdir, "dir1");
        let dir2 = sys::mash(&tmpdir, "dir2");

        // happy path
        assert_stdfs_no_dir!(&dir1);
        assert!(!Stdfs::is_dir(&dir1));
        assert_stdfs_mkdir_p!(&dir1);
        assert_stdfs_is_dir!(&dir1);
        assert!(Stdfs::is_dir(&dir1));

        // is_dir: bad abs
        let result = testing::capture_panic(|| {
            assert_stdfs_is_dir!("");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_is_dir!: failed to get absolute path\n  target: \"\"\n");

        // is_dir: doesn't exist
        let result = testing::capture_panic(|| {
            assert_stdfs_is_dir!(&dir2);
        });
        assert_eq!(result.unwrap_err().to_string(), format!("\nassert_stdfs_is_dir!: doesn't exist\n  target: {:?}\n", &dir2));

        // no_dir: bad abs
        let result = testing::capture_panic(|| {
            assert_stdfs_no_dir!("");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_no_dir!: failed to get absolute path\n  target: \"\"\n");

        // no_dir: does exist
        let result = testing::capture_panic(|| {
            assert_stdfs_no_dir!(&dir1);
        });
        assert_eq!(result.unwrap_err().to_string(), format!("\nassert_stdfs_no_dir!: directory still exists\n  target: {:?}\n", &dir1));

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_assert_stdfs_is_file_no_file()
    {
        let tmpdir = assert_stdfs_setup!();
        let file1 = sys::mash(&tmpdir, "file1");
        let file2 = sys::mash(&tmpdir, "file2");

        // happy path
        assert_stdfs_no_file!(&file1);
        assert!(!Stdfs::is_file(&file1));
        assert_stdfs_touch!(&file1);
        assert_stdfs_is_file!(&file1);
        assert!(Stdfs::is_file(&file1));

        // is_file: bad abs
        let result = testing::capture_panic(|| {
            assert_stdfs_is_file!("");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_is_file!: failed to get absolute path\n  target: \"\"\n");

        // is_file: doesn't exist
        let result = testing::capture_panic(|| {
            assert_stdfs_is_file!(&file2);
        });
        assert_eq!(result.unwrap_err().to_string(), format!("\nassert_stdfs_is_file!: doesn't exist\n  target: {:?}\n", &file2));

        // no_file: bad abs
        let result = testing::capture_panic(|| {
            assert_stdfs_no_file!("");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_no_file!: failed to get absolute path\n  target: \"\"\n");

        // no_file: does exist
        let result = testing::capture_panic(|| {
            assert_stdfs_no_file!(&file1);
        });
        assert_eq!(result.unwrap_err().to_string(), format!("\nassert_stdfs_no_file!: file still exists\n  target: {:?}\n", &file1));

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_assert_stdfs_remove()
    {
        let tmpdir = assert_stdfs_setup!();
        let file1 = sys::mash(&tmpdir, "file1");

        // happy path
        assert_stdfs_remove!(&file1);
        assert_stdfs_touch!(&file1);
        assert_stdfs_is_file!(&file1);
        assert_stdfs_remove!(&file1);
        assert_stdfs_no_file!(&file1);

        // bad abs
        let result = testing::capture_panic(|| {
            assert_stdfs_remove!("");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_remove!: failed to get absolute path\n  target: \"\"\n");

        // is a directory
        let result = testing::capture_panic(|| {
            assert_stdfs_remove!(&tmpdir);
        });
        assert_eq!(result.unwrap_err().to_string(), format!("\nassert_stdfs_remove!: exists and isn't a file\n  target: {:?}\n", &tmpdir));

        // // fail to remove file
        // assert_stdfs_no_file!(&file1);
        // assert_stdfs_eq!(Stdfs::mkfile_m(&file1, 0o000).unwrap(), file1);
        // let result = testing::capture_panic(|| {
        //     assert_stdfs_remove!(&file1);
        // });
        // assert_stdfs_eq!(to_string(result), format!("\nassert_stdfs_remove!: failed removing
        // file\n target: {:?}\n",
        // &file1));
        // assert!(Stdfs::chmod(&file1, 0o777).is_ok());

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_assert_stdfs_mkdir_p()
    {
        let tmpdir = assert_stdfs_setup!();
        let file1 = sys::mash(&tmpdir, "file1");
        let dir1 = sys::mash(&tmpdir, "dir1");
        assert_stdfs_touch!(&file1);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_stdfs_mkdir_p!("");
        });

        // fail abs
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_mkdir_p!: failed to get absolute path\n  target: \"\"\n");

        // exists but not a directory
        let result = testing::capture_panic(|| {
            assert_stdfs_mkdir_p!(&file1);
        });
        assert_eq!(result.unwrap_err().to_string(), format!("assert_stdfs_mkdir_p!: is not a directory: {}", &file1.display()));

        // happy path
        assert_stdfs_no_dir!(&dir1);
        assert_stdfs_mkdir_p!(&dir1);
        assert_stdfs_is_dir!(&dir1);

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_assert_stdfs_touch()
    {
        let tmpdir = assert_stdfs_setup!();
        let file1 = sys::mash(&tmpdir, "file1");
        let dir1 = sys::mash(&tmpdir, "dir1");
        assert_stdfs_mkdir_p!(&dir1);

        // fail abs
        let result = testing::capture_panic(|| {
            assert_stdfs_touch!("");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_touch!: failed to get absolute path\n  target: \"\"\n");

        // exists but not a file
        let result = testing::capture_panic(|| {
            assert_stdfs_touch!(&dir1);
        });
        assert_eq!(result.unwrap_err().to_string(), format!("assert_stdfs_touch!: is not a file: {}", dir1.display()));

        // happy path
        assert_stdfs_no_file!(&file1);
        assert_stdfs_touch!(&file1);
        assert_stdfs_is_file!(&file1);

        assert_stdfs_remove_all!(&tmpdir);
    }

    #[test]
    fn test_assert_stdfs_remove_all()
    {
        let tmpdir = assert_stdfs_setup!();
        let file1 = sys::mash(&tmpdir, "file1");

        assert_stdfs_touch!(&file1);
        assert_stdfs_is_file!(&file1);
        assert_stdfs_remove_all!(&tmpdir);
        assert_stdfs_no_dir!(&tmpdir);

        // bad abs
        let result = testing::capture_panic(|| {
            assert_stdfs_remove_all!("");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_remove_all!: failed to get absolute path\n  target: \"\"\n");
    }

    #[test]
    fn test_assert_stdfs_setup()
    {
        // Defaults
        {
            let tmpdir = assert_stdfs_setup!();
            assert_stdfs_mkdir_p!(&tmpdir);
            assert_eq!(tmpdir, Stdfs::abs(sys::mash(&PathBuf::from(testing::TEST_TEMP_DIR), "test_assert_stdfs_setup")).unwrap());
            assert_stdfs_remove_all!(&tmpdir);
        }

        // Alternate func name
        {
            let func_name = "test_assert_stdfs_setup_alt_func";
            let tmpdir = assert_stdfs_setup!(&func_name);
            assert_stdfs_mkdir_p!(&tmpdir);
            assert_eq!(tmpdir, Stdfs::abs(sys::mash(&PathBuf::from(testing::TEST_TEMP_DIR), &func_name)).unwrap());
            assert_stdfs_remove_all!(&tmpdir);
        }

        // Alternate temp dir name and func name
        {
            let root = "tests/temp/test_assert_stdfs_setup_dir";
            let func_name = "test_assert_stdfs_setup_alt_func";
            let tmpdir = assert_stdfs_setup!(&root, &func_name);
            assert_stdfs_mkdir_p!(&tmpdir);
            assert_eq!(tmpdir, Stdfs::abs(sys::mash(&PathBuf::from(&root), &func_name)).unwrap());
            assert_stdfs_remove_all!(&root);
        }
    }

    #[test]
    fn test_assert_stdfs_setup_func()
    {
        // root path is empty
        let result = testing::capture_panic(|| {
            assert_stdfs_setup!("", "foo");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_setup_func!: root path is empty\n  target: \"\"\n");

        // func name is empty
        let result = testing::capture_panic(|| {
            assert_stdfs_setup!("foo", "");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_setup_func!: function name is empty\n  target: \"\"\n");

        // fail abs because of multiple home symbols
        let result = testing::capture_panic(|| {
            assert_stdfs_setup!("foo", "~~");
        });
        assert_eq!(result.unwrap_err().to_string(), "\nassert_stdfs_setup_func!: failed to get absolute path\n  target: \"foo/~~\"\n");

        // fail to remove directory
        let path = Stdfs::abs(sys::mash(PathBuf::from(testing::TEST_TEMP_DIR), "test_assert_stdfs_setup_func_perms")).unwrap();
        assert_eq!(Stdfs::mkdir_m(&path, 0o000).unwrap(), path); // no write priv
        assert_eq!(Stdfs::mode(&path).unwrap(), 0o40000);
        let result = testing::capture_panic(|| {
            assert_stdfs_setup!(testing::TEST_TEMP_DIR, sys::name(&path).unwrap());
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_stdfs_setup_func!: failed while removing directory\n  target: {:?}\n", path)
        );
        assert!(Stdfs::chmod(&path, 0o777).is_ok());
        assert!(Stdfs::remove_all(&path).is_ok());
    }
}

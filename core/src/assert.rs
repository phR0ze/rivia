//! Provides a set of testing macros to reduce test boiler plate
//!
//! ## For testing only
//! All code in this module should only ever be used in testing and not in production.
use crate::{sys, errors::*};
use lazy_static::lazy_static;
use std::{
    panic,
    sync::{Arc, Mutex},
};

pub const TEST_TEMP_DIR: &str = "tests/temp";

// Setup a simple counter to track if a custom panic handler should be used. Mutex is used to ensure
// a single thread is accessing the buffer at a time, but mutex itself is not thread safe so we
// wrap it in an Arc to provide that safety.
lazy_static! {
    static ref USE_PANIC_HANDLER: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
}

/// Capture any unwinding panics, i.e. doesn't catch aborts, that may occur while executing the
/// given closure. Any panics captured will be converted into a FnResult with the SimpleError::Msg
/// type returned containing the panic output. This function is multi-thread safe.
pub fn capture_panic(f: impl FnOnce()+panic::UnwindSafe) -> RvResult<()> {
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
/// for testing that depends on modifying files on disk. The intent is to provide a thread safe
/// space from which to manipulate files during a test.
///
/// `setup` accepts two arguments `root` and `func_name`. `root` and `func_name` are
/// joined as a path and treated as the directory path that will be created for
/// tests.
///
/// ### Examples
/// ```
/// use rivia_core::*;
///
/// assert_setup_func!();
/// assert_setup!("tests/temp", "assert_setup_func");
/// assert_mkdir_p!("tests/temp/assert_setup_func");
/// assert_remove_all!("tests/temp/assert_setup_func");
/// ```
#[macro_export]
macro_rules! assert_setup_func {
    () => {
        fn setup<T: AsRef<Path>, U: AsRef<Path>>(root: T, func_name: U) -> PathBuf {
            // Validate the root path and function name
            if sys::is_empty(root.as_ref()) {
                panic_msg!("assert_setup_func!", "root path is empty", root.as_ref());
            } else if sys::is_empty(func_name.as_ref()) {
                panic_msg!("assert_setup_func!", "function name is empty", func_name.as_ref());
            }

            // Resolve absolute path of target
            let target = sys::mash(root.as_ref().to_owned(), func_name.as_ref());
            let target = match sys::abs(&target) {
                Ok(x) => x,
                _ => panic_msg!("assert_setup_func!", "failed to get absolute path", &target),
            };

            // Ensure the target has been removed
            if sys::remove_all(&target).is_err() {
                panic_msg!("assert_setup_func!", "failed while removing directory", &target);
            }

            // Create the target directory
            match sys::mkdir_p(&target) {
                Ok(dir) => dir,
                _ => panic_msg!("assert_setup_func!", "failed while creating directory", &target),
            }
        }
    };
}

/// Call the `setup` function created by `assert_setup_func!` with default `root` and `func_name`
/// based on the function context the setup function is run from or optionally override those
/// values. `root` will default to `TEST_TEMP_DIR` and `func_name` defaults to the function name
/// using the `function!` macro. If only one override is given it is assumed to be the `func_name`
/// to be passed into the `assert_setup_func` function. If two parameters are given the first is
/// assumed to be the `root` and the second to be the `func_name`.
///
/// WARNING: since doc tests always have a default function name of `rust_out::main` its required
/// to override the `func_name` param to get a unique directory to work in as this is not possible
/// by default.
///
/// ## Examples
///
/// ### Using the default `root` and `func_name` is fine if called from a named function
/// ```
/// use rivia_core::*;
///
/// assert_setup_func!();
/// fn assert_setup_default() {
///     let tmpdir = assert_setup!();
///     assert_mkdir!(&tmpdir);
///     assert_eq!(
///         &tmpdir,
///         &PathBuf::from(TEST_TEMP_DIR).abs().unwrap().mash("assert_setup_default")
///     );
///     assert_remove_all!(&tmpdir);
/// }
/// assert_setup_default();
/// ```
///
/// ### Doc tests don't have a named function and require the `func_name` param be overridden
/// ```
/// use rivia_core::*;
///
/// assert_setup_func!();
/// let tmpdir = assert_setup!("assert_setup_custom_func");
/// assert_mkdir!(&tmpdir);
/// assert_eq!(
///     &tmpdir,
///     &PathBuf::from(TEST_TEMP_DIR).abs().unwrap().mash("assert_setup_custom_func")
/// );
/// assert_remove_all!(&tmpdir);
/// ```
///
/// ### `root` is treated as the first and `func_name` as the second when two params are given.
/// ```
/// use rivia_core::*;
///
/// assert_setup_func!();
/// let tmpdir = assert_setup!("tests/temp/assert_setup_custom_root", "assert_setup_custom_func");
/// assert_mkdir!(&tmpdir);
/// assert_eq!(
///     &tmpdir,
///     &PathBuf::from("tests/temp/assert_setup_custom_root")
///         .abs()
///         .unwrap()
///         .mash("assert_setup_custom_func")
/// );
/// assert_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_setup {
    () => {
        setup(TEST_TEMP_DIR, function!())
    };
    ($func:expr) => {
        setup(TEST_TEMP_DIR, $func)
    };
    ($root:expr, $func:expr) => {
        setup($root, $func)
    };
}

/// Assert that a file or directory exists
///
/// ### Examples
/// ```
/// use rivia_core::*;
///
/// assert_setup_func!();
/// let tmpdir = assert_setup!("assert_exists");
/// assert_exists!(&tmpdir);
/// assert_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_exists {
    ($path:expr) => {
        let target = match sys::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_exists!", "failed to get absolute path", $path),
        };
        if !sys::exists(&target) {
            panic_msg!("assert_exists!", "doesn't exist", &target);
        }
    };
}

/// Assert the given path doesn't exist
///
/// ### Examples
/// ```
/// use rivia_core::*;
///
/// assert_no_exists!("tests/temp/assert_no_exists");
/// assert_no_exists!("tests/temp/assert_no_exists/file");
/// ```
#[macro_export]
macro_rules! assert_no_exists {
    ($path:expr) => {
        let target = match sys::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_no_exists!", "failed to get absolute path", $path),
        };
        if sys::exists(&target) {
            panic_msg!("assert_no_exists!", "still exists", &target);
        }
    };
}

/// Assert that the given path exists and is a directory
///
/// ### Examples
/// ```
/// use rivia_core::*;
///
/// assert_setup_func!();
/// let tmpdir = assert_setup!("assert_is_dir");
/// assert_is_dir!(&tmpdir);
/// assert_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_is_dir {
    ($path:expr) => {
        let target = match sys::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_is_dir!", "failed to get absolute path", $path),
        };
        match sys::is_dir(&target) {
            Ok() => panic_msg!("assert_is_dir!", "exists but is not a directory", &target);
        }
        } else {
            panic_msg!("assert_is_dir!", "doesn't exist", &target);
        }
    };
}

/// Assert that the given path isn't a directory
///
/// ### Examples
/// ```
/// use fungus::prelude::*;
///
/// let tmpdir = PathBuf::from(TEST_TEMP_DIR).mash("assert_no_dir");
/// assert_no_dir!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_no_dir {
    ($path:expr) => {
        let target = match sys::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_no_dir!", "failed to get absolute path", $path),
        };
        if sys::exists(&target) {
            if !sys::is_dir(&target) {
                panic_msg!("assert_no_dir!", "exists and is not a directory", &target);
            } else {
                panic_msg!("assert_no_dir!", "directory still exists", &target);
            }
        }
    };
}

/// Assert that the given path exists and is a file
///
/// ### Examples
/// ```
/// use rivia_core::*;
///
/// assert_setup_func!();
/// let tmpdir = assert_setup!("assert_is_file");
/// let file = tmpdir.mash("file");
/// assert_mkfile!(&file);
/// assert_is_file!(&file);
/// assert_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_is_file {
    ($path:expr) => {
        let target = match sys::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_is_file!", "failed to get absolute path", $path),
        };
        match sys::is_file(&target) {
            Ok() if !sys::is_file(&target) {
                panic_msg!("assert_is_file!", "exists but is not a file", &target);
            }
        } else {
            panic_msg!("assert_is_file!", "doesn't exist", &target);
        }
        Err(e) => panic_msg!("assert_mkdir_p!", e.to_string(), &target),
    };
}

/// Assert the creation of the given directory.
///
/// ### Examples
/// ```
/// use rivia_core::*;
///
/// assert_setup_func!();
/// let tmpdir = assert_setup!("assert_mkdir_p");
/// let dir1 = tmpdir.mash("dir1");
/// assert_mkdir_p!(&dir1);
/// assert_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_mkdir_p {
    ($path:expr) => {
        let target = match sys::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_mkdir_p!", "failed to get absolute path", $path),
        };
        match sys::mkdir_p(&target) {
            Ok(x) => {
                if &x != &target {
                    panic_compare_msg!(
                        "assert_mkdir_p!",
                        "created directory path doesn't match the target",
                        &x,
                        &target
                    );
                }
            },
            Err(e) => panic_msg!("assert_mkdir_p!", e.to_string(), &target),
        };
        if !sys::is_dir(&target) {
            panic_msg!("assert_mkdir_p!", "failed to create directory", &target);
        }
    };
}

/// Assert the creation of a file. If the file exists no change is made.
///
/// ### Examples
/// ```
/// use rivia_core::*;
///
/// assert_setup_func!();
/// let tmpdir = assert_setup!("assert_mkfile");
/// let file1 = tmpdir.mash("file1");
/// assert_no_file!(&file1);
/// assert_mkfile!(&file1);
/// assert_is_file!(&file1);
/// assert_remove_all!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_mkfile {
    ($path:expr) => {
        let target = match sys::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_mkfile!", "failed to get absolute path", $path),
        };
        match sys::mkfile(&target) {
            Ok(x) => {
                if &x != &target {
                    panic_compare_msg!(
                        "assert_mkfile!",
                        "created file path doesn't match the target",
                        &x,
                        &target
                    );
                }
            },
            Err(e) => panic_msg!("assert_mkfile!", e.to_string(), &target),
        };
        if !sys::is_file(&target) {
            panic_msg!("assert_mkfile!", "file doesn't exist", &target);
        }
    };
}

/// Assert the removal of the target path. Assertion fails if `sys::remove_all` fails or the target
/// path still exists after the call to `sys::remove_all`.
///
/// ### Examples
/// ```
/// use fungus::prelude::*;
///
/// assert_setup_func!();
/// let tmpdir = assert_setup!("assert_remove_all");
/// assert_exists!(&tmpdir);
/// assert_remove_all!(&tmpdir);
/// assert_no_exists!(&tmpdir);
/// ```
#[macro_export]
macro_rules! assert_remove_all {
    ($path:expr) => {
        let target = match sys::abs($path) {
            Ok(x) => x,
            _ => panic_msg!("assert_remove_all!", "failed to get absolute path", $path),
        };
        if sys::remove_all(&target).is_err() {
            panic_msg!("assert_remove_all!", "failed while removing", &target);
        }
        if sys::exists(&target) {
            panic_msg!("assert_remove_all!", "still exists", &target);
        }
    };
}

/// Helper function for testing to simply panic with the given message in a repeatable formatting.
///
/// ### Examples
/// ```ignore,no_run
/// use rivia_core::*;
///
/// panic_msg!("assert_mkdir_p!", "failed to create directory", PathBuf::from("foo"));
/// ```
#[macro_export]
macro_rules! panic_msg {
    ($name:expr, $msg:expr, $target:expr) => {
        panic!(
            "\n{}: {}\n  target: {}\n",
            $name,
            $msg,
            format!("{:?}", $target)
        )
    };
}

/// Helper function for testing to simply panic with the given message in a repeatable formatting.
///
/// ### Examples
/// ```ignore,no_run
/// use rivia_core::*;
///
/// panic_msg!("assert_mkdir_p!", "failed to create directory", PathBuf::from("foo"), PathBuf::from("foo"));
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
    use crate::*;
    assert_setup_func!();

    #[test]
    fn test_assert_mkdir_p()
    {
        let tmpdir = assert_setup!();
        let file1 = sys::mash(&tmpdir, "file1");
        let dir1 = sys::mash(&tmpdir, "dir1");
        assert_mkfile!(&file1);

        // fail abs
        let result = capture_panic(|| {
            assert_mkdir_p!("");
        });

        // fail abs
        assert_eq!(
            result.unwrap_err().to_string(),
            "\nassert_mkdir_p!: failed to get absolute path\n  target: \"\"\n"
        );

        // exists but not a directory
        let result = capture_panic(|| {
            assert_mkdir_p!(&file1);
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("\nassert_mkdir_p!: is not a directory: {}\n  target: {:?}\n", &file1.display(), &file1)
        );

        // happy path
        assert_no_dir!(&dir1);
        assert_mkdir_p!(&dir1);
        assert_is_dir!(&dir1);

        assert_remove_all!(&tmpdir);
    }
}
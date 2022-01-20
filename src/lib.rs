//! Provides essential macros and extensions to fill in gaps in Rust ergonomics and reduce the
//! amount of boiler plate code required for common tasks. The intent is to provide this while
//! keeping dependencies to a minimum.
//!
//! ## Provides
//!
//! ### Virtual FileSystem (VFS)
//! Using the power of Rust enums we can create a simple virtual file system with multiple backend 
//! implementations that can easily be switched out at compile time for testing purposes with 
//! almost zero additional overhead. By passing in a vfs reference to functions needing to 
//! manipulate the filesystem you can easily change the backing implementation. For those wishing 
//! for a truely seamless experience see the `rivia-vfs` crate for a global singleton that can be 
//! dynamically updated at runtime thus avoiding passing a vfs reference around.
//! ```
//! use rivia::prelude::*;
//!
//! let vfs = Vfs::stdfs();         // create the vfs as a standard filesystem
//! let vfs = Vfs::memfs();         // easily switch to a memory filesystem
//! vfs.mkdir_p("foo").unwrap();    // operations remain seamless
//! ```
//!
//! ## Rejected Considerations
//!
//! ### VfsPath
//! `VfsPath` was considered as a way to track and not repeat environment variable and absolute path
//! resolution. Additionally it would be used in the VFS case to track the VFS backend being used to
//! allow for extensions to a Path to chain call additional operations. However the relative amount
//! of overhead of resolving paths is small when compared to disk IO which will usually be being
//! done. What's more the benefit of trackig the VFS for chain calling is dubious when you introduce
//! the complexity of multiple ways to get access to VFS operations. Considering the ergonomic
//! impact and commplexity of such a change. I won't be implementing a notion of a `VfsPath` in
//! favor of a single point of entry into the VFS operations and much cleaner ergonomics.
//!
//! ### Using Rivia
//! ```
//! use rivia::prelude::*;
//! ```
#[macro_use]
pub mod testing;
#[macro_use]
pub mod exts;

pub mod errors;
pub mod sys;

/// All essential symbols in a simple consumable way
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
/// ```
pub mod prelude
{
    // Re-exports
    pub use std::{
        io::{Read, Seek, SeekFrom, Write},
        path::{Path, PathBuf},
    };

    // Export macros by name
    pub use crate::{
        assert_stdfs_exists, assert_stdfs_is_dir, assert_stdfs_is_file, assert_stdfs_mkdir_p, assert_stdfs_no_dir, assert_stdfs_no_exists,
        assert_stdfs_no_file, assert_stdfs_remove, assert_stdfs_remove_all, assert_stdfs_setup, assert_stdfs_setup_func,
        assert_stdfs_touch, cfgblock, function, trying, unwrap_or_false,
    };
    // Export internal types
    pub use crate::{
        errors::*,
        exts::*,
        sys::{self, Entry, FileSystem, Memfs, MemfsEntry, PathExt, Stdfs, StdfsEntry, Vfs, VfsEntry},
        testing,
    };
}

/// Provides the ability to define `#[cfg]` statements for multiple items
///
/// ### Examples
/// ```ignore
/// use rivia::prelude::*;
///
/// cfgblk! {
///     #[cfg(feature = "foo")])
///     use libc;
///     use std::ffi::CString;
/// }
///
/// // Expands to
/// #[cfg(feature = "foo")])
/// use libc;
/// #[cfg(feature = "foo")])
/// use std::ffi::CString;
/// ```
#[macro_export]
macro_rules! cfgblock {

    // Handle a single item
    (#[$attr:meta] $item:item) => {
        #[$attr] $item
    };

    // Handle more than one item recursively
    (#[$attr:meta] $($tail:item)*) => {
        $(cfgblock!{#[$attr] $tail})*
    };
}

/// Expands to a string literal of the current function's name similar to the venerable `file!` or
/// `line!`.
///
/// ### References
/// * https://github.com/rust-lang/rfcs/pull/1719.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// fn my_func() -> &'static str {
///     function!()
/// }
/// assert_eq!(my_func(), "my_func");
/// ```
#[macro_export]
macro_rules! function {
    () => {{
        // Capture the function's type and passes it to `std::any::type_name` to get the
        // function's fully qualified name, which includes our target.
        // https://doc.rust-lang.org/std/any/fn.type_name.html
        fn _f() {}
        fn type_of<T>(_: T) -> &'static str
        {
            std::any::type_name::<T>()
        }

        // Capture the fully qualified name
        let fqn = type_of(_f);

        // Trim off the suffix
        let fqn = &fqn[..fqn.len() - 4];

        // Trim off the prefix if it exists
        match fqn.rfind(':') {
            Some(i) => &fqn[i + 1..],
            None => &fqn,
        }
    }};
}

/// Return an iterator Err type conveniently
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// fn trying_func<T: AsRef<Path>>(path: T) -> Option<FnResult<PathBuf>> {
///     Some(Ok(trying!(path.as_ref().abs())))
/// }
/// assert_eq!(trying_func("").unwrap().unwrap_err().to_string(), "path empty");
/// ```
#[macro_export]
macro_rules! trying {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(err) => return Some(Err(From::from(err))),
        }
    };
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[test]
    fn test_function_macro()
    {
        fn indirect_func_name() -> &'static str
        {
            function!()
        }
        assert_eq!(function!(), "test_function_macro");
        assert_eq!(indirect_func_name(), "indirect_func_name");
    }

    #[test]
    fn test_trying_macro()
    {
        fn trying_func<T: AsRef<Path>>(path: T) -> Option<RvResult<PathBuf>>
        {
            Some(Ok(trying!(Stdfs::abs(path.as_ref()))))
        }
        assert_eq!(trying_func("").unwrap().unwrap_err().to_string(), "path empty");
    }
}

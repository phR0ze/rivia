//! Provides general extensions to common types like [`Option`], [`Result`] and [`Iterator`].
//!
//! ### Using Rivia extensions
//! ```
//! use rivia::prelude::*;
//! ```
#[macro_use]
mod result;

mod iter;
mod option;
mod peekable;
mod string;

pub use iter::*;
pub use option::*;
pub use peekable::*;
pub use result::*;
pub use string::*;

/// Expands to the current function's name similar to the venerable `file!` or `line!`
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

        // Trim off the suffix i.e. ::_f
        let fqn = &fqn[..fqn.len() - 4];

        // Trim off the prefix if it exists
        match fqn.rfind(':') {
            Some(i) => &fqn[i + 1..],
            None => &fqn,
        }
    }};
}

/// Expands to the current functions's fully qualified name
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
macro_rules! function_fqn {
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

        // Trim off the suffix i.e. ::_f
        let fqn = &fqn[..fqn.len() - 4];
        fqn
    }};
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
    fn test_function_fqn_macro()
    {
        fn indirect_fqn() -> &'static str
        {
            function_fqn!()
        }
        assert_eq!(function_fqn!(), "rivia::exts::tests::test_function_fqn_macro");
        assert_eq!(indirect_fqn(), "rivia::exts::tests::test_function_fqn_macro::indirect_fqn");
    }

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

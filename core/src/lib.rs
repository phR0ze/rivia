//! `rivia-core` provides essential macros and extensions to fill in gaps in Rust ergonomics
//! and reduce the amount of boiler plate code required for common tasks. The intent is to
//! provide this while keeping dependencies to a minimum.
//!
//! ### Using the `core` crate
//! ```
//! use rivia_core::*;
//! ```
#[macro_use]
mod assert;
mod errors;
mod iter;
mod option;
mod peekable;
mod string;

// Export module directly
pub mod sys;

// Export internal types
pub use assert::*;
pub use errors::*;
pub use iter::*;
pub use option::*;
pub use peekable::*;
pub use string::*;

// Re-exports
pub use std::path::{Path, PathBuf};

/// Provides the ability to define `#[cfg]` statements for multiple items
///
/// ### Examples
/// ```ignore
/// use rivia_core::*;
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

/// Expands to a string literal of the current function's name similar to the
/// venerable `file!` or `line!` https://github.com/rust-lang/rfcs/pull/1719.
///
/// ### Examples
/// ```
/// use rivia_core::*;
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
        fn type_of<T>(_: T) -> &'static str {
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

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_function_macro() {
        fn indirect_func_name() -> &'static str {
            function!()
        }
        assert_eq!(function!(), "test_function_macro");
        assert_eq!(indirect_func_name(), "indirect_func_name");
    }
}

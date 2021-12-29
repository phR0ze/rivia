//! Provides essential macros and extension traits
//!
//! ### Using the `core` crate
//! ```
//! use rivia-core::*;
//! ```
mod error;
mod iter;
mod iter_error;
mod option;
mod path;
mod path_error;
mod string;
mod string_error;

// Export module directly
pub mod sys;

// Export internal types
pub use error::*;
pub use iter::*;
pub use iter_error::*;
pub use option::*;
pub use path::*;
pub use path_error::*;
pub use string::*;
pub use string_error::*;

//! Provides essential macros and extension traits
//!
//! ### Using the `core` crate
//! ```
//! use rivia-core::*;
//! ```
mod error;
mod path_error;
mod string;
mod string_error;

// Export internal types
pub use error::*;
pub use path_error::*;
pub use string::*;
pub use string_error::*;

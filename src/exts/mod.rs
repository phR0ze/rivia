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

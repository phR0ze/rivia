//! Provides a set of testing functions and macros to reduce testing boiler plate
//!
//! ## For testing only
//! All code in this module should only ever be used in testing and not in production.
//!
//! ### How to use the Rivia `testing` module
//! ```
//! use rivia::prelude::*;
//! ```
#[macro_use]
mod assert;
use std::{
    panic,
    sync::{Arc, Mutex},
};

pub use assert::vfs_setup;
pub use assert::vfs_setup_p;
use lazy_static::lazy_static;

use crate::errors::*;

/// Defines the `tests/temp` location in the current project for file based testing if required
pub const TEST_TEMP_DIR: &str = "tests/temp";

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

/// Ensure the given closure is executed once the surrounding scope closes
///
/// * Use the `defer!` macro for a more ergonomic experience
/// * Triggered despite panics
/// * Inspired by Golang's `defer`, Java's finally and Ruby's `ensure`.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// let file = vfs.root().mash("file");
/// assert_vfs_mkfile!(vfs, &file);
///
/// // Create a scope that will trigger defer's destructor
/// {
///     let _defer = defer(|| vfs.remove(&file).unwrap());
/// }
/// assert_vfs_no_exists!(vfs, &file);
/// ```
pub fn defer<T: FnMut()>(f: T) -> impl Drop
{
    Defer(f)
}

/// Provides a means of ensuring a given closure is executed once the surrounding scope closes
///
/// This mechanism is inspired by Golang's `defer` but is similar to Java's finally and Ruby's
/// `ensure`. By creating a new [`Defer`] type that wraps a `FnMut` and implements `Drop` we
/// can execute the captured closure during when the `drop` is executed.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Vfs::memfs();
/// let file = vfs.root().mash("file");
/// assert_vfs_mkfile!(vfs, &file);
///
/// // Create a scope that will trigger defer's destructor
/// {
///     defer!(vfs.remove(&file).unwrap());
/// }
/// assert_vfs_no_exists!(vfs, &file);
/// ```
pub struct Defer<T: FnMut()>(T);
impl<T: FnMut()> Drop for Defer<T>
{
    fn drop(&mut self)
    {
        (self.0)();
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use std::{
        cell::Cell,
        panic::{self, catch_unwind, AssertUnwindSafe},
    };

    use crate::prelude::*;

    // Registers a panic hook that does nothing to supress the panic output
    // that get dumped to the screen regardless of panic handling with catch_unwind
    fn supress_panic_err()
    {
        panic::set_hook(Box::new(|_| {}));
    }

    #[test]
    fn test_defer_fires_even_with_panic()
    {
        supress_panic_err();

        let obj = Cell::new(1);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            defer!(obj.set(2));
            panic!();
        }));
        assert_eq!(obj.get(), 2);
    }

    #[test]
    fn test_defer_actually_waits_until_scope_closes_end()
    {
        let obj = Cell::new(1);
        {
            defer!(obj.set(2));
        }
        defer!(obj.set(3));
        assert_eq!(obj.get(), 2);
    }
}

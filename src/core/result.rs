/// Unwrap the value on Ok or return false on Err
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// fn test_unwrap_or_false(vfs: Vfs, path: PathBuf) -> bool
/// {
///     let abs = unwrap_or_false!(vfs.abs(path));
///     assert_eq!(abs, vfs.root().mash("foo"));
///     true
/// }
///
/// let vfs = Vfs::memfs();
/// assert_eq!(test_unwrap_or_false(vfs, PathBuf::from("foo")), true);
/// ```
#[macro_export]
macro_rules! unwrap_or_false {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(_) => return false,
        }
    };
}

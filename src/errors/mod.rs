//! Provides a common set of errors across the rivia crates to reduce the verbosity of error
//! handling
//!
//! ### Using Rivia errors
//! ```
//! use rivia::prelude::*;
//!
//! let mut err = RvError::from(std::env::VarError::NotPresent);
//! assert!(err.downcast_ref::<std::env::VarError>().is_some());
//! assert!(err.downcast_mut::<std::env::VarError>().is_some());
//! assert!(err.source().is_none());
//! ```
mod core;
mod file;
mod iter;
mod path;
mod string;
mod user;
mod vfs;

use std::{error::Error as StdError, fmt, io, time::SystemTimeError};

pub use file::*;
pub use iter::*;
pub use path::*;
pub use string::*;
pub use user::*;
pub use vfs::*;

pub use self::core::*;

/// Provides a simplified Rivia result type with a common Rivia error type
pub type RvResult<T> = std::result::Result<T, RvError>;

/// An error that provides a common error for Rivia wrapping other internal errors
#[derive(Debug)]
pub enum RvError {
    /// Core error
    Core(CoreError),

    /// File error
    File(FileError),

    /// An io error
    Io(io::Error),

    /// A interator error
    Iter(IterError),

    /// Nix low level error
    Nix(nix::errno::Errno),

    /// A pathing error
    Path(PathError),

    /// A string error
    String(StringError),

    /// A system time error
    SystemTime(SystemTimeError),

    /// A user errro
    User(UserError),

    /// An internal Utf8 error
    Utf8(std::str::Utf8Error),

    /// Environment variable error
    Var(std::env::VarError),

    /// Virtul File System errror
    Vfs(VfsError),
}

impl RvError {
    /// Implemented directly on the `Error` type to reduce casting required
    pub fn is<T: StdError + 'static>(&self) -> bool {
        self.as_ref().is::<T>()
    }

    /// Implemented directly on the `Error` type to reduce casting required
    pub fn downcast_ref<T: StdError + 'static>(&self) -> Option<&T> {
        self.as_ref().downcast_ref::<T>()
    }

    /// Implemented directly on the `Error` type to reduce casting required
    pub fn downcast_mut<T: StdError + 'static>(&mut self) -> Option<&mut T> {
        self.as_mut().downcast_mut::<T>()
    }

    /// Implemented directly on the `Error` type to reduce casting required
    /// which allows for using as_ref to get the correct pass through.
    pub fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.as_ref().source()
    }
}
impl StdError for RvError {}

impl fmt::Display for RvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RvError::Core(ref err) => write!(f, "{}", err),
            RvError::File(ref err) => write!(f, "{}", err),
            RvError::Io(ref err) => write!(f, "{}", err),
            RvError::Iter(ref err) => write!(f, "{}", err),
            RvError::Nix(ref err) => write!(f, "{}", err),
            RvError::Path(ref err) => write!(f, "{}", err),
            RvError::String(ref err) => write!(f, "{}", err),
            RvError::SystemTime(ref err) => write!(f, "{}", err),
            RvError::User(ref err) => write!(f, "{}", err),
            RvError::Utf8(ref err) => write!(f, "{}", err),
            RvError::Var(ref err) => write!(f, "{}", err),
            RvError::Vfs(ref err) => write!(f, "{}", err),
        }
    }
}

impl AsRef<dyn StdError> for RvError {
    fn as_ref(&self) -> &(dyn StdError + 'static) {
        match *self {
            RvError::Core(ref err) => err,
            RvError::File(ref err) => err,
            RvError::Io(ref err) => err,
            RvError::Iter(ref err) => err,
            RvError::Nix(ref err) => err,
            RvError::Path(ref err) => err,
            RvError::String(ref err) => err,
            RvError::SystemTime(ref err) => err,
            RvError::User(ref err) => err,
            RvError::Utf8(ref err) => err,
            RvError::Var(ref err) => err,
            RvError::Vfs(ref err) => err,
        }
    }
}

impl AsMut<dyn StdError> for RvError {
    fn as_mut(&mut self) -> &mut (dyn StdError + 'static) {
        match *self {
            RvError::Core(ref mut err) => err,
            RvError::File(ref mut err) => err,
            RvError::Io(ref mut err) => err,
            RvError::Iter(ref mut err) => err,
            RvError::Nix(ref mut err) => err,
            RvError::Path(ref mut err) => err,
            RvError::String(ref mut err) => err,
            RvError::SystemTime(ref mut err) => err,
            RvError::User(ref mut err) => err,
            RvError::Utf8(ref mut err) => err,
            RvError::Var(ref mut err) => err,
            RvError::Vfs(ref mut err) => err,
        }
    }
}

impl From<CoreError> for RvError {
    fn from(err: CoreError) -> RvError {
        RvError::Core(err)
    }
}

impl From<FileError> for RvError {
    fn from(err: FileError) -> RvError {
        RvError::File(err)
    }
}

impl From<io::Error> for RvError {
    fn from(err: io::Error) -> RvError {
        RvError::Io(err)
    }
}

impl From<IterError> for RvError {
    fn from(err: IterError) -> RvError {
        RvError::Iter(err)
    }
}

impl From<nix::errno::Errno> for RvError {
    fn from(err: nix::errno::Errno) -> RvError {
        RvError::Nix(err)
    }
}

impl From<PathError> for RvError {
    fn from(err: PathError) -> RvError {
        RvError::Path(err)
    }
}

impl From<StringError> for RvError {
    fn from(err: StringError) -> RvError {
        RvError::String(err)
    }
}

impl From<&str> for RvError {
    fn from(err: &str) -> RvError {
        RvError::Core(CoreError::msg(err))
    }
}

impl From<SystemTimeError> for RvError {
    fn from(err: SystemTimeError) -> RvError {
        RvError::SystemTime(err)
    }
}

impl From<UserError> for RvError {
    fn from(err: UserError) -> RvError {
        RvError::User(err)
    }
}

impl From<std::str::Utf8Error> for RvError {
    fn from(err: std::str::Utf8Error) -> RvError {
        RvError::Utf8(err)
    }
}

impl From<std::env::VarError> for RvError {
    fn from(err: std::env::VarError) -> RvError {
        RvError::Var(err)
    }
}

impl From<VfsError> for RvError {
    fn from(err: VfsError) -> RvError {
        RvError::Vfs(err)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::errors::*;

    #[test]
    fn test_error() {
        let mut err = RvError::from(CoreError::msg("foo"));
        assert_eq!(err.to_string(), "foo");
        assert_eq!(err.as_ref().to_string(), "foo");
        assert_eq!(err.as_mut().to_string(), "foo");
        assert!(err.downcast_ref::<CoreError>().is_some());
        assert!(err.downcast_mut::<CoreError>().is_some());
        assert!(err.source().is_none());

        let mut err = RvError::from(io::Error::new(io::ErrorKind::AlreadyExists, "foo"));
        assert_eq!("foo", err.to_string());
        assert_eq!("foo", err.as_ref().to_string());
        assert_eq!("foo", err.as_mut().to_string());
        assert!(err.downcast_ref::<io::Error>().is_some());
        assert!(err.downcast_mut::<io::Error>().is_some());
        assert!(err.source().is_none());

        let mut err = RvError::from(IterError::ItemNotFound);
        assert_eq!("iterator item not found", err.to_string());
        assert_eq!("iterator item not found", err.as_ref().to_string());
        assert_eq!("iterator item not found", err.as_mut().to_string());
        assert!(err.downcast_ref::<IterError>().is_some());
        assert!(err.downcast_mut::<IterError>().is_some());
        assert!(err.source().is_none());

        let mut err = RvError::from(FileError::FailedToExtractString);
        assert_eq!("Failed to extract string from file", err.to_string());
        assert_eq!("Failed to extract string from file", err.as_ref().to_string());
        assert_eq!("Failed to extract string from file", err.as_mut().to_string());
        assert!(err.downcast_ref::<FileError>().is_some());
        assert!(err.downcast_mut::<FileError>().is_some());
        assert!(err.source().is_none());

        let mut err = RvError::from(nix::errno::Errno::UnknownErrno);
        assert_eq!(err.to_string(), "UnknownErrno: Unknown errno");
        assert_eq!(err.as_ref().to_string(), "UnknownErrno: Unknown errno");
        assert_eq!(err.as_mut().to_string(), "UnknownErrno: Unknown errno");
        assert!(err.downcast_ref::<nix::errno::Errno>().is_some());
        assert!(err.downcast_mut::<nix::errno::Errno>().is_some());
        assert!(err.source().is_none());

        let mut err = RvError::from(PathError::Empty);
        assert_eq!("path empty", err.to_string());
        assert_eq!("path empty", err.as_ref().to_string());
        assert_eq!("path empty", err.as_mut().to_string());
        assert!(err.downcast_ref::<PathError>().is_some());
        assert!(err.downcast_mut::<PathError>().is_some());
        assert!(err.source().is_none());

        let mut err = RvError::from(StringError::FailedToString);
        assert_eq!("failed to convert value to string", err.to_string());
        assert_eq!("failed to convert value to string", err.as_ref().to_string());
        assert_eq!("failed to convert value to string", err.as_mut().to_string());
        assert!(err.downcast_ref::<StringError>().is_some());
        assert!(err.downcast_mut::<StringError>().is_some());
        assert!(err.source().is_none());

        let time1 = std::time::SystemTime::now();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let time2 = std::time::SystemTime::now();
        let mut err = RvError::from(time1.duration_since(time2).unwrap_err());
        assert_eq!(err.to_string(), "second time provided was later than self");
        assert_eq!(err.as_ref().to_string(), "second time provided was later than self");
        assert_eq!(err.as_mut().to_string(), "second time provided was later than self");
        assert!(err.downcast_ref::<std::time::SystemTimeError>().is_some());
        assert!(err.downcast_mut::<std::time::SystemTimeError>().is_some());
        assert!(err.source().is_none());

        let mut err = RvError::from(std::str::from_utf8(&vec![0, 159, 146, 150]).unwrap_err());
        assert_eq!(err.to_string(), "invalid utf-8 sequence of 1 bytes from index 1");
        assert_eq!(err.as_ref().to_string(), "invalid utf-8 sequence of 1 bytes from index 1");
        assert_eq!(err.as_mut().to_string(), "invalid utf-8 sequence of 1 bytes from index 1");
        assert!(err.downcast_ref::<std::str::Utf8Error>().is_some());
        assert!(err.downcast_mut::<std::str::Utf8Error>().is_some());
        assert!(err.source().is_none());

        let mut err = RvError::from(std::env::VarError::NotPresent);
        assert_eq!("environment variable not found", err.to_string());
        assert_eq!("environment variable not found", err.as_ref().to_string());
        assert_eq!("environment variable not found", err.as_mut().to_string());
        assert!(err.downcast_ref::<std::env::VarError>().is_some());
        assert!(err.downcast_mut::<std::env::VarError>().is_some());
        assert!(err.source().is_none());

        let mut err = RvError::from(VfsError::Unavailable);
        assert_eq!("Virtual filesystem is unavailable", err.to_string());
        assert_eq!("Virtual filesystem is unavailable", err.as_ref().to_string());
        assert_eq!("Virtual filesystem is unavailable", err.as_mut().to_string());
        assert!(err.downcast_ref::<VfsError>().is_some());
        assert!(err.downcast_mut::<VfsError>().is_some());
        assert!(err.source().is_none());
    }

    fn path_empty() -> RvResult<PathBuf> {
        Err(PathError::Empty)?
    }

    #[test]
    fn test_is() {
        assert!(path_empty().is_err());
        assert!(path_empty().unwrap_err().is::<PathError>());
    }

    #[test]
    fn test_downcast_ref() {
        assert!(path_empty().is_err());
        assert_eq!(path_empty().unwrap_err().downcast_ref::<PathError>(), Some(&PathError::Empty));
    }
}

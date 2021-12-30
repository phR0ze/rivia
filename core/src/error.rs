use crate::{
    iter_error::*,
    path_error::*,
    string_error::*,
    vfs_error::*,
};
use std::{error::Error as StdError, fmt};

/// `Result<T>` provides a simplified result type with a common error type
pub type RvResult<T> = std::result::Result<T, RvError>;

/// RiviaError wraps all the internal errors that might occur in one common error type
#[derive(Debug)]
pub enum RvError
{
    /// A interator error
    Iter(IterError),

    /// A pathing error
    Path(PathError),

    /// A string error
    String(StringError),

    /// An internal Utf8 error
    Utf8(std::str::Utf8Error),

    /// Environment variable error
    Var(std::env::VarError),

    /// Virtul File System errror
    Vfs(VfsError),
}

impl RvError
{
    /// Implemented directly on the `Error` type to reduce casting required
    pub fn is<T: StdError + 'static>(&self) -> bool
    {
        self.as_ref().is::<T>()
    }

    /// Implemented directly on the `Error` type to reduce casting required
    pub fn downcast_ref<T: StdError + 'static>(&self) -> Option<&T>
    {
        self.as_ref().downcast_ref::<T>()
    }

    /// Implemented directly on the `Error` type to reduce casting required
    pub fn downcast_mut<T: StdError + 'static>(&mut self) -> Option<&mut T>
    {
        self.as_mut().downcast_mut::<T>()
    }

    /// Implemented directly on the `Error` type to reduce casting required
    /// which allows for using as_ref to get the correct pass through.
    pub fn source(&self) -> Option<&(dyn StdError + 'static)>
    {
        self.as_ref().source()
    }
}
impl StdError for RvError {}

impl fmt::Display for RvError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match *self {
            RvError::Iter(ref err) => write!(f, "{}", err),
            RvError::Path(ref err) => write!(f, "{}", err),
            RvError::String(ref err) => write!(f, "{}", err),
            RvError::Utf8(ref err) => write!(f, "{}", err),
            RvError::Var(ref err) => write!(f, "{}", err),
            RvError::Vfs(ref err) => write!(f, "{}", err),
        }
    }
}

impl AsRef<dyn StdError> for RvError
{
    fn as_ref(&self) -> &(dyn StdError + 'static)
    {
        match *self {
            RvError::Iter(ref err) => err,
            RvError::Path(ref err) => err,
            RvError::String(ref err) => err,
            RvError::Utf8(ref err) => err,
            RvError::Var(ref err) => err,
            RvError::Vfs(ref err) => err,
        }
    }
}

impl AsMut<dyn StdError> for RvError
{
    fn as_mut(&mut self) -> &mut (dyn StdError + 'static)
    {
        match *self {
            RvError::Iter(ref mut err) => err,
            RvError::Path(ref mut err) => err,
            RvError::String(ref mut err) => err,
            RvError::Utf8(ref mut err) => err,
            RvError::Var(ref mut err) => err,
            RvError::Vfs(ref mut err) => err,
        }
    }
}

impl From<IterError> for RvError {
    fn from(err: IterError) -> RvError {
        RvError::Iter(err)
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

impl From<std::str::Utf8Error> for RvError
{
    fn from(err: std::str::Utf8Error) -> RvError
    {
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
mod tests
{
    use crate::*;
    use std::{io, path::PathBuf};

    #[test]
    fn test_error() {
        // let mut err = FnError::from(VfsError::FailedToExtractString);
        // assert_eq!("failed to extract string from file", err.to_string());
        // assert_eq!("failed to extract string from file", err.as_ref().to_string());
        // assert_eq!("failed to extract string from file", err.as_mut().to_string());
        // assert!(err.downcast_ref::<VfsError>().is_some());
        // assert!(err.downcast_mut::<VfsError>().is_some());
        // assert!(err.source().is_none());

        // let mut err = FnError::from(glob::PatternError { pos: 1, msg: "1" });
        // assert_eq!("glob failure: Pattern syntax error near position 1: 1", err.to_string());
        // assert_eq!(
        //     "glob failure: Pattern syntax error near position 1: 1",
        //     err.as_ref().to_string()
        // );
        // assert_eq!(
        //     "glob failure: Pattern syntax error near position 1: 1",
        //     err.as_mut().to_string()
        // );
        // assert!(err.downcast_ref::<MiscError>().is_some());
        // assert!(err.downcast_mut::<MiscError>().is_some());
        // assert!(err.source().is_none());

        // let mut err = FnError::from(io::Error::new(io::ErrorKind::AlreadyExists, "foo"));
        // assert_eq!("foo", err.to_string());
        // assert_eq!("foo", err.as_ref().to_string());
        // assert_eq!("foo", err.as_mut().to_string());
        // assert!(err.downcast_ref::<io::Error>().is_some());
        // assert!(err.downcast_mut::<io::Error>().is_some());
        // assert!(err.source().is_none());

        // let mut err = FnError::from(IterError::ItemNotFound);
        // assert_eq!("iterator item not found", err.to_string());
        // assert_eq!("iterator item not found", err.as_ref().to_string());
        // assert_eq!("iterator item not found", err.as_mut().to_string());
        // assert!(err.downcast_ref::<IterError>().is_some());
        // assert!(err.downcast_mut::<IterError>().is_some());
        // assert!(err.source().is_none());

        // let mut err = FnError::from(std::ffi::CString::new(b"f\0oo".to_vec()).unwrap_err());
        // assert_eq!("nul byte found in provided data at position: 1", err.to_string());
        // assert_eq!("nul byte found in provided data at position: 1", err.as_ref().to_string());
        // assert_eq!("nul byte found in provided data at position: 1", err.as_mut().to_string());
        // assert!(err.downcast_ref::<std::ffi::NulError>().is_some());
        // assert!(err.downcast_mut::<std::ffi::NulError>().is_some());
        // assert!(err.source().is_none());

        // let mut err = FnError::from(OsError::KernelReleaseNotFound);
        // assert_eq!("kernel release was not found", err.to_string());
        // assert_eq!("kernel release was not found", err.as_ref().to_string());
        // assert_eq!("kernel release was not found", err.as_mut().to_string());
        // assert!(err.downcast_ref::<OsError>().is_some());
        // assert!(err.downcast_mut::<OsError>().is_some());
        // assert!(err.source().is_none());

        // let mut err = FnError::from(PathError::Empty);
        // assert_eq!("path empty", err.to_string());
        // assert_eq!("path empty", err.as_ref().to_string());
        // assert_eq!("path empty", err.as_mut().to_string());
        // assert!(err.downcast_ref::<PathError>().is_some());
        // assert!(err.downcast_mut::<PathError>().is_some());
        // assert!(err.source().is_none());

        // let mut err = FnError::from(regex::Error::Syntax("foo".to_string()));
        // assert_eq!("foo", err.to_string());
        // assert_eq!("foo", err.as_ref().to_string());
        // assert_eq!("foo", err.as_mut().to_string());
        // assert!(err.downcast_ref::<regex::Error>().is_some());
        // assert!(err.downcast_mut::<regex::Error>().is_some());
        // assert!(err.source().is_none());

        // let mut err = FnError::from(MiscError::Msg("foo".to_string()));
        // assert_eq!("foo", err.to_string());
        // assert_eq!("foo", err.as_ref().to_string());
        // assert_eq!("foo", err.as_mut().to_string());
        // assert!(err.downcast_ref::<MiscError>().is_some());
        // assert!(err.downcast_mut::<MiscError>().is_some());
        // assert!(err.source().is_none());

        // let mut err = FnError::from(StringError::FailedToString);
        // assert_eq!("failed to convert value to string", err.to_string());
        // assert_eq!("failed to convert value to string", err.as_ref().to_string());
        // assert_eq!("failed to convert value to string", err.as_mut().to_string());
        // assert!(err.downcast_ref::<StringError>().is_some());
        // assert!(err.downcast_mut::<StringError>().is_some());
        // assert!(err.source().is_none());

        // let mut err = FnError::from(UserError::DoesNotExistById(1));
        // assert_eq!("user does not exist: 1", err.to_string());
        // assert_eq!("user does not exist: 1", err.as_ref().to_string());
        // assert_eq!("user does not exist: 1", err.as_mut().to_string());
        // assert!(err.downcast_ref::<UserError>().is_some());
        // assert!(err.downcast_mut::<UserError>().is_some());
        // assert!(err.source().is_none());

        let mut err = RvError::from(std::env::VarError::NotPresent);
        assert_eq!("environment variable not found", err.to_string());
        assert_eq!("environment variable not found", err.as_ref().to_string());
        assert_eq!("environment variable not found", err.as_mut().to_string());
        assert!(err.downcast_ref::<std::env::VarError>().is_some());
        assert!(err.downcast_mut::<std::env::VarError>().is_some());
        assert!(err.source().is_none());
    }

    fn path_empty() -> RvResult<PathBuf> {
        Err(PathError::Empty)?
    }

    #[test]
    fn test_is()
    {
        assert!(path_empty().is_err());
        assert!(path_empty().unwrap_err().is::<PathError>());
    }

    #[test]
    fn test_downcast_ref()
    {
        assert!(path_empty().is_err());
        assert_eq!(path_empty().unwrap_err().downcast_ref::<PathError>(), Some(&PathError::Empty));
    }
}

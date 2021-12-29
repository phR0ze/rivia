use std::{error::Error as StdError, fmt};
use crate::{path_error::*, string_error::*};

/// `Result<T>` provides a simplified result type with a common error type
pub type RvResult<T> = std::result::Result<T, RvError>;

/// RiviaError wraps all the internal errors that might occur in one common error type
#[derive(Debug)]
pub enum RvError
{
    /// A pathing error
    Path(PathError),

    /// A string error
    String(StringError),

    /// An internal Utf8 error
    Utf8(std::str::Utf8Error),
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
            RvError::Path(ref err) => write!(f, "{}", err),
            RvError::String(ref err) => write!(f, "{}", err),
            RvError::Utf8(ref err) => write!(f, "{}", err),
        }
    }
}

impl AsRef<dyn StdError> for RvError
{
    fn as_ref(&self) -> &(dyn StdError + 'static)
    {
        match *self {
            RvError::Path(ref err) => err,
            RvError::String(ref err) => err,
            RvError::Utf8(ref err) => err,
        }
    }
}

impl AsMut<dyn StdError> for RvError
{
    fn as_mut(&mut self) -> &mut (dyn StdError + 'static)
    {
        match *self {
            RvError::Path(ref mut err) => err,
            RvError::String(ref mut err) => err,
            RvError::Utf8(ref mut err) => err,
        }
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

#[cfg(test)]
mod tests
{
}

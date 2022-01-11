use std::{error::Error as StdError, fmt};

// An error indicating that something went wrong
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoreError
{
    /// Error return panic capture output
    PanicCapture(String),

    /// Error indicating a panic capture failed
    PanicCaptureFailure,
}

impl CoreError
{
    /// Return a simple error with the given message
    pub fn panic_capture<T: AsRef<str>>(msg: T) -> CoreError
    {
        CoreError::PanicCapture(msg.as_ref().to_owned())
    }
}

impl StdError for CoreError {}

impl fmt::Display for CoreError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match *self {
            CoreError::PanicCapture(ref msg) => write!(f, "{}", msg),
            CoreError::PanicCaptureFailure => write!(f, "an error occured during a panic capture"),
        }
    }
}

#[cfg(test)]
mod tests
{
    use crate::errors::*;

    #[test]
    fn test_errors()
    {
        assert_eq!(CoreError::PanicCapture("foo".to_string()).to_string(), "foo");
        assert_eq!(CoreError::PanicCaptureFailure.to_string(), "an error occured during a panic capture");
    }
}

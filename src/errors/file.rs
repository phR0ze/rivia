use std::{error::Error as StdError, fmt};

/// An error indicating something went wrong with a Rivia File operation
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FileError
{
    /// An error indicating that a regex string extraction failed.
    FailedToExtractString,

    /// An error indicating that the insert location was not found
    InsertLocationNotFound,
}

impl StdError for FileError {}

impl AsRef<dyn StdError> for FileError
{
    fn as_ref(&self) -> &(dyn StdError+'static)
    {
        self
    }
}

impl fmt::Display for FileError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match *self {
            FileError::FailedToExtractString => write!(f, "Failed to extract string from file"),
            FileError::InsertLocationNotFound => write!(f, "Failed to find the insert location in the file"),
        }
    }
}

#[cfg(test)]
mod tests
{
    use crate::errors::*;

    fn failed_to_extract_string() -> RvResult<FileError>
    {
        Err(FileError::FailedToExtractString)?
    }

    #[test]
    fn test_as_ref()
    {
        assert_eq!(
            FileError::FailedToExtractString.as_ref().downcast_ref::<FileError>(),
            Some(&FileError::FailedToExtractString)
        );
    }

    #[test]
    fn test_downcast()
    {
        assert!(failed_to_extract_string().is_err());
        assert_eq!(
            failed_to_extract_string().unwrap_err().downcast_ref::<FileError>(),
            Some(&FileError::FailedToExtractString)
        );
    }

    #[test]
    fn test_file_errors()
    {
        assert_eq!(FileError::FailedToExtractString.to_string(), "Failed to extract string from file");
        assert_eq!(
            FileError::InsertLocationNotFound.to_string(),
            "Failed to find the insert location in the file"
        );
    }
}

use std::{error::Error as StdError, fmt};

// An error indicating that something went wrong with a file operation
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum VfsError
{
    /// An error indicating that a regex string extraction failed.
    FailedToExtractString,

    /// An error indicating that the chmod pattern is invalid
    InvalidChmod(String),

    /// An error indicating that the symbolic chmod group is invalid
    InvalidChmodGroup(String),

    /// An error indicating that the symbolic chmod operation is invalid
    InvalidChmodOp(String),

    /// An error indicating that the symbolic chmod permmisions is invalid
    InvalidChmodPermissions(String),

    /// An error indicating that the symbolic chmod target is invalid
    InvalidChmodTarget(String),

    /// An error indicating that the virtual filesystem is unavailable
    Unavailable,
}

impl StdError for VfsError {}

impl AsRef<dyn StdError> for VfsError
{
    fn as_ref(&self) -> &(dyn StdError+'static)
    {
        self
    }
}

impl fmt::Display for VfsError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match *self {
            VfsError::FailedToExtractString => write!(f, "failed to extract string from file"),
            VfsError::InvalidChmod(ref sym) => write!(f, "invalid chmod symbols given: {}", sym),
            VfsError::InvalidChmodGroup(ref sym) => write!(f, "invalid chmod group given: {}", sym),
            VfsError::InvalidChmodOp(ref sym) => {
                write!(f, "invalid chmod operation given: {}", sym)
            },
            VfsError::InvalidChmodPermissions(ref sym) => {
                write!(f, "invalid chmod permissions given: {}", sym)
            },
            VfsError::InvalidChmodTarget(ref sym) => {
                write!(f, "invalid chmod target given: {}", sym)
            },
            VfsError::Unavailable => write!(f, "virtual filesystem is unavailable"),
        }
    }
}

#[cfg(test)]
mod tests
{
    use crate::errors::*;

    fn vfs_unavailable() -> RvResult<VfsError>
    {
        Err(VfsError::Unavailable)?
    }

    #[test]
    fn test_downcast()
    {
        assert!(vfs_unavailable().is_err());
        assert_eq!(
            vfs_unavailable().unwrap_err().downcast_ref::<VfsError>(),
            Some(&VfsError::Unavailable)
        );
    }

    #[test]
    fn test_vfs_errors()
    {
        assert_eq!(
            VfsError::FailedToExtractString.to_string(),
            "failed to extract string from file"
        );
        assert_eq!(
            VfsError::InvalidChmod("foo".to_string()).to_string(),
            "invalid chmod symbols given: foo"
        );
        assert_eq!(
            VfsError::InvalidChmodGroup("foo".to_string()).to_string(),
            "invalid chmod group given: foo"
        );
        assert_eq!(
            VfsError::InvalidChmodOp("foo".to_string()).to_string(),
            "invalid chmod operation given: foo"
        );
        assert_eq!(
            VfsError::InvalidChmodPermissions("foo".to_string()).to_string(),
            "invalid chmod permissions given: foo"
        );
        assert_eq!(
            VfsError::InvalidChmodTarget("foo".to_string()).to_string(),
            "invalid chmod target given: foo"
        );
        assert_eq!(VfsError::Unavailable.to_string(), "virtual filesystem is unavailable");
    }
}

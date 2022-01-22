use std::{
    error::Error as StdError,
    fmt,
    path::{Path, PathBuf},
};

/// An error indicating something went wrong with a Rivia path operation
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PathError
{
    /// An error indicating that the directory contains files
    DirContainsFiles(PathBuf),

    /// An error indicating that the path's directory doesn't match its parent
    DirDoesNotMatchParent(PathBuf),

    /// An error indicating that the path does not exist.
    DoesNotExist(PathBuf),

    /// An error indicating that the path is empty.
    Empty,

    /// An error indicating that the path exists already.
    ExistsAlready(PathBuf),

    /// An error indicating that the path does not have an extension.
    ExtensionNotFound(PathBuf),

    /// An error indicating a failure to convert the path to a string.
    FailedToString(PathBuf),

    /// An error indicating that the path does not contain a filename.
    FileNameNotFound(PathBuf),

    /// An error indicating that the path failed to expand properly.
    InvalidExpansion(PathBuf),

    /// An error indicating that the path is not a directory.
    IsNotDir(PathBuf),

    /// An error indicating that the path is not an executable file.
    IsNotExec(PathBuf),

    /// An error indicating that the path is not a file.
    IsNotFile(PathBuf),

    /// An error indicating that the path is not a file or symlink to a file.
    IsNotFileOrSymlinkToFile(PathBuf),

    /// An error indicating that a link loop was detected.
    LinkLooping(PathBuf),

    /// An error indicating that the path contains multiple user home symbols i.e. tilda.
    MultipleHomeSymbols(PathBuf),

    /// An error indicating that the path does not have a valid parent path.
    ParentNotFound(PathBuf),
}
impl PathError
{
    /// Return an error indicating that the directory contains files
    pub fn dir_contains_files<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::DirContainsFiles(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path's directory doesn't match its parent
    pub fn dir_does_not_match_parent<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::DirDoesNotMatchParent(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path does not exist
    pub fn does_not_exist<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::DoesNotExist(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path exists already
    pub fn exists_already<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::ExistsAlready(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path extension was not found
    pub fn extension_not_found<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::ExtensionNotFound(path.as_ref().to_path_buf())
    }

    /// Return an error indicating a failure to convert the path to a string
    pub fn failed_to_string<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::FailedToString(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path does not contain a filename
    pub fn filename_not_found<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::FileNameNotFound(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path is not a directory
    pub fn is_not_dir<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::IsNotDir(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path is not an executable
    pub fn is_not_exec<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::IsNotExec(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path is not a file
    pub fn is_not_file<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::IsNotFile(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path is not a file or symlink to file
    pub fn is_not_file_or_symlink_to_file<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::IsNotFileOrSymlinkToFile(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path failed to expand properly
    pub fn invalid_expansion<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::InvalidExpansion(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that link looping was detected
    pub fn link_looping<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::LinkLooping(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path contains multiple user home symbols i.e. tilda
    pub fn multiple_home_symbols<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::MultipleHomeSymbols(path.as_ref().to_path_buf())
    }

    /// Return an error indicating that the path does not have a valid parent path
    pub fn parent_not_found<T: AsRef<Path>>(path: T) -> PathError
    {
        PathError::ParentNotFound(path.as_ref().to_path_buf())
    }
}

impl StdError for PathError {}

impl AsRef<dyn StdError> for PathError
{
    fn as_ref(&self) -> &(dyn StdError+'static)
    {
        self
    }
}

impl fmt::Display for PathError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match *self {
            PathError::DirContainsFiles(ref path) => {
                write!(f, "Target directory contains files: {}", path.display())
            },
            PathError::DirDoesNotMatchParent(ref path) => {
                write!(f, "Target path's directory doesn't match parent: {}", path.display())
            },
            PathError::DoesNotExist(ref path) => {
                write!(f, "Target path does not exist: {}", path.display())
            },
            PathError::Empty => write!(f, "path empty"),
            PathError::ExistsAlready(ref path) => {
                write!(f, "Target path exists already: {}", path.display())
            },
            PathError::ExtensionNotFound(ref path) => {
                write!(f, "Target path extension not found: {}", path.display())
            },
            PathError::FailedToString(ref path) => {
                write!(f, "Target path failed to convert to string: {}", path.display())
            },
            PathError::FileNameNotFound(ref path) => {
                write!(f, "Target path filename not found: {}", path.display())
            },
            PathError::InvalidExpansion(ref path) => {
                write!(f, "Target path has an invalid expansion: {}", path.display())
            },
            PathError::IsNotDir(ref path) => {
                write!(f, "Target path is not a directory: {}", path.display())
            },
            PathError::IsNotExec(ref path) => {
                write!(f, "Target path is not an executable: {}", path.display())
            },
            PathError::IsNotFile(ref path) => {
                write!(f, "Target path is not a file: {}", path.display())
            },
            PathError::IsNotFileOrSymlinkToFile(ref path) => {
                write!(f, "Target path is not a file or a symlink to a file: {}", path.display())
            },
            PathError::LinkLooping(ref path) => {
                write!(f, "Target path causes link looping: {}", path.display())
            },
            PathError::MultipleHomeSymbols(ref path) => {
                write!(f, "Target path has multiple home symbols: {}", path.display())
            },
            PathError::ParentNotFound(ref path) => {
                write!(f, "Target path's parent not found: {}", path.display())
            },
        }
    }
}

#[cfg(test)]
mod tests
{
    use std::path::{Path, PathBuf};

    use crate::errors::*;

    fn path_empty() -> RvResult<PathBuf>
    {
        Err(PathError::Empty)?
    }

    fn parent_not_found() -> RvResult<PathBuf>
    {
        Err(PathError::parent_not_found("foo"))?
    }

    #[test]
    fn test_new_path_empty()
    {
        assert!(path_empty().is_err());
        assert_eq!(path_empty().unwrap_err().downcast_ref::<PathError>(), Some(&PathError::Empty));
    }

    #[test]
    fn test_parent_not_found()
    {
        assert!(parent_not_found().is_err());
        assert_ne!(
            parent_not_found().unwrap_err().downcast_ref::<PathError>(),
            Some(&PathError::parent_not_found("bar"))
        );
        assert_eq!(
            parent_not_found().unwrap_err().downcast_ref::<PathError>(),
            Some(&PathError::parent_not_found("foo"))
        );
        assert_eq!(
            format!("{}", parent_not_found().unwrap_err().downcast_ref::<PathError>().unwrap()),
            "Target path's parent not found: foo"
        );
    }

    #[test]
    fn test_other_errors()
    {
        assert_eq!(
            PathError::dir_contains_files(Path::new("foo")),
            PathError::DirContainsFiles(PathBuf::from("foo"))
        );
        assert_eq!(
            PathError::dir_does_not_match_parent(Path::new("foo")),
            PathError::DirDoesNotMatchParent(PathBuf::from("foo"))
        );
        assert_eq!(
            format!("{}", PathError::dir_does_not_match_parent(PathBuf::from("foo"))),
            "Target path's directory doesn't match parent: foo"
        );
        assert_eq!(PathError::does_not_exist(Path::new("foo")), PathError::DoesNotExist(PathBuf::from("foo")));
        assert_eq!(
            format!("{}", PathError::DoesNotExist(PathBuf::from("foo"))),
            "Target path does not exist: foo"
        );
        assert_eq!(format!("{}", PathError::Empty), "path empty");
        assert_eq!(PathError::exists_already(Path::new("foo")), PathError::ExistsAlready(PathBuf::from("foo")));
        assert_eq!(
            format!("{}", PathError::ExistsAlready(PathBuf::from("foo"))),
            "Target path exists already: foo"
        );
        assert_eq!(
            PathError::extension_not_found(Path::new("foo")),
            PathError::ExtensionNotFound(PathBuf::from("foo"))
        );
        assert_eq!(
            format!("{}", PathError::ExtensionNotFound(PathBuf::from("foo"))),
            "Target path extension not found: foo"
        );
        assert_eq!(PathError::failed_to_string(Path::new("foo")), PathError::FailedToString(PathBuf::from("foo")));
        assert_eq!(
            format!("{}", PathError::failed_to_string(PathBuf::from("foo"))),
            "Target path failed to convert to string: foo"
        );
        assert_eq!(
            PathError::filename_not_found(Path::new("foo")),
            PathError::FileNameNotFound(PathBuf::from("foo"))
        );
        assert_eq!(
            format!("{}", PathError::filename_not_found(PathBuf::from("foo"))),
            "Target path filename not found: foo"
        );
        assert_eq!(
            PathError::invalid_expansion(Path::new("foo")),
            PathError::InvalidExpansion(PathBuf::from("foo"))
        );
        assert_eq!(
            format!("{}", PathError::invalid_expansion(PathBuf::from("foo"))),
            "Target path has an invalid expansion: foo"
        );
        assert_eq!(PathError::is_not_dir(Path::new("foo")), PathError::IsNotDir(PathBuf::from("foo")));
        assert_eq!(
            format!("{}", PathError::is_not_dir(PathBuf::from("foo"))),
            "Target path is not a directory: foo"
        );
        assert_eq!(PathError::is_not_exec(Path::new("foo")), PathError::IsNotExec(PathBuf::from("foo")));
        assert_eq!(
            format!("{}", PathError::is_not_exec(PathBuf::from("foo"))),
            "Target path is not an executable: foo"
        );
        assert_eq!(PathError::is_not_file(Path::new("foo")), PathError::IsNotFile(PathBuf::from("foo")));
        assert_eq!(format!("{}", PathError::is_not_file(PathBuf::from("foo"))), "Target path is not a file: foo");
        assert_eq!(
            PathError::is_not_file_or_symlink_to_file(Path::new("foo")),
            PathError::IsNotFileOrSymlinkToFile(PathBuf::from("foo"))
        );
        assert_eq!(
            format!("{}", PathError::is_not_file_or_symlink_to_file(PathBuf::from("foo"))),
            "Target path is not a file or a symlink to a file: foo"
        );
        assert_eq!(PathError::link_looping(Path::new("foo")), PathError::LinkLooping(PathBuf::from("foo")));
        assert_eq!(
            format!("{}", PathError::link_looping(PathBuf::from("foo"))),
            "Target path causes link looping: foo"
        );
        assert_eq!(
            PathError::multiple_home_symbols(Path::new("foo")),
            PathError::MultipleHomeSymbols(PathBuf::from("foo"))
        );
        assert_eq!(
            format!("{}", PathError::multiple_home_symbols(PathBuf::from("foo"))),
            "Target path has multiple home symbols: foo"
        );
    }

    #[test]
    fn test_backtrace()
    {
        let err = path_empty().unwrap_err();
        println!("{:?}", err);
    }
}

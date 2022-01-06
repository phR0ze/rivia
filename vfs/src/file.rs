//! # VfsFile
//! Provides a virtual file system reference to an open file on the filesystem loosely following
//! the std::fs::File https://doc.rust-lang.org/src/std/fs.rs.html#92-94 but redirects all calls
//! back to the the original Vfs backend to keep Vfs operations consistent.
//! 
//! An instance of a `VfsFile` can be read and/or written depending on what options it was opened
//! with. Files also implement [`Seek`] to alter the logical cursor that the file contains
//! internally.
//! 
//! Files are automatically closed when they go out of scope. Errors detected on closing are
//! ignored by the implementation of `Drop`. Use the method [`sync_all`] if these errors must be
//! manually handled.
//!
//! ## Features
//! * 
pub trait VfsFile
{
    // fn open<P: AsRef<Path>>(path: P) -> io::Result<File>;
    // fn create<P: AsRef<Path>>(path: P) -> io::Result<File>;
    // fn with_options() -> OpenOptions;

    // fn metadata(&self) -> io::Result<Metadata>;
    // fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>>;
    // fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String>;
    // fn set_len(&self, size: u64) -> io::Result<()>;
    // fn set_permissions(&self, perm: Permissions) -> io::Result<()>;
    // fn sync_all(&self) -> io::Result<()>;
    // fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()>;
}

/// Virtual file system loosely based on the std::fs::File
#[derive(Debug)]
pub enum VfsFileType
{
    File,
    Directory,
}

impl VfsFileType
{
    // pub fn is_dir(&self) -> bool;
    // pub fn is_file(&self) -> bool;
    // pub fn is_symlink(&self) -> bool;
}

/// Virtual file system metadata information
#[derive(Debug)]
pub struct VfsMetadata
{
    pub typ: VfsFileType,
    pub len: u64,
}

impl VfsMetadata
{
    // pub fn file_type(&self) -> FileType;
    // pub fn is_dir(&self) -> bool;
    // pub fn is_file(&self) -> bool;
    // pub fn is_symlink(&self) -> bool;
    // pub fn len(&self) -> u64;
    // pub fn permissions(&self) -> Permissions;
    // pub fn modified(&self) -> io::Result<SystemTime>;
    // pub fn accessed(&self) -> io::Result<SystemTime>;
    // pub fn created(&self) -> io::Result<SystemTime>;
}

#[derive(Debug)]
pub struct VfsPermissions;

impl VfsPermissions
{
    // pub fn readonly(&self) -> bool;
    // pub fn set_readonly(&mut self, readonly: bool);
}
//! # VfsPath
//! Provides a virtual file system compatibile Path type that redirects all calls back to the
//! the original Vfs backend to keep Vfs operations consistent.
//!
//! ## Features
//! * Distinction between absolute paths and relative paths
//! * Allows for chaining of path related operations using the same Vfs backend
//!
//! ## Warning
//! Once a VfsPath is converted to a std::path::Path or std::path::PathBuf all method and extension
//! calls will of course call into the standard Rust libraries outside of Vfs.
use crate::Vfs;
use std::{
    fmt,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

#[derive(Debug)]
pub struct VfsPath {
    vfs: Arc<dyn Vfs>, // originating vfs backend
    path: PathBuf,     // underlying path
    abs_run: bool,     // has absolute path expansion been run
}

impl fmt::Display for VfsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl Clone for VfsPath {
    fn clone(&self) -> Self {
        Self { path: self.path.clone(), abs_run: self.abs_run, vfs: self.vfs.clone() }
    }
}

impl VfsPath {
    /// Create a new [`VfsPath`] instance with the given 'vfs' backend and 'path'.
    pub fn from<T: Into<PathBuf>>(vfs: Arc<dyn Vfs>, path: T) -> VfsPath {
        VfsPath { path: path.into(), abs_run: false, vfs }
    }

    /// Return the Vfs Path as a &std::path::Path
    pub fn as_ref(&self) -> &Path {
        &self.path
    }

    /// Return the Vfs Path as a std::path::PathBuf
    pub fn as_buf(self) -> PathBuf {
        self.path
    }

    /// Return the path default display implementation
    pub fn display(&self) -> std::path::Display<'_> {
        self.path.display()
    }

    /// Convert the path to a string via the underlying display() implementation
    pub fn to_string(&self) -> String {
        format!("{}", self.display())
    }
}
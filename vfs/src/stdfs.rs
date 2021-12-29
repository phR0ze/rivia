//! # Stdfs is a Vfs backend implementation that wraps the standard library `std::fs`
//!
//! ### Example
/// ```no_run
/// use rivia-vfs::prelude::*;
/// ```
use crate::{path::VfsPath, Vfs};
use rivia_core::*;
use std::fmt::Debug;
use std::{
    fs::File,
    path::{Path, PathBuf},
};

/// `Stdfs` is a Vfs backend implementation that wraps the standard library `std::fs`
/// functions for use with Vfs.
#[derive(Debug)]
pub struct Stdfs;
impl Stdfs
{
    /// Create a new instance of the Stdfs Vfs backend implementation
    pub fn new() -> Self
    {
        Self
    }

    /// Return the path in an absolute clean form
    ///
    /// ### Examples
    /// ```
    /// ```
    pub fn abs<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
        let path = path.as_ref();

        // // Check for empty string
        // if path.is_empty() {
        //     return PathError::Empty.into()?;
        // }

        // // Expand home directory
        // let mut path_buf = Stdfs::expand(path)?;

        // // Trim protocol prefix if needed
        // path_buf = path_buf.trim_protocol();

        // // Clean the resulting path
        // path_buf = path_buf.clean()?;

        // // Expand relative directories if needed
        // if !path_buf.is_absolute() {
        //     let mut curr = Stdfs::cwd()?;
        //     while let Ok(path) = path_buf.first() {
        //         match path {
        //             Component::CurDir => {
        //                 path_buf = path_buf.trim_first();
        //             },
        //             Component::ParentDir => {
        //                 curr = curr.dir()?;
        //                 path_buf = path_buf.trim_first();
        //             },
        //             _ => return Ok(curr.mash(path_buf)),
        //         };
        //     }
        //     return Ok(curr);
        // }

        //Ok(path_buf)
        Ok(path.to_owned())
    }

    /// Expand all environment variables in the path as well as the home directory.
    ///
    /// WARNING: Does not expand partials e.g. "/foo${BAR}ing/blah" only complete components
    /// e.g. "/foo/${BAR}/blah"
    ///
    /// ### Examples
    /// ```
    /// ```
    pub fn expand<T: AsRef<Path>>(path: T) -> RvResult<PathBuf> {
        let mut path = path.as_ref().to_path_buf();
        let pathstr = path.to_string()?;

        // // Expand home directory
        // match pathstr.matches('~').count() {
        //     // Only home expansion at the begining of the path is allowed
        //     cnt if cnt > 1 => return Err(PathError::multiple_home_symbols(path).into()),

        //     // Invalid home expansion requested
        //     cnt if cnt == 1 && !path.has_prefix("~/") && pathstr != "~" => {
        //         return Err(PathError::invalid_expansion(path).into());
        //     },

        //     // Single tilda only
        //     cnt if cnt == 1 && pathstr == "~" => {
        //         path = Stdfs::home_dir()?;
        //     },

        //     // Replace prefix with home directory
        //     1 => path = Stdfs::home_dir()?.mash(&pathstr[2..]),
        //     _ => {},
        // };

        // // Expand other variables that may exist in the path
        // let pathstr = path.to_string()?;
        // if pathstr.matches('$').some() {
        //     let mut path_buf = PathBuf::new();
        //     for x in path.components() {
        //         match x {
        //             Component::Normal(y) => {
        //                 let seg = y.to_string()?;
        //                 if let Some(chunk) = seg.strip_prefix("${") {
        //                     if let Some(key) = chunk.strip_suffix("}") {
        //                         let var = sys::var(key)?;
        //                         path_buf.push(var);
        //                     } else {
        //                         return Err(PathError::invalid_expansion(seg).into());
        //                     }
        //                 } else if let Some(key) = seg.strip_prefix('$') {
        //                     let var = sys::var(key)?;
        //                     path_buf.push(var);
        //                 } else {
        //                     path_buf.push(seg);
        //                 }
        //             },
        //             _ => path_buf.push(x),
        //         };
        //     }
        //     path = path_buf;
        // }

        Ok(path)
    }

    /// Returns the contents of the `path` as a `String`.
    ///
    /// ### Examples
    /// ```
    /// ```
    pub fn read<T: AsRef<Path>>(path: T) -> RvResult<String> {
        // let path = Stdfs::abs(path.as_ref())?;
        // match std::fs::read_to_string(path) {
        //     Ok(data) => Ok(data),
        //     Err(err) => Err(err.into()),
        // }
        Ok("foo".to_string())
    }

    /// Write `[u8]` data to a file which means `str` or `String`. Handles path expansion.
    ///
    /// ### Examples
    /// ```
    /// ```
    pub fn write<T: AsRef<Path>, U: AsRef<[u8]>>(path: T, data: U) -> RvResult<()> {
        // let path = Stdfs::abs(path.as_ref())?;
        // let mut f = File::create(path)?;
        // f.write_all(data.as_ref())?;

        // // f.sync_all() works better than f.flush()?
        // f.sync_all()?;
        Ok(())
    }
}

impl Vfs for Stdfs
{

    /// Return the path in an absolute clean form
    ///
    /// ### Examples
    /// ```
    /// ```
    fn abs(&self, path: &Path) -> RvResult<PathBuf> {
        Stdfs::abs(path)
    }

    /// Expand all environment variables in the path as well as the home directory.
    ///
    /// WARNING: Does not expand partials e.g. "/foo${BAR}ing/blah" only complete components
    /// e.g. "/foo/${BAR}/blah"
    ///
    /// ### Examples
    /// ```
    /// ```
    fn expand(&self, path: &Path) -> RvResult<PathBuf> {
        Stdfs::expand(path)
    }

    /// Returns the contents of the `path` as a `String`.
    ///
    /// ### Examples
    /// ```
    /// ```
    fn read(&self, path: &Path) -> RvResult<String> {
        Stdfs::read(path)
    }

    /// Write `[u8]` data to a file which means `str` or `String`. Handles path expansion.
    ///
    /// ### Examples
    /// ```
    /// ```
    fn write(&self, path: &Path, data: &[u8]) -> RvResult<()>
    {
        Stdfs::write(path, data)
    }
}
use std::{
    cmp,
    collections::HashMap,
    fmt::Debug,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::{
    errors::*,
    fs::{Entry, EntryIter, VfsEntry},
};

// Simple type to use when referring to the multi-thread safe locked hashmap that is
// the memory filesystem's backend storage.
pub(crate) type MemfsDir = Arc<RwLock<HashMap<PathBuf, MemfsEntry>>>;

/// MemfsEntry is an implementation a virtual filesystem trait for a single filesystem item. It is
/// implemented
///
/// ### Example
/// ```
/// use rivia::prelude::*;
/// ```
#[derive(Debug)]
pub struct MemfsEntry
{
    pub(crate) fs: MemfsDir,  // multi-thread safe filesystem storage
    pub(crate) data: Vec<u8>, // memory file data
    pub(crate) pos: u64,      // position in the file when reading or writing

    pub(crate) path: PathBuf, // path of the entry
    pub(crate) alt: PathBuf,  // alternate path for the entry, used with links
    pub(crate) dir: bool,     // is this entry a dir
    pub(crate) file: bool,    // is this entry a file
    pub(crate) link: bool,    // is this entry a link
    pub(crate) mode: u32,     // permission mode of the entry
    pub(crate) follow: bool,  // tracks if the path and alt have been switched
    pub(crate) cached: bool,  // tracks if properties have been cached
}

impl Clone for MemfsEntry
{
    fn clone(&self) -> Self
    {
        Self {
            fs: self.fs.clone(),
            data: self.data.clone(),
            pos: self.pos,
            path: self.path.clone(),
            alt: self.alt.clone(),
            dir: self.dir,
            file: self.file,
            link: self.link,
            mode: self.mode,
            follow: self.follow,
            cached: self.cached,
        }
    }
}

impl MemfsEntry
{
    pub(crate) fn new<T: Into<PathBuf>>(path: T) -> Self
    {
        Self {
            fs: Arc::new(RwLock::new(HashMap::new())),
            data: vec![],
            pos: 0,
            path: path.into(),
            alt: PathBuf::new(),
            dir: true, // directory by default
            file: false,
            link: false,
            mode: 0,
            follow: false,
            cached: false,
        }
    }

    /// Set the entry to be a directory. Will automatically set file and link to false. In order to
    /// have a link that points to a directory you need to call link() after this call.
    pub fn dir(mut self) -> Self
    {
        self.file = false;
        self.link = false;
        self.dir = true;
        self
    }

    /// Set the entry to be a file. Will automatically set dir and link to false. In order to have a
    /// link that points to a file you need to call link() after this call.
    pub fn file(mut self) -> Self
    {
        self.dir = false;
        self.link = false;
        self.file = true;
        self
    }

    /// Len reports the length of the data in bytes until the end of the file from the current
    /// position.
    pub fn len(&self) -> u64
    {
        self.data.len() as u64 - self.pos
    }

    /// Set the entry to be a link
    pub fn link(mut self) -> Self
    {
        self.link = true;
        self
    }

    /// Create an iterator from the given path to iterate over just the contents of this path
    /// non-recursively.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn iter(path: &Path, follow: bool) -> RvResult<EntryIter>
    {
        Ok(EntryIter {
            path: path.to_path_buf(),
            cached: false,
            following: follow,
            iter: Box::new(MemfsEntryIter(fs::read_dir(path)?)),
        })
    }

    /// Switch the `path` and `alt` values if `is_symlink` reports true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub fn follow(mut self, follow: bool) -> Self
    {
        if follow && !self.follow {
            self.follow = true;
            if self.link {
                let path = self.path;
                self.path = self.alt;
                self.alt = path;
            }
        }
        self
    }
}

impl Entry for MemfsEntry
{
    /// `path` reports the actual file or directory when `is_symlink` reports false. When
    /// `is_symlink` reports true and `follow` reports true `path` will report the actual file or
    /// directory that the link points to and `alt` will report the link's path. When `is_symlink`
    /// reports true and `follow` reports false `path` will report the link's path and `alt` will
    /// report the actual file or directory the link points to.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn path(&self) -> &Path
    {
        &self.path
    }

    /// Move the `path` value out of this struct as an owned value
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn path_buf(self) -> PathBuf
    {
        self.path
    }

    /// `alt` will be empty unless `is_symlink` reports true. When `is_symlink` reports true and
    /// `follow` reports true `alt` will report the path to the link and `path` will report the
    /// path to the actual file or directory the link points to. When `is_symlink` reports trueand
    /// `follow` reports false `alt` will report the actual file or directory the link points to
    /// and `path` will report the link path.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn alt(&self) -> &Path
    {
        &self.alt
    }

    /// Move the `link` value out of this struct as an owned value
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn alt_buf(self) -> PathBuf
    {
        self.alt
    }

    /// Switch the `path` and `alt` values if `is_symlink` reports true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn follow(self, follow: bool) -> VfsEntry
    {
        VfsEntry::Memfs(self.follow(follow))
    }

    /// Return the current following state
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn following(&self) -> bool
    {
        self.follow
    }

    /// Regular directories and symlinks that point to directories will report
    /// true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_dir(&self) -> bool
    {
        self.dir
    }

    /// Regular files and symlinks that point to files will report true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_file(&self) -> bool
    {
        self.file
    }

    /// Links will report true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn is_symlink(&self) -> bool
    {
        self.link
    }

    /// Reports the mode of the path
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn mode(&self) -> u32
    {
        self.mode
    }

    /// Create an iterator from the given path to iterate over just the contents of this path
    /// non-recursively.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn iter(&self) -> RvResult<EntryIter>
    {
        MemfsEntry::iter(&self.path, false)
    }

    /// Up cast the trait type to the enum wrapper
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    fn upcast(self) -> VfsEntry
    {
        VfsEntry::Memfs(self)
    }
}

// Implement the Read trait for the MemfsEntry
impl io::Read for MemfsEntry
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        // Ensure that we are working with a valid file
        if self.dir || self.link {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Target path '{}' is not a readable file", self.path.display()),
            ));
        }

        let pos = self.pos as usize;

        // Determine how much data to read from the file
        let len = cmp::min(buf.len(), self.len() as usize);

        // Read the indicated data length
        buf[..len].copy_from_slice(&self.data.as_slice()[pos..pos + len]);

        // Advance the position in the file
        self.pos += len as u64;

        // Return the length of data read
        Ok(len)
    }
}

// Implement the Seek trait for the MemfsEntry
impl io::Seek for MemfsEntry
{
    fn seek(&mut self, pos: io::SeekFrom) -> std::io::Result<u64>
    {
        match pos {
            io::SeekFrom::Start(offset) => self.pos = offset,
            io::SeekFrom::Current(offset) => self.pos = (self.pos as i64 + offset) as u64,
            io::SeekFrom::End(offset) => self.pos = (self.data.len() as i64 + offset) as u64,
        }
        Ok(self.pos)
    }
}

// Implement the Write trait for the MemfsEntry
impl io::Write for MemfsEntry
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
        // Ensure that we are working with a valid file
        if self.dir || self.link {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Target path '{}' is not a writable file", self.path.display()),
            ));
        }
        self.data.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()>
    {
        self.data.flush()
    }
}

#[derive(Debug)]
struct MemfsEntryIter(fs::ReadDir);
impl Iterator for MemfsEntryIter
{
    type Item = RvResult<VfsEntry>;

    fn next(&mut self) -> Option<RvResult<VfsEntry>>
    {
        // if let Some(value) = self.0.next() {
        //     return Some(match MemfsEntry::from(&trying!(value).path()) {
        //         Ok(x) => Ok(x.upcast()),
        //         Err(e) => Err(e),
        //     });
        // }
        None
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    use crate::prelude::*;

    #[test]
    fn test_dir() -> RvResult<()>
    {
        Ok(())
    }

    #[test]
    fn test_not_readable_writable_file() -> RvResult<()>
    {
        let fs = Arc::new(RwLock::new(HashMap::new()));
        let mut memfile = MemfsEntry::new("foo", fs);

        // Not readable
        let mut buf = [0; 1];
        assert_eq!(
            memfile.read(&mut buf).unwrap_err().to_string(),
            "Target path 'foo' is not a readable file"
        );

        // Not writable
        assert_eq!(
            memfile.write(b"foobar1, ").unwrap_err().to_string(),
            "Target path 'foo' is not a writable file"
        );
        Ok(())
    }

    #[test]
    fn test_file_read_write_seek_len() -> RvResult<()>
    {
        let fs = Arc::new(RwLock::new(HashMap::new()));
        let mut memfile = MemfsEntry::new("foo", fs).file();

        // Write out the data
        assert_eq!(memfile.len(), 0);
        memfile.write(b"foobar1, ")?;
        assert_eq!(memfile.data, b"foobar1, ");
        assert_eq!(memfile.len(), 9);

        // Write out using the write macro
        write!(memfile, "foobar2, ")?;
        assert_eq!(memfile.len(), 18);
        assert_eq!(memfile.data, b"foobar1, foobar2, ");

        memfile.write(b"foobar3")?;
        assert_eq!(memfile.len(), 25);
        assert_eq!(memfile.data, b"foobar1, foobar2, foobar3");

        // read 1 byte
        let mut buf = [0; 1];
        memfile.read(&mut buf)?;
        assert_eq!(memfile.len(), 24);
        assert_eq!(&buf, b"f");

        // Seek back to start and try again
        memfile.seek(SeekFrom::Start(0))?;
        assert_eq!(memfile.len(), 25);
        let mut buf = [0; 9];
        memfile.read(&mut buf)?;
        assert_eq!(memfile.len(), 16);
        assert_eq!(&buf, b"foobar1, ");

        // Read the remaining data
        let mut buf = Vec::new();
        memfile.read_to_end(&mut buf)?;
        assert_eq!(memfile.len(), 0);
        assert_eq!(&buf, b"foobar2, foobar3");

        // rewind and read into a String
        let mut buf = String::new();
        memfile.rewind()?;
        assert_eq!(memfile.len(), 25);
        memfile.read_to_string(&mut buf)?;
        assert_eq!(memfile.len(), 0);
        assert_eq!(buf, "foobar1, foobar2, foobar3".to_string());

        Ok(())
    }
}

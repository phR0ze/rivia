use std::{
    cmp::{self, Ordering},
    collections::{HashMap, HashSet},
    fmt, fs,
    hash::{Hash, Hasher},
    io,
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
};

use itertools::Itertools;

use crate::{
    errors::*,
    exts::*,
    sys::{self, Entry, EntryIter, PathExt, VfsEntry},
};

// Simple type to use when referring to the multi-thread safe locked hashmap that is a directory on
// the memory filesystem.
pub(crate) type MemfsFiles = Arc<RwLock<HashMap<String, MemfsEntry>>>;

// MemfsEntryOpts implements the builder pattern to provide advanced options for creating
// MemfsEntry instances
#[derive(Debug)]
pub(crate) struct MemfsEntryOpts
{
    path: PathBuf, // path of the entry
    alt: PathBuf,  // alternate path for the entry, used with links
    dir: bool,     // is this entry a dir
    file: bool,    // is this entry a file
    link: bool,    // is this entry a link
    mode: u32,     // permission mode of the entry
}

impl MemfsEntryOpts
{
    // Create a MemfsEntry instance from the MemfsEntryOpts instance
    pub(crate) fn new(self) -> MemfsEntry
    {
        MemfsEntry {
            entries: HashMap::new(),
            data: vec![],
            pos: 0,
            files: if self.dir { Some(HashSet::new()) } else { None },
            path: self.path,
            alt: self.alt,
            dir: self.dir,
            file: self.file,
            link: self.link,
            mode: self.mode,
            follow: false,
            cached: false,
        }
    }

    pub(crate) fn alt<T: Into<PathBuf>>(mut self, path: T) -> Self
    {
        self.alt = path.into();
        self
    }

    pub(crate) fn dir(mut self) -> Self
    {
        self.dir = true;
        self.file = false;
        self
    }

    pub(crate) fn file(mut self) -> Self
    {
        self.file = true;
        self.dir = false;
        self
    }

    pub(crate) fn link(mut self) -> Self
    {
        self.link = true;
        self
    }

    pub(crate) fn mode(mut self, mode: u32) -> Self
    {
        self.mode = mode;
        self
    }
}

/// MemfsEntry is an implementation of a single entry in a virtual filesystem.
///
/// ### Example
/// ```
/// use rivia::prelude::*;
/// ```
#[derive(Debug)]
pub struct MemfsEntry
{
    pub(crate) entries: HashMap<String, MemfsEntry>, // files in the directory
    pub(crate) data: Vec<u8>,                        // memory file data
    pub(crate) pos: u64,                             // position in the file

    pub(crate) files: Option<HashSet<String>>, // file or directory names

    pub(crate) path: PathBuf, // path of the entry
    pub(crate) alt: PathBuf,  // alternate path for the entry, used with links
    pub(crate) dir: bool,     // is this entry a dir
    pub(crate) file: bool,    // is this entry a file
    pub(crate) link: bool,    // is this entry a link
    pub(crate) mode: u32,     // permission mode of the entry
    pub(crate) follow: bool,  // tracks if the path and alt have been switched
    pub(crate) cached: bool,  // tracks if properties have been cached
}

impl MemfsEntry
{
    /// Create a new MemfsEntryOpts to allow for more advanced Memfs creation
    ///
    /// # Arguments
    /// * `abs` - target path expected to already be in absolute form
    pub(crate) fn opts<T: Into<PathBuf>>(path: T) -> MemfsEntryOpts
    {
        MemfsEntryOpts {
            path: path.into(),
            alt: PathBuf::new(),
            dir: true, // directory by default
            file: false,
            link: false,
            mode: 0,
        }
    }

    // Add an entry to this directory
    //
    // # Arguments
    // * `entry` - the entry to add to this one
    //
    // # Errors
    // * PathError::IsNotDir(PathBuf) when this entry is not a directory.
    // * PathError::ExistsAlready(PathBuf) when the given entry already exists.
    // entry's path
    pub(crate) fn add<T: Into<String>>(&mut self, name: T) -> RvResult<()>
    {
        let name = name.into();

        // Ensure this is a valid directory
        if !self.dir {
            return Err(PathError::is_not_dir(&self.path).into());
        }

        // Insert the new entry or error out if already exists
        if let Some(ref mut files) = self.files {
            if !files.insert(name.clone()) {
                let path = self.path.mash(name);
                return Err(PathError::exists_already(path).into());
            }
        } else {
            let mut files = HashSet::new();
            files.insert(name);
            self.files = Some(files);
        }

        Ok(())
    }

    /// Len reports the length of the data in bytes until the end of the file from the current
    /// position.
    pub(crate) fn len(&self) -> u64
    {
        self.data.len() as u64 - self.pos
    }

    /// Create an iterator from the given path to iterate over just the contents of this path
    /// non-recursively.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub(crate) fn iter(path: &Path, follow: bool) -> RvResult<EntryIter>
    {
        Ok(EntryIter {
            path: path.to_path_buf(),
            cached: false,
            following: follow,
            iter: Box::new(MemfsEntryIter(fs::read_dir(path)?)),
        })
    }

    /// Switch the `path` and `alt` values if `is_link` reports true.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    /// ```
    pub(crate) fn follow(mut self, follow: bool) -> Self
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

    /// Provide pretty printing for our filesystem
    pub(crate) fn display(&self, f: &mut fmt::Formatter, indent: Option<usize>) -> fmt::Result
    {
        let indent = indent.unwrap_or_default();
        if indent == 0 {
            writeln!(f, "{}", &self.path.display())?;
        } else {
            writeln!(f, " ({})", &self.path.display())?;
        }

        let indent = indent + 2;
        if self.dir {
            for k in self.entries.keys().sorted() {
                write!(f, "{:>w$}{}", "", &k, w = indent)?;
                self.entries[k].display(f, Some(indent))?;
            }
        }
        Ok(())
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

impl Clone for MemfsEntry
{
    fn clone(&self) -> Self
    {
        Self {
            entries: self.entries.clone(),
            data: self.data.clone(),
            pos: self.pos,
            files: self.files.clone(),
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

// Implement hashing requirements
impl Eq for MemfsEntry {}
impl PartialEq for MemfsEntry
{
    fn eq(&self, other: &Self) -> bool
    {
        self.path == other.path
    }
}
impl Hash for MemfsEntry
{
    fn hash<T: Hasher>(&self, hasher: &mut T)
    {
        self.path.hash(hasher);
    }
}

// Implement ordering
impl PartialOrd for MemfsEntry
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        self.path.partial_cmp(&other.path)
    }
}
impl Ord for MemfsEntry
{
    fn cmp(&self, other: &Self) -> Ordering
    {
        self.path.cmp(&other.path)
    }
}

// Implement the Read trait for the MemfsEntry
impl io::Read for MemfsEntry
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        // Ensure that we are working with a valid file
        if self.dir || self.link {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Target path '{}' is not a readable file", self.path.display())));
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
            return Err(io::Error::new(io::ErrorKind::Other, format!("Target path '{}' is not a writable file", self.path.display())));
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
    use crate::prelude::*;

    // #[test]
    // fn test_add_remove() -> RvResult<()>
    // {
    //     // Add a file to a directory
    //     let mut memfile1 = MemfsEntry::opts("/").new();
    //     assert_eq!(memfile1.entries.len(), 0);
    //     let memfile2 = MemfsEntry::opts("/foo").new();
    //     memfile1.add_child(memfile2.clone())?;
    //     assert_eq!(memfile1.entries.len(), 1);

    //     // Remove a file from a directory
    //     assert_eq!(memfile1.remove_child(&memfile2.path)?, Some(memfile2));
    //     assert_eq!(memfile1.entries.len(), 0);
    //     Ok(())
    // }

    // #[test]
    // fn test_remove_non_existing()
    // {
    //     let mut memfile = MemfsEntry::opts("foo").new();
    //     assert_eq!(memfile.remove_child("blah").unwrap(), None);
    // }

    // #[test]
    // fn test_remove_from_file_fails()
    // {
    //     let mut memfile = MemfsEntry::opts("foo").file().new();
    //     assert_eq!(memfile.remove_child("bar").unwrap_err().to_string(), "Target path is not a
    // directory: foo"); }

    // #[test]
    // fn test_add_already_exists_fails()
    // {
    //     let mut memfile1 = MemfsEntry::opts("/").new();
    //     let memfile2 = MemfsEntry::opts("/foo").file().new();
    //     memfile1.add_child(memfile2.clone()).unwrap();
    //     assert_eq!(memfile1.add_child(memfile2).unwrap_err().to_string(), "Target path exists
    // already: /foo"); }

    // #[test]
    // fn test_add_mismatch_path_fails()
    // {
    //     let mut memfile1 = MemfsEntry::opts("/").new();
    //     let memfile2 = MemfsEntry::opts("foo").file().new();
    //     assert_eq!(memfile1.add_child(memfile2).unwrap_err().to_string(), "Target path's
    // directory doesn't match parent: /"); }

    // #[test]
    // fn test_add_to_link_fails()
    // {
    //     let mut memfile = MemfsEntry::opts("foo").link().new();
    //     assert_eq!(memfile.add_child(MemfsEntry::opts("").new()).unwrap_err().to_string(),
    // "Target path filename not found: "); }

    // #[test]
    // fn test_add_to_file_fails()
    // {
    //     let mut memfile = MemfsEntry::opts("foo").file().new();
    //     assert_eq!(memfile.add_child(MemfsEntry::opts("").new()).unwrap_err().to_string(),
    // "Target path is not a directory: foo"); }

    // #[test]
    // fn test_ordering_and_equality()
    // {
    //     let entry1 = MemfsEntry::opts("1").new();
    //     let entry2 = MemfsEntry::opts("2").new();
    //     let entry3 = MemfsEntry::opts("3").new();

    //     let mut entries = vec![&entry1, &entry3, &entry2];
    //     entries.sort();

    //     assert_eq!(entries[0], &entry1);
    //     assert_ne!(entries[1], &entry3);
    //     assert_eq!(entries[1], &entry2);
    //     assert_eq!(entries[2], &entry3);
    // }

    // #[test]
    // fn test_not_readable_writable_file() -> RvResult<()>
    // {
    //     let mut memfile = MemfsEntry::opts("foo").new();

    //     // Not readable
    //     let mut buf = [0; 1];
    //     assert_eq!(memfile.read(&mut buf).unwrap_err().to_string(), "Target path 'foo' is not a
    // readable file");

    //     // Not writable
    //     assert_eq!(memfile.write(b"foobar1, ").unwrap_err().to_string(), "Target path 'foo' is
    // not a writable file");     Ok(())
    // }

    // #[test]
    // fn test_file_read_write_seek_len() -> RvResult<()>
    // {
    //     let mut memfile = MemfsEntry::opts("foo").file().new();

    //     // Write out the data
    //     assert_eq!(memfile.len(), 0);
    //     memfile.write(b"foobar1, ")?;
    //     assert_eq!(memfile.data, b"foobar1, ");
    //     assert_eq!(memfile.len(), 9);

    //     // Write out using the write macro
    //     write!(memfile, "foobar2, ")?;
    //     assert_eq!(memfile.len(), 18);
    //     assert_eq!(memfile.data, b"foobar1, foobar2, ");

    //     memfile.write(b"foobar3")?;
    //     assert_eq!(memfile.len(), 25);
    //     assert_eq!(memfile.data, b"foobar1, foobar2, foobar3");

    //     // read 1 byte
    //     let mut buf = [0; 1];
    //     memfile.read(&mut buf)?;
    //     assert_eq!(memfile.len(), 24);
    //     assert_eq!(&buf, b"f");

    //     // Seek back to start and try again
    //     memfile.seek(SeekFrom::Start(0))?;
    //     assert_eq!(memfile.len(), 25);
    //     let mut buf = [0; 9];
    //     memfile.read(&mut buf)?;
    //     assert_eq!(memfile.len(), 16);
    //     assert_eq!(&buf, b"foobar1, ");

    //     // Read the remaining data
    //     let mut buf = Vec::new();
    //     memfile.read_to_end(&mut buf)?;
    //     assert_eq!(memfile.len(), 0);
    //     assert_eq!(&buf, b"foobar2, foobar3");

    //     // rewind and read into a String
    //     let mut buf = String::new();
    //     memfile.rewind()?;
    //     assert_eq!(memfile.len(), 25);
    //     memfile.read_to_string(&mut buf)?;
    //     assert_eq!(memfile.len(), 0);
    //     assert_eq!(buf, "foobar1, foobar2, foobar3".to_string());

    //     Ok(())
    // }
}

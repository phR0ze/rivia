use std::{cmp, io, path::PathBuf};

use super::Memfs;

/// `MemfsFile` is an implementation of memory based file in the memory filesytem.
///
/// ### Example
/// ```
/// use rivia::prelude::*;
/// ```
#[derive(Debug, Default)]
pub(crate) struct MemfsFile
{
    pub(crate) pos: u64,              // position in the memory file
    pub(crate) data: Vec<u8>,         // datastore for the memory file
    pub(crate) path: Option<PathBuf>, // optional path to write to
    pub(crate) fs: Option<Memfs>,     // optional sharable filesystem for writes
}

impl MemfsFile
{
    /// Returns the length of the file remaining from the current position
    pub(crate) fn len(&self) -> u64
    {
        self.data.len() as u64 - self.pos
    }

    /// Attempt to write the data to the data store
    ///
    /// ### Errors
    /// * PathError::DoesNotExist(PathBuf) when the target entry or file don't exist
    pub(crate) fn sync(&mut self) -> io::Result<()>
    {
        if let Some(ref fs) = self.fs {
            if let Some(ref path) = self.path {
                let mut guard = fs.write_guard();
                if guard.contains_entry(path) {
                    if let Some(f) = guard.get_file_mut(path) {
                        f.data = self.data.clone();
                    }
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Target doesn't exist: {}", path.display()),
                    ));
                }
            }
        }
        Ok(())
    }
}

impl Clone for MemfsFile
{
    fn clone(&self) -> Self
    {
        Self {
            pos: self.pos,
            data: self.data.clone(),
            path: self.path.clone(),
            fs: match self.fs {
                Some(ref fs) => Some(fs.clone()),
                None => None,
            },
        }
    }
}

// Implement the Read trait for the MemfsFile
impl io::Read for MemfsFile
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        let pos = self.pos as usize;

        // Determine max data to read from the file
        let len = cmp::min(buf.len(), self.len() as usize);

        // Read the indicated data length
        buf[..len].copy_from_slice(&self.data.as_slice()[pos..pos + len]);

        // Advance the position in the file
        self.pos += len as u64;

        // Return the length of data read
        Ok(len)
    }
}

// Implement the Seek trait for the MemfsFile
impl io::Seek for MemfsFile
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

// Implement the Write trait for the MemfsFile
impl io::Write for MemfsFile
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
        self.data.write(buf)
    }

    fn flush(&mut self) -> io::Result<()>
    {
        self.sync()
    }
}

// Use custom drop implementation to write data to the shared filesystem
impl Drop for MemfsFile
{
    fn drop(&mut self)
    {
        // Sync data to storage
        let _result = self.sync();

        // Clear out references
        self.data.clear();
        self.path = None;
        self.fs = None;
    }
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests
{
    use super::MemfsFile;
    use crate::prelude::*;

    #[test]
    fn test_read_write_seek_len()
    {
        let mut memfile = MemfsFile::default();

        // Write using the function
        assert_eq!(memfile.len(), 0);
        memfile.write(b"foobar1, ").unwrap();
        assert_eq!(memfile.data, b"foobar1, ");
        assert_eq!(memfile.len(), 9);

        // Write out using the write macro
        write!(memfile, "foobar2, ").unwrap();
        assert_eq!(memfile.len(), 18);
        assert_eq!(memfile.data, b"foobar1, foobar2, ");

        memfile.write(b"foobar3").unwrap();
        assert_eq!(memfile.len(), 25);
        assert_eq!(memfile.data, b"foobar1, foobar2, foobar3");

        // read 1 byte
        let mut buf = [0; 1];
        memfile.read(&mut buf).unwrap();
        assert_eq!(memfile.len(), 24);
        assert_eq!(&buf, b"f");

        // Seek back to start and try again
        memfile.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(memfile.len(), 25);
        let mut buf = [0; 9];
        memfile.read(&mut buf).unwrap();
        assert_eq!(memfile.len(), 16);
        assert_eq!(&buf, b"foobar1, ");

        // Read the remaining data
        let mut buf = Vec::new();
        memfile.read_to_end(&mut buf).unwrap();
        assert_eq!(memfile.len(), 0);
        assert_eq!(&buf, b"foobar2, foobar3");

        // rewind and read into a String
        let mut buf = String::new();
        memfile.rewind().unwrap();
        assert_eq!(memfile.len(), 25);
        memfile.read_to_string(&mut buf).unwrap();
        assert_eq!(memfile.len(), 0);
        assert_eq!(buf, "foobar1, foobar2, foobar3".to_string());
    }
}

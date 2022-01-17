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

/// `MemfsFile` is an implementation of memory based file in the memory filesytem.
///
/// ### Example
/// ```
/// use rivia::prelude::*;
/// ```
#[derive(Debug)]
pub(crate) struct MemfsFile
{
    pub(crate) pos: u64,      // position in the memory file
    pub(crate) data: Vec<u8>, // datastore for the memory file
}

impl Clone for MemfsFile
{
    fn clone(&self) -> Self
    {
        Self {
            pos: self.pos,
            data: self.data.clone(),
        }
    }
}

// Implement the Read trait for the MemfsFile
impl io::Read for MemfsFile
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        let pos = self.pos as usize;

        // Determine how much data to read from the file
        let len = cmp::min(buf.len(), self.data.len() as usize);

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

    fn flush(&mut self) -> std::io::Result<()>
    {
        self.data.flush()
    }
}

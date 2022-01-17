/// `MemfsObj` is an implementation of memory based entry in the memory filesytem. It can be either
/// a directory or a file. Links will be handled by `MemfsEntry`
///
/// ### Example
/// ```
/// use rivia::prelude::*;
/// ```
#[derive(Debug)]
pub(crate) enum MemfsObj
{
    Dir(MemfsDir),
    File(MemfsFile),
}

#[derive(Debug)]
pub(crate) struct MemfsDir
{
    pub(crate) files: HashMap<String, MemfsEntry>, // files in the directory
}

#[derive(Debug)]
pub(crate) struct MemfsFile
{
    pub(crate) data: Vec<u8>, // memory file data
    pub(crate) pos: u64,      // position in the file when reading or writing
}

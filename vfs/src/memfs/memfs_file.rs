
#[derive(Debug)]
struct MemfsFile {
    path: PathBuf, // entry path
    dir: bool,     // is this entry a dir
    file: bool,    // is this entry a file
    link: bool,    // is this entry a link
    mode: u32, /* permission mode of the entry
                * data: Cursor<Vec<u8>>, // actual entry data */
}

impl Default for MemfsFile {
    fn default() -> Self {
        Self { path: PathBuf::new(), dir: false, file: false, link: false, mode: 0 }
    }
}

impl Clone for MemfsFile {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            dir: self.dir,
            file: self.file,
            link: self.link,
            mode: self.mode,
        }
    }
}

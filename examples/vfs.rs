use rivia::prelude::*;

fn main()
{
    // Write data to a file and read it back using Stdfs and Memfs via a single
    // vfs replacemnent

    // 1. Setup file to write to
    let file1 = Stdfs::mash(testing::TEST_TEMP_DIR, "file1");

    // 2. Create a new stfs instance that we can change to memfs later
    let vfs = Vfs::new_stdfs();

    // 3. Make the file writing out the data
    vfs.mkfile(&file1, b"foo bar").unwrap();
}
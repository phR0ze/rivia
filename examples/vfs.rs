use rivia::prelude::*;

fn main()
{
    // Write data to a file and read it back using Stdfs and Memfs via a single
    // vfs replacemnent

    // 1. Setup file to write to
    let file1 = sys::mash(testing::TEST_TEMP_DIR, "file1");

    // 2. Create a new stdfs instance that we can change to memfs later
    // let vfs = Vfs::new_stdfs();

    // // 3. Make the file writing out the data
    // vfs.write_all(&file1, b"hello").unwrap();

    // // 4. Read back the file contents
    // let data = vfs.read_all(&file1).unwrap();

    // println!("Data: {}", data);
    // Stdfs::remove(file1).unwrap();

    // Testing
    let memfs = Memfs::new();
    memfs.mkdir_p(Path::new("foo")).unwrap();
    println!("{}", memfs);
}

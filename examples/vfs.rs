use rivia::prelude::*;

fn main()
{
    // Write data to a file and read it back using Stdfs and Memfs via a single vfs replacemnent

    // 1. Setup file to write to

    // 2. Create a new stdfs instance that we can change to memfs later
    // let vfs = Vfs::stdfs();

    // // 3. Make the file writing out the data
    // vfs.write_all(&file1, b"hello").unwrap();

    // // 4. Read back the file contents
    // let data = vfs.read_all(&file1).unwrap();

    // println!("Data: {}", data);
    // Stdfs::remove(file1).unwrap();

    let vfs = Vfs::memfs();
    vfs_test(&vfs).unwrap();
}

fn vfs_test(vfs: &Vfs) -> RvResult<()>
{
    // 1. Create the test directory and test files
    let dir1 = vfs.mkdir_p(testing::TEST_TEMP_DIR)?;
    // vfs.mkfile(dir1.mash("file1"))?;

    for entry in vfs.entries(testing::TEST_TEMP_DIR)?.into_iter() {
        println!("{}", entry?.path().display());
    }

    Ok(())
}

use rivia_vfs::prelude::*;

fn main()
{
    // Write out and read back the file as stdfs
    read_write_all("file1").unwrap();

    // Repeat but with the Memfs provider
    vfs::set_memfs().unwrap();
    read_write_all("file1").unwrap();
}

fn read_write_all<T: AsRef<Path>>(path: T) -> RvResult<()>
{
    let tmpdir = assert_setup!();
    let file1 = tmpdir.mash(path);
    vfs::write_all(&file1, "this is a test")?;
    assert_eq!(vfs::read_all(&file1)?, "this is a test".to_string());
    assert_remove_all!(&tmpdir);
    Ok(())
}

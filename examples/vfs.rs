use rivia::prelude::*;

fn main()
{
    // Write data to a file and read it back using Stdfs and Memfs
    vfs_read_write_all(assert_vfs_setup!(Vfs::memfs(), "vfs_memfs_read_write_example")).unwrap();
    vfs_read_write_all(assert_vfs_setup!(Vfs::stdfs(), "vfs_stdfs_read_write_example")).unwrap();
}

fn vfs_read_write_all((vfs, tmpdir): (Vfs, PathBuf)) -> RvResult<()>
{
    let file1 = tmpdir.mash("file1");
    vfs.write_all(&file1, b"this is a test")?;
    assert_eq!(vfs.read_all(&file1)?, "this is a test".to_string());

    assert_vfs_remove_all!(vfs, &tmpdir);
    Ok(())
}

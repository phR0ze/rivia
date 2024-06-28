use rivia::prelude::*;

fn main() {
    // Simply replace this line with `let vfs = Vfs::stdfs();` for the real filesystem
    let vfs = Vfs::memfs();
    let config = load_config(vfs);
    assert_eq!(config, "this is a test");
    println!("VFS test passed");
}

// Load an example application configuration file using VFS.
// This allows you to test with a memory backed VFS implementation during testing and with
// the real filesystem during production.
fn load_config(vfs: Vfs) -> String {
    let dir = PathBuf::from("/etc/xdg");
    vfs.mkdir_p(&dir).unwrap();
    let filepath = dir.mash("rivia.toml");
    vfs.write_all(&filepath, "this is a test").unwrap();
    assert_eq!(vfs.config_dir("rivia.toml").unwrap().to_str().unwrap(), "/etc/xdg");

    if let Some(config_dir) = vfs.config_dir("rivia.toml") {
        let path = config_dir.mash("rivia.toml");
        return vfs.read_all(&path).unwrap();
    }
    "".into()
}

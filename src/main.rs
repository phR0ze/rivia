use rivia_vfs as vfs;

pub const APP_NAME: &str = "Rivia";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const APP_GIT_COMMIT: &str = env!("APP_GIT_COMMIT");
pub const APP_BUILD_DATE: &str = env!("APP_BUILD_DATE");

fn main() {
    println!("{} - {}", APP_NAME, APP_DESCRIPTION);
    println!("{:->w$}", "-", w = 60);
    println!("{:<w$} {}", "Version:", APP_VERSION, w = 18);
    println!("{:<w$} {}", "Build Date:", APP_BUILD_DATE, w = 18);
    println!("{:<w$} {}", "Git Commit:", APP_GIT_COMMIT, w = 18);

    vfs::test();
}

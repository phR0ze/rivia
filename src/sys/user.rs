//! Provides a unified, simplified systems api
//!
//! * Implements the XDB Base Directory Specification <https://wiki.archlinux.org/index.php/XDG_Base_Directory>
//!
//! ### How to use the Rivia `user` module
//! ```
//! use rivia::prelude::*;
//!
//! assert!(user::home_dir().is_ok());
//! ```
use std::{env, path::PathBuf};

use nix::unistd::{Gid, Uid};

use crate::{
    errors::*,
    sys::{self, PathExt},
};

/// Returns the full path to the current user's home directory
///
/// This is an alternate implementation as Rust std::env::home_dir has been deprecated
/// <https://doc.rust-lang.org/std/env/fn.home_dir.html>
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::home_dir().is_ok());
/// ```
pub fn home_dir() -> RvResult<PathBuf> {
    sys::home_dir()
}

/// Returns the full path to the current user's config directory
///
/// * Where user-specific configurations should be written (analogous to /etc)
/// * Defaults to $HOME/.config.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::config_dir().is_ok());
/// ```
pub fn config_dir() -> RvResult<PathBuf> {
    Ok(match env::var("XDG_CONFIG_HOME") {
        Ok(x) => PathBuf::from(x),
        Err(_) => home_dir()?.mash(".config"),
    })
}

/// Returns the full path to the current user's cache directory
///
/// * Where user-specific non-essential (cached) data should be written (analogous to /var/cache)
/// * Defaults to $HOME/.cache.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::cache_dir().is_ok());
/// ```
pub fn cache_dir() -> RvResult<PathBuf> {
    Ok(match env::var("XDG_CACHE_HOME") {
        Ok(x) => PathBuf::from(x),
        Err(_) => home_dir()?.mash(".cache"),
    })
}

/// Returns the full path to the current user's data directory
///
/// * Where user-specific data files should be written
/// * Defaults to $HOME/.local/share
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::data_dir().is_ok());
/// ```
pub fn data_dir() -> RvResult<PathBuf> {
    Ok(match env::var("XDG_DATA_HOME") {
        Ok(x) => PathBuf::from(x),
        Err(_) => home_dir()?.mash(".local").mash("share"),
    })
}

/// Returns the full path to the current user's state directory
///
/// * Where user-specific state files should be written
/// * Defaults to $HOME/.local/state
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::state_dir().is_ok());
/// ```
pub fn state_dir() -> RvResult<PathBuf> {
    Ok(match env::var("XDG_STATE_HOME") {
        Ok(x) => PathBuf::from(x),
        Err(_) => home_dir()?.mash(".local").mash("state"),
    })
}

/// Returns the full path to the current user's runtime directory
///
/// * Used for non-essential, user-specific data files such as sockets, named pipes, etc
/// * Must be owned by the user with an access mode of 0700
/// * Must be on the local filesystem
/// * May be subject to periodic cleanup
/// * Can only exist for the duration of the user's login
/// * Should not store large files as it may be mounted as a tmpfs
/// * Defaults to /tmp
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// println!("runtime directory of the current user: {:?}", user::runtime_dir());
/// ```
pub fn runtime_dir() -> PathBuf {
    match env::var("XDG_RUNTIME_DIR") {
        Ok(x) => PathBuf::from(x),
        Err(_) => PathBuf::from("/tmp"),
    }
}

/// Returns a preferenced-ordered set of system data directories to search for data files
/// in addition to the $XDG_DATA_HOME directory.
///
/// * Defaults to [/usr/local/share, /usr/share]
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::sys_data_dirs().is_ok());
/// ```
pub fn sys_data_dirs() -> RvResult<Vec<PathBuf>> {
    let default = vec![PathBuf::from("/usr/local/share"), PathBuf::from("/usr/share")];
    Ok(match env::var("XDG_DATA_DIRS") {
        Ok(x) => {
            let paths = sys::parse_paths(x)?;
            if paths.is_empty() {
                default
            } else {
                paths
            }
        },
        Err(_) => default,
    })
}

/// Returns a preferenced-ordered set of system configuration directories to search for configuration
/// files in addition to the $XDG_CONFIG_HOME directory.
///
/// * Default configuration files should be installed to /etc/xdg/<appname>/filename
/// * Defaults to [/etc/xdg]
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::sys_config_dirs().is_ok());
/// ```
pub fn sys_config_dirs() -> RvResult<Vec<PathBuf>> {
    let default = vec![PathBuf::from("/etc/xdg")];
    Ok(match env::var("XDG_CONFIG_DIRS") {
        Ok(x) => {
            let paths = sys::parse_paths(x)?;
            if paths.is_empty() {
                default
            } else {
                paths
            }
        },
        Err(_) => default,
    })
}

/// Returns the current user's path directories
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::path_dirs().is_ok());
/// ```
pub fn path_dirs() -> RvResult<Vec<PathBuf>> {
    sys::parse_paths(env::var("PATH")?)
}

/// Provides options for a specific user
#[derive(Debug, Clone, Default)]
pub struct User {
    pub uid: u32,           // user id
    pub gid: u32,           // user group id
    pub name: String,       // user name
    pub home: PathBuf,      // user home
    pub shell: PathBuf,     // user shell
    pub ruid: u32,          // real user id behind sudo
    pub rgid: u32,          // real user group id behind sudo
    pub realname: String,   // real user name behind sudo
    pub realhome: PathBuf,  // real user home behind sudo
    pub realshell: PathBuf, // real user shell behind sudo
}

impl User {
    /// Returns true if the user is root
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// assert_eq!(user::current().unwrap().is_root(), false);
    /// ```
    pub fn is_root(&self) -> bool {
        self.uid == 0
    }
}

/// Get the current user
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::current().is_ok());
/// ```
pub fn current() -> RvResult<User> {
    let user = from_uid(getuid())?;
    Ok(user)
}

/// Get a user by user id
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::from_uid(user::getuid()).is_ok());
/// ```
pub fn from_uid(uid: u32) -> RvResult<User> {
    if let Some(nix_user) = nix::unistd::User::from_uid(Uid::from_raw(uid))? {
        let username = nix_user.name;
        let uid = nix_user.uid.as_raw();
        let gid = nix_user.gid.as_raw();
        // let userpwd = nix_user.passwd;
        let userhome = nix_user.dir;
        let usershell = nix_user.shell;
        // let usergecos = nix_user.gecos;

        // Get the user's real ids as well if applicable
        let (ruid, rgid) = getrids(uid, gid);
        let realuser = if uid != ruid {
            from_uid(ruid)?
        } else {
            User {
                uid,
                gid,
                name: username.to_string(),
                home: PathBuf::from(&userhome),
                shell: PathBuf::from(&usershell),
                ..Default::default()
            }
        };
        Ok(User {
            uid,
            gid,
            name: username,
            home: PathBuf::from(&userhome),
            shell: PathBuf::from(&usershell),
            ruid,
            rgid,
            realname: realuser.name,
            realhome: realuser.home,
            realshell: realuser.shell,
        })
    } else {
        Err(UserError::does_not_exist_by_id(uid).into())
    }
}

/// Switches back to the original user under the sudo mask with no way to go back
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::drop_sudo().is_ok());
/// ```
pub fn drop_sudo() -> RvResult<()> {
    match getuid() {
        0 => {
            let (ruid, rgid) = getrids(0, 0);
            switchuser(ruid, ruid, ruid, rgid, rgid, rgid)
        },
        _ => Ok(()),
    }
}

/// Returns the user ID for the current user
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::getuid() != 0);
/// ```
pub fn getuid() -> u32 {
    nix::unistd::getuid().as_raw()
}

/// Returns the group ID for the current user
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::getgid() != 0);
/// ```
pub fn getgid() -> u32 {
    nix::unistd::getgid().as_raw()
}

/// Returns the user effective ID for the current user
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::geteuid() != 0);
/// ```
pub fn geteuid() -> u32 {
    nix::unistd::geteuid().as_raw()
}

/// Returns the group effective ID for the current user
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert!(user::getegid() != 0);
/// ```
pub fn getegid() -> u32 {
    nix::unistd::getegid().as_raw()
}

/// Returns the real IDs for the given user
///
/// * Peels back the sudo mask to reveal the real user ids
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(user::getrids(user::getuid(), user::getgid()), (user::getuid(), user::getgid()));
/// ```
pub fn getrids(uid: u32, gid: u32) -> (u32, u32) {
    match uid {
        0 => match (env::var("SUDO_UID"), env::var("SUDO_GID")) {
            (Ok(u), Ok(g)) => match (u.parse::<u32>(), g.parse::<u32>()) {
                (Ok(u), Ok(g)) => (u, g),
                _ => (uid, gid),
            },
            _ => (uid, gid),
        },
        _ => (uid, gid),
    }
}

/// Return true if the current user is the root user
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// assert_eq!(user::is_root(), false);
/// ```
pub fn is_root() -> bool {
    getuid() == 0
}

/// Returns the current user's name.
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// println!("current user name: {:?}", user::name().unwrap());
/// ```
pub fn name() -> RvResult<String> {
    Ok(current()?.name)
}

/// Set the user ID for the current user
///
/// ### Examples
/// ```ignore
/// use rivia::prelude::*;
///
/// assert!(user::setuid(user::getuid()).is_ok());
/// ```
pub fn setuid(uid: u32) -> RvResult<()> {
    nix::unistd::setuid(Uid::from_raw(uid))?;
    Ok(())
}

/// Set the user effective ID for the current user
///
/// ### Examples
/// ```ignore
/// use rivia::prelude::*;
///
/// assert!(user::seteuid(user::geteuid()).is_ok());
/// ```
pub fn seteuid(euid: u32) -> RvResult<()> {
    nix::unistd::seteuid(Uid::from_raw(euid))?;
    Ok(())
}

/// Set the group ID for the current user
///
/// ### Examples
/// ```ignore
/// use rivia::prelude::*;
///
/// assert!(user::setgid(user::getgid()).is_ok());
/// ```
pub fn setgid(gid: u32) -> RvResult<()> {
    nix::unistd::setgid(Gid::from_raw(gid))?;
    Ok(())
}

/// Set the group effective ID for the current user
///
/// ### Examples
/// ```ignore
/// use rivia::prelude::*;
///
/// assert!(user::setegid(user::getegid()).is_ok());
/// ```
pub fn setegid(egid: u32) -> RvResult<()> {
    nix::unistd::setegid(Gid::from_raw(egid))?;
    Ok(())
}

/// Raise root privileges for user with root masked off from `sudo_down`
///
/// * Returns an error if not allowed
///
/// ### Examples
/// ```ignore
/// use rivia::prelude::*;
///
/// user:sudo_up().unwrap();
/// ```
pub fn sudo_up() -> RvResult<()> {
    if is_root() {
        return Ok(());
    }
    switchuser(0, 0, 0, 0, 0, 0)
}

/// Switches back to the original user under the sudo mask
///
/// * Preserves the ability to raise sudo again
///
/// ### Examples
/// ```ignore
/// use rivia::prelude::*;
///
/// assert!(user::sudo_down().is_ok());
/// ```
pub fn sudo_down() -> RvResult<()> {
    if !is_root() {
        return Ok(());
    }
    match getuid() {
        0 => {
            let (ruid, rgid) = getrids(0, 0);
            switchuser(ruid, ruid, 0, rgid, rgid, 0)
        },
        _ => Ok(()),
    }
}

/// Switches to another user by setting the real, effective and saved user and group ids
///
/// ### Examples
/// ```ignore
/// use rivia::prelude::*;
///
/// // Switch to user 1000 but preserve root privileges to switch again
/// user::switchuser(1000, 1000, 0, 1000, 1000, 0);
///
/// // Switch to user 1000 and drop root privileges permanantely
/// user::switchuser(1000, 1000, 1000, 1000, 1000, 1000);
/// ```
pub fn switchuser(ruid: u32, euid: u32, suid: u32, rgid: u32, egid: u32, sgid: u32) -> RvResult<()> {
    // Best practice to drop the group first
    nix::unistd::setresgid(Gid::from_raw(rgid), Gid::from_raw(egid), Gid::from_raw(sgid))?;
    nix::unistd::setresuid(Uid::from_raw(ruid), Uid::from_raw(euid), Uid::from_raw(suid))?;
    Ok(())
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use crate::prelude::*;

    #[test]
    fn test_user_home() {
        let home_str = env::var("HOME").unwrap();
        let home_path = PathBuf::from(home_str);
        let home_dir = home_path.parent().unwrap();
        assert_eq!(home_dir.to_path_buf(), user::home_dir().unwrap().dir().unwrap());
    }

    #[test]
    fn test_user_ids() {
        assert!(user::sudo_down().is_ok());
        assert!(user::drop_sudo().is_ok());
        assert!(user::getuid() != 0);
        assert!(user::getgid() != 0);
        assert!(user::geteuid() != 0);
        assert!(user::getegid() != 0);
        assert_eq!(user::getrids(user::getuid(), user::getgid()), (user::getuid(), user::getgid()));
        assert_eq!(user::is_root(), false);
        assert!(user::from_uid(user::getuid()).is_ok());
        assert_ne!(user::name().unwrap(), "");
        assert!(user::current().is_ok());
        assert_eq!(user::current().unwrap().is_root(), false);
        // assert!(user::sudo().is_err());
        // assert!(user::setegid(user::getegid()).is_ok());
        // assert!(user::setgid(user::getgid()).is_ok());
        // assert!(user::seteuid(user::geteuid()).is_ok());

        // if !user::is_root() {
        //     return;
        // }
        // let (vfs, tmpdir) = assert_vfs_setup!(Vfs::stdfs());
        // let file1 = tmpdir.mash("file1");
        // // let file2 = tmpdir.mash("file2");

        // // Create a file with the current user
        // assert_vfs_mkfile!(vfs, &file1);
        // assert_vfs_exists!(vfs, &file1);
        // assert_ne!(Stdfs::uid(&file1).unwrap(), 0);
        // assert_ne!(Stdfs::gid(&file1).unwrap(), 0);
        // assert_vfs_remove_all!(vfs, &tmpdir);

        // // Now escalate via sudo
        // // assert!(exec::sudo().is_ok());
        // // assert_eq!(user::getuid(), 0);
        // // assert_eq!(user::getgid(), 0);
        // // assert_eq!(user::is_root(), true);
        // // assert_eq!(Stdfs::mkfile(&file1).unwrap(), file1);

        // assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_user_dirs() {
        assert!(user::home_dir().is_ok());
        assert!(user::config_dir().is_ok());
        assert!(user::cache_dir().is_ok());
        assert!(user::data_dir().is_ok());
        user::runtime_dir();
        assert!(user::sys_data_dirs().is_ok());
        assert!(user::sys_config_dirs().is_ok());
        assert!(user::path_dirs().is_ok());
    }
}

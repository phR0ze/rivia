use std::path::PathBuf;

use crate::{
    errors::{RvResult, VfsError},
    sys::{Entry, VfsEntry},
};

/// Provides a builder pattern for flexibly changing file permissions
///
/// Use the Vfs functions `chmod_b` to create a new instance followed by one or more options and
/// complete the operation by calling `exec`.
///
/// # Octal form
/// `Chmod` supports the standard Linux octal permissions values via the `dirs`, `files` and `all`
/// options to set permissions to directories, files or both distictly at the same time. The octal
/// form will takes precedence over the symbolic form if both are set.
///
/// Octal  Binary  File Mode
/// 0      000
/// 1      001     --x
/// 2      010     -w-
/// 3      011     -wx
/// 4      100     r--
/// 5      101     r-x
/// 6      110     rw-
/// 7      111     rwx
///
/// # Symbolic form
/// `Chmod` supports a symbol form via the `sym` option, inspired by linux's chmod. The supported
/// syntax is a repeatable pattern following this form `[dfa]:[ugoa][-+=][rwx]`. All segments are
/// required. The first segment calls out the target filesystem type i.e. `d` directories, `f` files
/// or `a` both. The second segment is separated from the first by a colon and calls out the group
/// to target i.e. `u` user, `g` group, `o` other, or `a` all. The second segment calls out the
/// operation to perform `-` subtractive, `+` addative, or `=` an assignment. The third segment
/// calls out the permission to subtracet, add or assign. Finally the pattern can be repeated by
/// separating repetitions with a comma.
///
/// ```
/// use rivia::prelude::*;
///
/// let vfs = Memfs::new();
/// let file = vfs.root().mash("file");
/// assert_vfs_mkfile!(vfs, &file);
/// assert_eq!(vfs.is_exec(&file), false);
/// assert!(vfs.chmod_b(&file).unwrap().sym("f:a+x").exec().is_ok());
/// assert_eq!(vfs.is_exec(&file), true);
/// ```
pub struct Chmod {
    pub(crate) opts: ChmodOpts,
    pub(crate) exec: Box<dyn Fn(ChmodOpts) -> RvResult<()>>, // provider callback
}

// Internal type used to encapsulate just the options. This separates the provider implementation
// from the options allowing for sharing options between different vfs providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChmodOpts {
    pub(crate) path: PathBuf,   // path to chmod
    pub(crate) dirs: u32,       // mode to use for dirs
    pub(crate) files: u32,      // mode to use for files
    pub(crate) follow: bool,    // follow links
    pub(crate) recursive: bool, // chmod recursively
    pub(crate) sym: String,     // add permissions via symbols
}

impl Chmod {
    /// Set the permissions to use for both directories and files
    ///
    /// * Uses the standard linux octal form
    /// * Takes precedence over any symbolic settings set with `sym`
    /// * The operations `all`, `files` and `dirs` are mutually exclusive with `sym`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.chmod_b(&dir).unwrap().recurse().all(0o777).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40777);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100777);
    /// ```
    pub fn all(mut self, mode: u32) -> Self {
        self.opts.dirs = mode;
        self.opts.files = mode;
        self
    }

    /// Set the permissions to use for directories only
    ///
    /// * Uses the standard linux octal form
    /// * Takes precedence over any symbolic settings set with `sym`
    /// * The operations `all`, `files` and `dirs` are mutually exclusive with `sym`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.chmod_b(&dir).unwrap().recurse().dirs(0o755).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40755);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// ```
    pub fn dirs(mut self, mode: u32) -> Self {
        self.opts.dirs = mode;
        self
    }

    /// Set the permissions to use for files only
    ///
    /// * Uses the standard linux octal form
    /// * Takes precedence over any symbolic settings set with `sym`
    /// * The operations `all`, `files` and `dirs` are mutually exclusive with `sym`
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.chmod_b(&dir).unwrap().recurse().files(0o600).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40755);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100600);
    /// ```
    pub fn files(mut self, mode: u32) -> Self {
        self.opts.files = mode;
        self
    }

    /// Follow links so that the directories/files they point to are also affected
    ///
    /// * Default: false
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let file = vfs.root().mash("file");
    /// let link = vfs.root().mash("link");
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert_vfs_symlink!(vfs, &link, &file);
    /// assert!(vfs.chmod_b(&link).unwrap().follow().files(0o600).exec().is_ok());
    /// assert_eq!(vfs.mode(&link).unwrap(), 0o120777);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100600);
    /// ```
    pub fn follow(mut self) -> Self {
        self.opts.follow = true;
        self
    }

    /// Remove write and execute permissions for all groups for files only
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.chmod_b(&dir).unwrap().readonly().exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40755);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100444);
    /// ```
    pub fn readonly(mut self) -> Self {
        self.opts.sym = "f:a+r,f:a-wx".to_string();
        self
    }

    /// Follow paths recursively
    ///
    /// * Default: true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.chmod_b(&dir).unwrap().recurse().all(0o777).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40777);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100777);
    /// ```
    pub fn recurse(mut self) -> Self {
        self.opts.recursive = true;
        self
    }

    /// Don't follow paths recursively
    ///
    /// * Default: true
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.chmod_b(&dir).unwrap().no_recurse().all(0o777).exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40777);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100644);
    /// ```
    pub fn no_recurse(mut self) -> Self {
        self.opts.recursive = false;
        self
    }

    /// Drop all permissions for group and other so that only user permissions remain.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.chmod_b(&dir).unwrap().secure().exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40700);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100600);
    /// ```
    pub fn secure(mut self) -> Self {
        self.opts.sym = "a:go-rwx".to_string();
        self
    }

    /// Update the `mode` using symbols inspired by linux's chmod
    ///
    /// * Uses the following repeatable pattern `[dfa]:[ugoa][-+=][rwx]`
    /// * All segments are required
    /// * The first segment calls out the target filesystem type i.e. `d` directories, `f` files or
    ///   `a` both.
    /// * The second segment is separated from the first by a colon and calls out the group to
    ///   target i.e. `u` user, `g` group, `o` other, or `a` all.
    /// * The second segment calls out the operation to perform `-` subtractive, `+` addative, or
    ///   `=` an assignment.
    /// * The third segment calls out the permission to subtract, add or assign.
    /// * Finally the pattern can be repeated by separating repetitions with a comma.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.chmod_b(&dir).unwrap().sym("a:go-rwx").exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40700);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100600);
    /// ```
    pub fn sym(mut self, symbolic: &str) -> Self {
        self.opts.sym = symbolic.into();
        self
    }

    /// Execute the [`Chmod`] options against the path provided during construction with the Vfs
    /// `chmod_b` functions.
    ///
    /// ### Examples
    /// ```
    /// use rivia::prelude::*;
    ///
    /// let vfs = Memfs::new();
    /// let dir = vfs.root().mash("dir");
    /// let file = dir.mash("file");
    /// assert_vfs_mkdir_p!(vfs, &dir);
    /// assert_vfs_mkfile!(vfs, &file);
    /// assert!(vfs.chmod_b(&dir).unwrap().sym("a:go-rwx").exec().is_ok());
    /// assert_eq!(vfs.mode(&dir).unwrap(), 0o40700);
    /// assert_eq!(vfs.mode(&file).unwrap(), 0o100600);
    /// ```
    pub fn exec(&self) -> RvResult<()> {
        (self.exec)(self.opts.clone())
    }
}

// Symbolic mode state machine states
#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    Target,
    Group,
    Perms,
}

/// Update the `mode` using symbols inspired by linux's chmod if given
///
/// * Octal mode takes priority if given
/// * Symbolic mode takes the following repeatable pattern `[dfa]:[ugoa][-+=][rwx]`
/// * All segments are required, repeats are comma separated
/// * The 1st seg calls out the entry type i.e. `d` directories, `f` files or `a` both
/// * The 2nd seg is separated from the first by a colon and calls out the group to target i.e. `u`
///   user, `g` group, `o` other, or `a` all
/// * The 3rd seg calls out the operation to perform `-` subtractive, `+` addative, or `=` an
///   assignment
/// * The fourth segment calls out the permission to subtract, add or assign
pub(crate) fn mode(entry: &VfsEntry, octal: u32, sym: &str) -> RvResult<u32> {
    // Octal mode takes priority
    if octal != 0 {
        return Ok(octal);
    }

    // No octal and no symbolic form given
    if sym.is_empty() {
        return Ok(0);
    }

    // Start from the entry's mode and apply symbolic manipulations
    let mut mode = entry.mode();
    let mut group = 0;
    let mut op = '0';
    let mut chars: Vec<char> = sym.chars().rev().collect();

    let mut state = State::Target;
    while let Some(mut c) = chars.pop() {
        match state {
            State::Target => {
                group = 0; // reset group for next chmod
                op = '0'; // reset op for next chmod

                loop {
                    if c != 'd' && c != 'f' && c != 'a' && c != ':' {
                        return Err(VfsError::InvalidChmodTarget(sym.to_string()).into());
                    }
                    if entry.is_symlink() || (c == 'd' && !entry.is_dir()) || (c == 'f' && !entry.is_file()) {
                        return Ok(mode); // target mismatch so just return the original mode
                    } else if c == ':' {
                        state = State::Group;
                        break;
                    }
                    c = _pop(&mut chars, sym)?;
                }
            },
            State::Group => {
                loop {
                    match c {
                        'u' => group |= 0o0700,
                        'g' => group |= 0o0070,
                        'o' => group |= 0o0007,
                        'a' => group |= 0o0777,
                        '-' | '+' | '=' => {
                            op = c;
                            state = State::Perms;
                            break;
                        },
                        _ => return Err(VfsError::InvalidChmodGroup(sym.to_string()).into()),
                    }
                    c = _pop(&mut chars, sym)?;
                }
                if group == 0 {
                    return Err(VfsError::InvalidChmodGroup(sym.to_string()).into());
                }
                if op == '0' {
                    return Err(VfsError::InvalidChmodOp(sym.to_string()).into());
                }
            },
            State::Perms => {
                let mut perm = 0;
                while state == State::Perms {
                    match c {
                        'r' | 'w' | 'x' => {
                            // Accumulate current permission
                            match c {
                                'r' => perm |= 0o0444,
                                'w' => perm |= 0o0222,
                                _ => perm |= 0o0111,
                            }

                            // Get next permission or break if done
                            if !chars.is_empty() {
                                c = chars.pop().unwrap();
                            } else {
                                break;
                            }
                        },
                        ',' => {
                            state = State::Target;
                        },
                        _ => return Err(VfsError::InvalidChmodPermissions(sym.to_string()).into()),
                    }
                }
                if perm == 0 {
                    return Err(VfsError::InvalidChmodPermissions(sym.to_string()).into());
                }

                // Process permission
                match op {
                    '-' => mode &= !(group & perm),
                    '+' => mode |= group & perm,
                    _ => mode = (!group & mode) | (group & perm),
                }
            },
        }
    }

    Ok(mode)
}

// handle pop gracefully
fn _pop(chars: &mut Vec<char>, sym: &str) -> RvResult<char> {
    if !chars.is_empty() {
        Ok(chars.pop().unwrap())
    } else {
        Err(VfsError::InvalidChmod(sym.to_string()).into())
    }
}

// Returns true if the new mode is revoking permissions as compared to the old mode as pertains
// directory read/execute permissions. This is useful when recursively modifying file
// permissions.
pub(crate) fn revoking_mode(old: u32, new: u32) -> bool {
    old & 0o0500 > new & 0o0500 || old & 0o0050 > new & 0o0050 || old & 0o0005 > new & 0o0005
}

// Unit tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_vfs_chmod() {
        test_chmod(assert_vfs_setup!(Vfs::memfs()));
        test_chmod(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_chmod((vfs, tmpdir): (Vfs, PathBuf)) {
        let file1 = tmpdir.mash("file1");
        assert_vfs_mkfile!(vfs, &file1);
        assert!(vfs.chmod(&file1, 0o644).is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100644);
        assert!(vfs.chmod(&file1, 0o555).is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100555);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_chmod_b() {
        test_chmod_b(assert_vfs_setup!(Vfs::memfs()));
        test_chmod_b(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_chmod_b((vfs, tmpdir): (Vfs, PathBuf)) {
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");

        // setup
        assert_eq!(vfs.mkdir_p(&dir1).unwrap(), dir1);
        assert_eq!(vfs.mkdir_p(&dir2).unwrap(), dir2);
        assert_eq!(vfs.mkfile_m(&file1, 0o644).unwrap(), file1);
        assert_eq!(vfs.mkfile_m(&file2, 0o644).unwrap(), file2);

        // all files
        assert!(vfs.chmod_b(&dir1).unwrap().all(0o600).exec().is_ok());
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40600);

        // now fix dirs only to allow for listing directries
        assert!(vfs.chmod_b(&dir1).unwrap().dirs(0o755).exec().is_ok());
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100600);
        assert_eq!(vfs.mode(&dir2).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file2).unwrap(), 0o100600);

        // now change just the files back to 644
        assert!(vfs.chmod_b(&dir1).unwrap().files(0o644).exec().is_ok());
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100644);
        assert_eq!(vfs.mode(&dir2).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file2).unwrap(), 0o100644);

        // set all back to 0o600 and then set both at the same time
        assert!(vfs.chmod_b(&dir1).unwrap().all(0o600).exec().is_ok());
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40600);
        assert!(vfs.chmod_b(&dir1).unwrap().dirs(0o755).files(0o644).exec().is_ok());
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100644);
        assert_eq!(vfs.mode(&dir2).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file2).unwrap(), 0o100644);

        // doesn't exist
        assert!(vfs.chmod_b("bogus").unwrap().all(0o644).exec().is_err());

        // no path given
        assert!(vfs.chmod_b("").is_err());

        // cleanup
        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_chmod_b_symbolic() {
        test_chmod_b_symbolic(assert_vfs_setup!(Vfs::memfs()));
        test_chmod_b_symbolic(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_chmod_b_symbolic((vfs, tmpdir): (Vfs, PathBuf)) {
        let file1 = tmpdir.mash("file1");

        // setup
        assert!(vfs.mkfile_m(&file1, 0o644).is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100644);
        assert_eq!(vfs.is_exec(&file1), false);

        // add_x
        assert!(vfs.chmod_b(&file1).unwrap().sym("f:a+x").exec().is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100755);
        assert_eq!(vfs.is_exec(&file1), true);

        // sub_x
        assert!(vfs.chmod_b(&file1).unwrap().sym("f:a-x").exec().is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100644);
        assert_eq!(vfs.is_exec(&file1), false);

        // sub_w
        assert!(vfs.chmod_b(&file1).unwrap().sym("f:a-w").exec().is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100444);
        assert_eq!(vfs.is_readonly(&file1), true);

        // add_w
        assert!(vfs.chmod_b(&file1).unwrap().sym("f:a+w").exec().is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100666);
        assert_eq!(vfs.is_readonly(&file1), false);

        // sub_r
        assert!(vfs.chmod_b(&file1).unwrap().sym("f:a-r").exec().is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100222);

        // add_r
        assert!(vfs.chmod_b(&file1).unwrap().sym("f:a+r").exec().is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100666);

        // secure
        assert!(vfs.chmod_b(&file1).unwrap().sym("f:a+rwx").exec().is_ok());
        assert!(vfs.chmod_b(&file1).unwrap().secure().exec().is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100700);

        // readonly
        assert!(vfs.chmod_b(&file1).unwrap().readonly().exec().is_ok());
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100444);
        assert_eq!(vfs.is_readonly(&file1), true);

        // cleanup
        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_chmod_follow() {
        test_chmod_follow(assert_vfs_setup!(Vfs::memfs()));
        test_chmod_follow(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_chmod_follow((vfs, tmpdir): (Vfs, PathBuf)) {
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let link1 = tmpdir.mash("link1");

        assert_eq!(vfs.mkdir_m(&dir1, 0o777).unwrap(), dir1);
        assert_eq!(vfs.mkfile_m(&file1, 0o777).unwrap(), file1);
        assert_eq!(vfs.symlink(&link1, &dir1).unwrap(), link1);

        // no follow = no change for link, dir or file
        assert_eq!(vfs.mode(&link1).unwrap(), 0o120777);
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40777);
        assert!(vfs.chmod(&link1, 0o555).is_ok());
        assert_eq!(vfs.mode(&link1).unwrap(), 0o120777);
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40777);
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100777);

        // follow but no recurse = no change for for link, dir or file
        assert!(vfs.chmod_b(&link1).unwrap().no_recurse().dirs(0o755).files(0o444).exec().is_ok());
        assert_eq!(vfs.mode(&link1).unwrap(), 0o120777);
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40777);
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100777);

        // follow with recurse = no chang for link but change dir and file
        assert!(vfs.chmod_b(&link1).unwrap().follow().dirs(0o755).files(0o444).exec().is_ok());
        assert_eq!(vfs.mode(&link1).unwrap(), 0o120777);
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100444);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_chmod_recurse() {
        test_chmod_recurse(assert_vfs_setup!(Vfs::memfs()));
        test_chmod_recurse(assert_vfs_setup!(Vfs::stdfs()));
    }
    fn test_chmod_recurse((vfs, tmpdir): (Vfs, PathBuf)) {
        let dir1 = tmpdir.mash("dir1");
        let file1 = dir1.mash("file1");
        let dir2 = dir1.mash("dir2");
        let file2 = dir2.mash("file2");

        assert_eq!(vfs.mkdir_m(&dir2, 0o777).unwrap(), dir2);
        assert_eq!(vfs.mkfile_m(&file1, 0o777).unwrap(), file1);
        assert_eq!(vfs.mkfile_m(&file2, 0o777).unwrap(), file2);
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40777);
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100777);
        assert_eq!(vfs.mode(&dir2).unwrap(), 0o40777);
        assert_eq!(vfs.mode(&file2).unwrap(), 0o100777);

        // no recurse = dir1 is the only chmod that will occur
        assert!(vfs.chmod_b(&dir1).unwrap().no_recurse().dirs(0o755).files(0o644).exec().is_ok());
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100777);
        assert_eq!(vfs.mode(&dir2).unwrap(), 0o40777);
        assert_eq!(vfs.mode(&file2).unwrap(), 0o100777);

        // recurse, default behavior
        assert!(vfs.chmod_b(&dir1).unwrap().dirs(0o755).files(0o644).exec().is_ok());
        assert_eq!(vfs.mode(&dir1).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file1).unwrap(), 0o100644);
        assert_eq!(vfs.mode(&dir2).unwrap(), 0o40755);
        assert_eq!(vfs.mode(&file2).unwrap(), 0o100644);

        assert_vfs_remove_all!(vfs, &tmpdir);
    }

    #[test]
    fn test_vfs_chmod_symbolic() {
        test_chmod_symbolic(
            Box::new(|m: u32| -> VfsEntry {
                let mut entry = MemfsEntry::opts(PathBuf::new()).dir().build();
                entry.mode = m;
                entry.upcast()
            }),
            Box::new(|m: u32| -> VfsEntry {
                let mut entry = MemfsEntry::opts(PathBuf::new()).file().build();
                entry.mode = m;
                entry.upcast()
            }),
        );

        test_chmod_symbolic(
            Box::new(|m: u32| -> VfsEntry {
                StdfsEntry {
                    path: PathBuf::new(),
                    alt: PathBuf::new(),
                    rel: PathBuf::new(),
                    dir: true,
                    file: false,
                    link: false,
                    mode: m,
                    follow: false,
                    cached: false,
                }
                .upcast()
            }),
            Box::new(|m: u32| -> VfsEntry {
                StdfsEntry {
                    path: PathBuf::new(),
                    alt: PathBuf::new(),
                    rel: PathBuf::new(),
                    dir: false,
                    file: true,
                    link: false,
                    mode: m,
                    follow: false,
                    cached: false,
                }
                .upcast()
            }),
        );
    }
    fn test_chmod_symbolic(d: Box<dyn Fn(u32) -> VfsEntry>, f: Box<dyn Fn(u32) -> VfsEntry>) {
        assert_eq!(0o0700 & 0o0444 | 0o0000, 0o0400); // u+r
        assert_eq!(0o0770 & 0o0444 | 0o0000, 0o0440); // ug+r
        assert_eq!(!(0o0700 & 0o0444) & 0o0444, 0o0044); // u-r
        assert_eq!(!(0o0770 & 0o0444) & 0o0444, 0o0004); // ug-r

        // Repeating tests
        // -----------------------------------------------------------------------------------------
        assert_eq!(sys::mode(&f(0o0000), 0, "a:a=rwx,a:g=rw,a:o=r").unwrap(), 0o0764);
        assert_eq!(sys::mode(&f(0o0077), 0, "f:u=rwx,f:g=rw,f:o=r").unwrap(), 0o0764);
        assert_eq!(sys::mode(&f(0o0077), 0, "f:u=rwx,f:g=rw,f:o-rwx").unwrap(), 0o0760);

        // Target tests
        // -----------------------------------------------------------------------------------------

        // bad target type
        assert_eq!(
            sys::mode(&f(0o0300), 0, "sf:u+r").unwrap_err().to_string(),
            "Invalid chmod target given: sf:u+r"
        );

        // mismatch on target type
        assert_eq!(sys::mode(&d(0o0300), 0, "f:u+r").unwrap(), 0o0300);
        assert_eq!(sys::mode(&f(0o0300), 0, "d:u+r").unwrap(), 0o0300);

        // any target type
        assert_eq!(sys::mode(&d(0o0300), 0, "a:u+r").unwrap(), 0o0700);
        assert_eq!(sys::mode(&f(0o0300), 0, "a:u+r").unwrap(), 0o0700);

        // multiple targets
        assert_eq!(sys::mode(&d(0o0300), 0, "ad:u+r").unwrap(), 0o0700);
        assert_eq!(sys::mode(&f(0o0300), 0, "af:u+r").unwrap(), 0o0700);

        // Group tests
        // -----------------------------------------------------------------------------------------

        // single group
        assert_eq!(sys::mode(&d(0o0300), 0, "d:u+r").unwrap(), 0o0700);
        assert_eq!(sys::mode(&d(0o0300), 0, "d:g+r").unwrap(), 0o0340);
        assert_eq!(sys::mode(&d(0o0300), 0, "d:o+r").unwrap(), 0o0304);

        assert_eq!(sys::mode(&f(0o0300), 0, "f:u+r").unwrap(), 0o0700);
        assert_eq!(sys::mode(&f(0o0300), 0, "f:g+r").unwrap(), 0o0340);
        assert_eq!(sys::mode(&f(0o0300), 0, "f:o+r").unwrap(), 0o0304);

        // multiple groups
        assert_eq!(sys::mode(&f(0o0300), 0, "f:ug+r").unwrap(), 0o0740);
        assert_eq!(sys::mode(&f(0o0300), 0, "f:uo+r").unwrap(), 0o0704);
        assert_eq!(sys::mode(&f(0o0300), 0, "f:go+r").unwrap(), 0o0344);
        assert_eq!(sys::mode(&f(0o0300), 0, "f:gu+r").unwrap(), 0o0740);
        assert_eq!(sys::mode(&f(0o0300), 0, "f:gugugu+r").unwrap(), 0o0740);

        // all groups
        assert_eq!(sys::mode(&f(0o0300), 0, "f:a+r").unwrap(), 0o0744);
        assert_eq!(sys::mode(&f(0o0300), 0, "f:ugo+r").unwrap(), 0o0744);
        assert_eq!(sys::mode(&f(0o0300), 0, "f:ugoa+r").unwrap(), 0o0744);

        // Permission tests
        // -----------------------------------------------------------------------------------------

        // assign permissions
        assert_eq!(sys::mode(&f(0o0000), 0, "f:u=rw").unwrap(), 0o0600);
        assert_eq!(sys::mode(&f(0o0100), 0, "f:u=rw").unwrap(), 0o0600);
        assert_eq!(sys::mode(&f(0o0200), 0, "f:u=rw").unwrap(), 0o0600);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u=rw").unwrap(), 0o0600);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u=rwrw").unwrap(), 0o0600);

        // sub user execute
        assert_eq!(sys::mode(&f(0o0000), 0, "f:u-x").unwrap(), 0o0000);
        assert_eq!(sys::mode(&f(0o0100), 0, "f:u-x").unwrap(), 0o0000);
        assert_eq!(sys::mode(&f(0o0200), 0, "f:u-x").unwrap(), 0o0200);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u-x").unwrap(), 0o0400);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u-xx").unwrap(), 0o0400);

        // sub user write
        assert_eq!(sys::mode(&f(0o0000), 0, "f:u-w").unwrap(), 0o0000);
        assert_eq!(sys::mode(&f(0o0100), 0, "f:u-w").unwrap(), 0o0100);
        assert_eq!(sys::mode(&f(0o0200), 0, "f:u-w").unwrap(), 0o0000);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u-w").unwrap(), 0o0400);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u-www").unwrap(), 0o0400);

        // add user write
        assert_eq!(sys::mode(&f(0o0000), 0, "f:u+w").unwrap(), 0o0200);
        assert_eq!(sys::mode(&f(0o0100), 0, "f:u+w").unwrap(), 0o0300);
        assert_eq!(sys::mode(&f(0o0200), 0, "f:u+w").unwrap(), 0o0200);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u+w").unwrap(), 0o0600);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u+www").unwrap(), 0o0600);

        // add user execute
        assert_eq!(sys::mode(&f(0o0000), 0, "f:u+x").unwrap(), 0o0100);
        assert_eq!(sys::mode(&f(0o0100), 0, "f:u+x").unwrap(), 0o0100);
        assert_eq!(sys::mode(&f(0o0200), 0, "f:u+x").unwrap(), 0o0300);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u+x").unwrap(), 0o0500);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u+xx").unwrap(), 0o0500);

        // add user all
        assert_eq!(sys::mode(&f(0o0000), 0, "f:u+rwx").unwrap(), 0o0700);
        assert_eq!(sys::mode(&f(0o0100), 0, "f:u+rwx").unwrap(), 0o0700);
        assert_eq!(sys::mode(&f(0o0200), 0, "f:u+rwx").unwrap(), 0o0700);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u+rwx").unwrap(), 0o0700);
        assert_eq!(sys::mode(&f(0o0400), 0, "f:u+rwxrwx").unwrap(), 0o0700);
    }

    #[test]
    fn test_revoking_mode() {
        // test other octet
        assert_eq!(sys::revoking_mode(0o0777, 0o0777), false);
        assert_eq!(sys::revoking_mode(0o0776, 0o0775), false);
        assert_eq!(sys::revoking_mode(0o0770, 0o0771), false);
        assert_eq!(sys::revoking_mode(0o0776, 0o0772), true);
        assert_eq!(sys::revoking_mode(0o0775, 0o0776), true);
        assert_eq!(sys::revoking_mode(0o0775, 0o0774), true);

        // Test group octet
        assert_eq!(sys::revoking_mode(0o0777, 0o0777), false);
        assert_eq!(sys::revoking_mode(0o0767, 0o0757), false);
        assert_eq!(sys::revoking_mode(0o0707, 0o0717), false);
        assert_eq!(sys::revoking_mode(0o0767, 0o0727), true);
        assert_eq!(sys::revoking_mode(0o0757, 0o0767), true);
        assert_eq!(sys::revoking_mode(0o0757, 0o0747), true);

        // Test owner octet
        assert_eq!(sys::revoking_mode(0o0777, 0o0777), false);
        assert_eq!(sys::revoking_mode(0o0677, 0o0577), false);
        assert_eq!(sys::revoking_mode(0o0077, 0o0177), false);
        assert_eq!(sys::revoking_mode(0o0677, 0o0277), true);
        assert_eq!(sys::revoking_mode(0o0577, 0o0677), true);
        assert_eq!(sys::revoking_mode(0o0577, 0o0477), true);
        assert_eq!(sys::revoking_mode(0o0577, 0o0177), true);
    }
}

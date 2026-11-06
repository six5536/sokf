//! This module reads and sets the executable bit of a file of a pack's
//! `files/` in the working tree. A platform without the bit, such as
//! Windows, has no mode: the module reads every file as not executable
//! there, and sets nothing, and its callers compare no bit there (the
//! decision "The executable bit" of design-pack).

use std::{fs::Metadata, io, path::Path};

/// `is_executable` returns whether the file of `meta` is executable by its
/// owner.
fn is_executable(meta: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o100 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        false
    }
}

/// `file_is_executable` returns whether the file `path` exists and is
/// executable by its owner.
pub fn file_is_executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| is_executable(&m))
}

/// `set_executable` gives the file `path` the executable bit, for each
/// class that may read it, or takes the bit away from every class.
// sokf-implements: CLI-13_AC-59
pub fn set_executable(path: &Path, executable: bool) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        let mode = perms.mode();
        let mode = if executable {
            // Each class that may read the file may run it.
            mode | ((mode & 0o444) >> 2)
        } else {
            mode & !0o111
        };
        perms.set_mode(mode);
        std::fs::set_permissions(path, perms)
    }
    #[cfg(not(unix))]
    {
        let _ = (path, executable);
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn the_bit_follows_each_class_that_reads_the_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("sokf-mode-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("run.sh");
        std::fs::write(&file, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();
        assert!(!file_is_executable(&file));
        set_executable(&file, true).unwrap();
        let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o750);
        assert!(file_is_executable(&file));
        set_executable(&file, false).unwrap();
        let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

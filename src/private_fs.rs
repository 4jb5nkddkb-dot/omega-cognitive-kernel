//! Private-by-default creation helpers for persistence artifacts.

use std::fs::{self, OpenOptions};
use std::io;
use std::path::Path;

/// Create missing directories with owner-only permissions on Unix.
/// Existing directories are deliberately left unchanged.
pub(crate) fn create_dir_all(path: &Path) -> io::Result<()> {
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700).create(path)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(path)
    }
}

/// Return OpenOptions configured for owner-only permissions when creating a file.
/// The mode is only applied by the OS when the file is newly created; opening an
/// existing file does not chmod it.
pub(crate) fn open_options() -> OpenOptions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = OpenOptions::new();
        options.mode(0o600);
        options
    }
    #[cfg(not(unix))]
    {
        OpenOptions::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[cfg(unix)]
    #[test]
    fn new_directory_and_file_are_private_but_existing_mode_is_unchanged() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir().join(format!("omega-private-fs-{}", crate::new_uuid()));
        create_dir_all(&dir).unwrap();
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );

        let path = dir.join("new.json");
        open_options()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        open_options()
            .append(true)
            .create(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        let _ = fs::remove_dir_all(dir);
    }
}

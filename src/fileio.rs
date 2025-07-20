use std::fs;
use std::io;
use std::path::Path;

/// Read the entire contents of a file into a string.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path)
}

/// Write bytes to a file, creating parent directories if needed.
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

/// Recursively create a directory and all of its parents.
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    fs::create_dir_all(path)
}

/// Recursively remove a directory.
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    fs::remove_dir_all(path)
}

/// Remove a single file if it exists.
pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    fs::remove_file(path)
}

/// Rename or move a file, creating the destination directory if needed.
pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    if let Some(parent) = to.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(from, to)
}

/// Set executable permissions on Unix platforms.
#[cfg(unix)]
pub fn set_executable<P: AsRef<Path>>(path: P) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perm = fs::Permissions::from_mode(0o755);
    fs::set_permissions(path, perm)
}

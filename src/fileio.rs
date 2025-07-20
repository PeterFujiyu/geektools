use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;

/// Read file content as UTF-8 string
pub fn read(path: impl AsRef<Path>) -> io::Result<String> {
    fs::read_to_string(path)
}

/// Write a UTF-8 string to file, creating parent directories if needed
pub fn write(path: impl AsRef<Path>, data: &str) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, data)
}

/// Write raw bytes to a file, creating parent directories if needed
pub fn write_bytes(path: impl AsRef<Path>, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, data)
}

/// Open a file in append mode, creating parent directories if needed
pub fn open_append(path: impl AsRef<Path>) -> io::Result<File> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// Recursively create a directory
pub fn create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(path)
}

/// Remove a single file
pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    fs::remove_file(path)
}

/// Remove a directory recursively
pub fn remove_dir(path: impl AsRef<Path>) -> io::Result<()> {
    fs::remove_dir_all(path)
}

/// Rename a file
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    if let Some(parent) = to.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::rename(from, to)
}

#[cfg(unix)]
/// Set executable permission (Unix only)
pub fn set_executable(path: impl AsRef<Path>) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = fs::metadata(&path)?.permissions();
    perm.set_mode(0o755);
    fs::set_permissions(path, perm)
}

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;
use crate::errors::{GeekToolsError, Result};

/// Read file content as UTF-8 string
pub fn read(path: impl AsRef<Path>) -> Result<String> {
    fs::read_to_string(&path).map_err(|e| GeekToolsError::FileOperationError {
        path: path.as_ref().display().to_string(),
        source: e,
    })
}

/// Write a UTF-8 string to file, creating parent directories if needed
pub fn write(path: impl AsRef<Path>, data: &str) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| GeekToolsError::FileOperationError {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }
    fs::write(&path, data).map_err(|e| GeekToolsError::FileOperationError {
        path: path.as_ref().display().to_string(),
        source: e,
    })
}

/// Write raw bytes to a file, creating parent directories if needed
pub fn write_bytes(path: impl AsRef<Path>, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| GeekToolsError::FileOperationError {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }
    fs::write(&path, data).map_err(|e| GeekToolsError::FileOperationError {
        path: path.as_ref().display().to_string(),
        source: e,
    })
}

/// Open a file in append mode, creating parent directories if needed
pub fn open_append(path: impl AsRef<Path>) -> Result<File> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| GeekToolsError::FileOperationError {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }
    OpenOptions::new().create(true).append(true).open(&path).map_err(|e| GeekToolsError::FileOperationError {
        path: path.as_ref().display().to_string(),
        source: e,
    })
}

/// Recursively create a directory
pub fn create_dir(path: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&path).map_err(|e| GeekToolsError::FileOperationError {
        path: path.as_ref().display().to_string(),
        source: e,
    })
}

/// Remove a single file
pub fn remove_file(path: impl AsRef<Path>) -> Result<()> {
    fs::remove_file(&path).map_err(|e| GeekToolsError::FileOperationError {
        path: path.as_ref().display().to_string(),
        source: e,
    })
}

/// Remove a directory recursively
pub fn remove_dir(path: impl AsRef<Path>) -> Result<()> {
    fs::remove_dir_all(&path).map_err(|e| GeekToolsError::FileOperationError {
        path: path.as_ref().display().to_string(),
        source: e,
    })
}

/// Rename a file
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    if let Some(parent) = to.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| GeekToolsError::FileOperationError {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }
    fs::rename(&from, &to).map_err(|e| GeekToolsError::FileOperationError {
        path: format!("{} -> {}", from.as_ref().display(), to.as_ref().display()),
        source: e,
    })
}

#[cfg(unix)]
/// Set executable permission (Unix only)
pub fn set_executable(path: impl AsRef<Path>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = fs::metadata(&path).map_err(|e| GeekToolsError::FileOperationError {
        path: path.as_ref().display().to_string(),
        source: e,
    })?.permissions();
    perm.set_mode(0o755);
    fs::set_permissions(&path, perm).map_err(|e| GeekToolsError::FileOperationError {
        path: path.as_ref().display().to_string(),
        source: e,
    })
}

// Keep backward compatibility functions
pub mod compat {
    use super::*;
    
    /// Backward compatibility wrapper for read operation
    pub fn read_compat(path: impl AsRef<Path>) -> io::Result<String> {
        super::read(path).map_err(|e| match e {
            GeekToolsError::FileOperationError { source, .. } => source,
            _ => io::Error::new(io::ErrorKind::Other, e.to_string()),
        })
    }
    
    /// Backward compatibility wrapper for write operation
    pub fn write_compat(path: impl AsRef<Path>, data: &str) -> io::Result<()> {
        super::write(path, data).map_err(|e| match e {
            GeekToolsError::FileOperationError { source, .. } => source,
            _ => io::Error::new(io::ErrorKind::Other, e.to_string()),
        })
    }
}

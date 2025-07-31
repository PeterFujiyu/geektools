use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use std::time::{SystemTime, Duration};
use once_cell::sync::Lazy;
use crate::errors::{GeekToolsError, Result};

/// 文件内容缓存条目
#[derive(Clone)]
struct CacheEntry {
    content: String,
    last_modified: SystemTime,
    cached_at: SystemTime,
}

/// 全局文件读取缓存，减少重复I/O
static FILE_CACHE: Lazy<Arc<Mutex<HashMap<PathBuf, CacheEntry>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

const CACHE_TTL: Duration = Duration::from_secs(300); // 5分钟缓存

/// 检查缓存条目是否有效
fn is_cache_valid(entry: &CacheEntry, file_modified: SystemTime) -> bool {
    let now = SystemTime::now();
    entry.last_modified >= file_modified && 
    now.duration_since(entry.cached_at).unwrap_or(Duration::MAX) < CACHE_TTL
}

/// Read file content as UTF-8 string with caching
pub fn read(path: impl AsRef<Path>) -> Result<String> {
    let path_buf = path.as_ref().to_path_buf();
    
    // 首先检查缓存
    if let Ok(cache) = FILE_CACHE.lock() {
        if let (Some(entry), Ok(metadata)) = (cache.get(&path_buf), fs::metadata(&path_buf)) {
            if let Ok(modified) = metadata.modified() {
                if is_cache_valid(entry, modified) {
                    return Ok(entry.content.clone());
                }
            }
        }
    }
    
    // 缓存未命中，读取文件
    let content = fs::read_to_string(&path_buf).map_err(|e| GeekToolsError::FileOperationError {
        path: path_buf.display().to_string(),
        source: e,
    })?;
    
    // 缓存读取结果
    if let (Ok(mut cache), Ok(metadata)) = (FILE_CACHE.lock(), fs::metadata(&path_buf)) {
        if let Ok(modified) = metadata.modified() {
            cache.insert(path_buf, CacheEntry {
                content: content.clone(),
                last_modified: modified,
                cached_at: SystemTime::now(),
            });
        }
    }
    
    Ok(content)
}

/// Write a UTF-8 string to file, creating parent directories if needed
pub fn write(path: impl AsRef<Path>, data: &str) -> Result<()> {
    let path_buf = path.as_ref().to_path_buf();
    
    if let Some(parent) = path_buf.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| GeekToolsError::FileOperationError {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }
    
    let result = fs::write(&path_buf, data).map_err(|e| GeekToolsError::FileOperationError {
        path: path_buf.display().to_string(),
        source: e,
    });
    
    // 写入成功后，使缓存失效
    if result.is_ok() {
        if let Ok(mut cache) = FILE_CACHE.lock() {
            cache.remove(&path_buf);
        }
    }
    
    result
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

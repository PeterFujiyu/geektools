use log::{Level, Record};
use chrono::{DateTime, Local};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::fs::{File, Metadata};
use std::sync::{Arc, Mutex};
use flate2::write::GzEncoder;
use flate2::Compression;
use crate::errors::{GeekToolsError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl From<Level> for LogLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => LogLevel::Error,
            Level::Warn => LogLevel::Warn,
            Level::Info => LogLevel::Info,
            Level::Debug => LogLevel::Debug,
            Level::Trace => LogLevel::Trace,
        }
    }
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Level::Error,
            LogLevel::Warn => Level::Warn,
            LogLevel::Info => Level::Info,
            LogLevel::Debug => Level::Debug,
            LogLevel::Trace => Level::Trace,
        }
    }
}

/// 结构化日志条目
#[derive(Debug)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub level: LogLevel,
    pub module: String,
    pub message: String,
    pub metadata: Option<Value>,
}

impl LogEntry {
    pub fn to_json(&self) -> Value {
        json!({
            "timestamp": self.timestamp.to_rfc3339(),
            "level": match self.level {
                LogLevel::Error => "ERROR",
                LogLevel::Warn => "WARN",
                LogLevel::Info => "INFO",
                LogLevel::Debug => "DEBUG",
                LogLevel::Trace => "TRACE",
            },
            "module": self.module,
            "message": self.message,
            "metadata": self.metadata
        })
    }
    
    pub fn to_formatted_string(&self) -> String {
        format!(
            "[{}] {} [{}] {}{}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            match self.level {
                LogLevel::Error => "ERROR",
                LogLevel::Warn => "WARN ",
                LogLevel::Info => "INFO ",
                LogLevel::Debug => "DEBUG",
                LogLevel::Trace => "TRACE",
            },
            self.module,
            self.message,
            self.metadata.as_ref()
                .map(|m| format!(" {}", m))
                .unwrap_or_default()
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    pub max_file_size: u64,     // 最大文件大小 (bytes)
    pub max_files: usize,       // 最大保留文件数
    pub compress_old_logs: bool, // 是否压缩旧日志
    pub cleanup_days: u64,      // 自动清理天数
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_files: 10,
            compress_old_logs: true,
            cleanup_days: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,           // "ERROR", "WARN", "INFO", "DEBUG", "TRACE"
    pub file_enabled: bool,      // 是否启用文件日志
    pub console_enabled: bool,   // 是否启用控制台日志
    pub rotation: LogRotationConfig,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "INFO".to_string(),
            file_enabled: true,
            console_enabled: true,
            rotation: LogRotationConfig::default(),
        }
    }
}

pub struct RotatingLogger {
    base_path: PathBuf,
    current_file: Arc<Mutex<Option<File>>>,
    config: LogRotationConfig,
    current_size: Arc<Mutex<u64>>,
}

impl RotatingLogger {
    pub fn new(base_path: PathBuf, config: LogRotationConfig) -> Result<Self> {
        let current_file = Self::create_log_file(&base_path)?;
        let file_size = current_file.metadata()
            .map(|m| m.len())
            .unwrap_or(0);
        
        Ok(Self {
            base_path,
            current_file: Arc::new(Mutex::new(Some(current_file))),
            config,
            current_size: Arc::new(Mutex::new(file_size)),
        })
    }
    
    fn create_log_file(base_path: &Path) -> Result<File> {
        if let Some(parent) = base_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| GeekToolsError::FileOperationError {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
        File::create(base_path).map_err(|e| GeekToolsError::FileOperationError {
            path: base_path.display().to_string(),
            source: e,
        })
    }
    
    pub fn write(&self, entry: &LogEntry) -> Result<()> {
        let formatted = entry.to_formatted_string();
        let bytes = formatted.as_bytes();
        
        {
            let size = self.current_size.lock().unwrap();
            if *size + bytes.len() as u64 > self.config.max_file_size {
                drop(size);
                self.rotate()?;
            }
        }
        
        {
            let mut file_guard = self.current_file.lock().unwrap();
            if let Some(ref mut file) = file_guard.as_mut() {
                file.write_all(bytes).map_err(|e| GeekToolsError::FileOperationError {
                    path: self.base_path.display().to_string(),
                    source: e,
                })?;
                file.write_all(b"\n").map_err(|e| GeekToolsError::FileOperationError {
                    path: self.base_path.display().to_string(),
                    source: e,
                })?;
                file.flush().map_err(|e| GeekToolsError::FileOperationError {
                    path: self.base_path.display().to_string(),
                    source: e,
                })?;
            }
        }
        
        {
            let mut size = self.current_size.lock().unwrap();
            *size += bytes.len() as u64 + 1;
        }
        
        Ok(())
    }
    
    fn rotate(&self) -> Result<()> {
        // 关闭当前文件
        {
            let mut file_guard = self.current_file.lock().unwrap();
            *file_guard = None;
        }
        
        // 重命名文件
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let rotated_path = self.base_path.with_extension(format!("log.{}", timestamp));
        std::fs::rename(&self.base_path, &rotated_path).map_err(|e| GeekToolsError::FileOperationError {
            path: self.base_path.display().to_string(),
            source: e,
        })?;
        
        // 压缩旧文件（如果启用）
        if self.config.compress_old_logs {
            self.compress_file(&rotated_path)?;
        }
        
        // 创建新文件
        let new_file = Self::create_log_file(&self.base_path)?;
        {
            let mut file_guard = self.current_file.lock().unwrap();
            *file_guard = Some(new_file);
        }
        *self.current_size.lock().unwrap() = 0;
        
        // 清理旧文件
        self.cleanup_old_logs()?;
        
        Ok(())
    }
    
    fn compress_file(&self, path: &Path) -> Result<()> {
        let input = std::fs::read(path).map_err(|e| GeekToolsError::FileOperationError {
            path: path.display().to_string(),
            source: e,
        })?;
        let compressed_path = path.with_extension("log.gz");
        
        let file = File::create(&compressed_path).map_err(|e| GeekToolsError::FileOperationError {
            path: compressed_path.display().to_string(),
            source: e,
        })?;
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(&input).map_err(|e| GeekToolsError::FileOperationError {
            path: compressed_path.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::Other, e),
        })?;
        encoder.finish().map_err(|e| GeekToolsError::FileOperationError {
            path: compressed_path.display().to_string(),
            source: e,
        })?;
        
        // 删除原文件
        std::fs::remove_file(path).map_err(|e| GeekToolsError::FileOperationError {
            path: path.display().to_string(),
            source: e,
        })?;
        
        Ok(())
    }
    
    fn cleanup_old_logs(&self) -> Result<()> {
        if let Some(parent_dir) = self.base_path.parent() {
            let entries = std::fs::read_dir(parent_dir).map_err(|e| GeekToolsError::FileOperationError {
                path: parent_dir.display().to_string(),
                source: e,
            })?;
            
            let mut log_files = Vec::new();
            let base_name = self.base_path.file_name().unwrap().to_string_lossy();
            
            for entry in entries {
                let entry = entry.map_err(|e| GeekToolsError::FileOperationError {
                    path: parent_dir.display().to_string(),
                    source: e,
                })?;
                
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy();
                    if file_name_str.starts_with(&format!("{}.log.", base_name)) {
                        if let Ok(metadata) = entry.metadata() {
                            log_files.push((path, metadata.modified().unwrap_or(std::time::UNIX_EPOCH)));
                        }
                    }
                }
            }
            
            // 按修改时间排序，最新的在前
            log_files.sort_by(|a, b| b.1.cmp(&a.1));
            
            // 删除超过最大数量的文件
            if log_files.len() > self.config.max_files {
                for (path, _) in log_files.iter().skip(self.config.max_files) {
                    let _ = std::fs::remove_file(path);
                }
            }
            
            // 删除超过保留天数的文件
            let cutoff_time = std::time::SystemTime::now()
                .checked_sub(std::time::Duration::from_secs(self.config.cleanup_days * 24 * 3600))
                .unwrap_or(std::time::UNIX_EPOCH);
            
            for (path, modified_time) in &log_files {
                if *modified_time < cutoff_time {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        
        Ok(())
    }
}

// 保留兼容性的同时添加新的日志功能
#[macro_export]
macro_rules! log_with_metadata {
    ($level:ident, $msg:expr) => {
        log::$level!("{}", $msg);
    };
    ($level:ident, $msg:expr, $($key:expr => $value:expr),+) => {
        log::$level!("{} [{}]", $msg, 
            vec![$(format!("{}={}", $key, $value)),+].join(", ")
        );
    };
}

pub fn init_logging(config: &LoggingConfig, log_file_path: Option<PathBuf>) -> Result<()> {
    let log_level = match config.level.as_str() {
        "ERROR" => log::LevelFilter::Error,
        "WARN" => log::LevelFilter::Warn,
        "INFO" => log::LevelFilter::Info,
        "DEBUG" => log::LevelFilter::Debug,
        "TRACE" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    };
    
    // 初始化 env_logger 用于控制台输出
    if config.console_enabled {
        env_logger::Builder::from_default_env()
            .filter_level(log_level)
            .init();
    }
    
    // 如果启用文件日志且提供了路径，初始化文件日志
    if config.file_enabled {
        if let Some(path) = log_file_path {
            let _rotating_logger = RotatingLogger::new(path, config.rotation.clone())?;
            // 注意：这里需要实现一个自定义的 Log trait 实现来同时写入文件和控制台
            // 目前先保持简单实现
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_log_entry_formatting() {
        let entry = LogEntry {
            timestamp: Local::now(),
            level: LogLevel::Info,
            module: "test".to_string(),
            message: "Test message".to_string(),
            metadata: Some(json!({"key": "value"})),
        };
        
        let formatted = entry.to_formatted_string();
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("test"));
        assert!(formatted.contains("Test message"));
    }
    
    #[test]
    fn test_log_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let config = LogRotationConfig {
            max_file_size: 100, // 很小的大小以触发轮转
            max_files: 3,
            compress_old_logs: false,
            cleanup_days: 1,
        };
        
        let logger = RotatingLogger::new(log_path.clone(), config).unwrap();
        
        // 写入超过限制的数据
        for i in 0..10 {
            let entry = LogEntry {
                timestamp: Local::now(),
                level: LogLevel::Info,
                module: "test".to_string(),
                message: format!("Test message {}", i),
                metadata: None,
            };
            logger.write(&entry).unwrap();
        }
        
        // 验证文件轮转是否发生
        let parent_dir = log_path.parent().unwrap();
        let entries: Vec<_> = std::fs::read_dir(parent_dir).unwrap().collect();
        assert!(entries.len() > 1);
    }
}
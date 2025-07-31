# GeekTools 核心稳定性改良技术文档

## 文档概述

本文档详细规划了 GeekTools 项目的核心稳定性改良方案，包括错误处理机制完善、日志系统优化和配置管理增强三个核心方面。这些改进将显著提升应用程序的可靠性、可维护性和用户体验。

---

## 1. 错误处理机制完善

### 1.1 当前状态分析

**现有问题**：
- 缺乏统一的错误类型定义，不同模块使用不同的错误处理方式
- 错误信息不够详细，缺乏用户友好的错误提示
- 缺少错误恢复机制，遇到错误通常直接退出

**当前错误处理模式**：
```rust
// 当前使用 std::io::Result 和简单的 unwrap_or_else
let file = fileio::open_append(&*LOG_FILE_PATH).unwrap_or_else(|e| {
    eprintln!("Failed to open log file: {e}");
    File::create("/dev/null").unwrap()
});
```

### 1.2 技术实施方案

#### 1.2.1 统一错误类型定义

**新增依赖**：
```toml
[dependencies]
thiserror = "1.0"
anyhow = "1.0"
```

**错误类型定义** (`src/errors.rs`)：
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeekToolsError {
    // 文件操作错误
    #[error("File operation failed: {path}")]
    FileOperationError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    
    // 网络请求错误
    #[error("Network request failed: {url}")]
    NetworkError {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    
    // 配置文件错误
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
    
    // 脚本执行错误
    #[error("Script execution failed: {script_name}")]
    ScriptExecutionError {
        script_name: String,
        exit_code: Option<i32>,
        #[source]
        source: std::io::Error,
    },
    
    // 插件系统错误
    #[error("Plugin error: {plugin_name} - {message}")]
    PluginError {
        plugin_name: String,
        message: String,
    },
    
    // 语言/国际化错误
    #[error("Localization error: {key}")]
    LocalizationError { key: String },
    
    // 权限错误
    #[error("Permission denied: {operation}")]
    PermissionError { operation: String },
    
    // 验证错误
    #[error("Validation failed: {field} - {message}")]
    ValidationError {
        field: String,
        message: String,
    },
}

pub type Result<T> = std::result::Result<T, GeekToolsError>;
```

#### 1.2.2 错误处理工具函数

**错误转换和增强** (`src/errors.rs`)：
```rust
impl GeekToolsError {
    /// 获取用户友好的错误信息
    pub fn user_friendly_message(&self, lang: Language) -> String {
        match self {
            Self::FileOperationError { path, .. } => {
                t("error.file_operation", &[("path", path)], lang)
            }
            Self::NetworkError { url, .. } => {
                t("error.network", &[("url", url)], lang)
            }
            Self::ConfigError { message } => {
                t("error.config", &[("message", message)], lang)
            }
            Self::ScriptExecutionError { script_name, exit_code, .. } => {
                let code = exit_code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
                t("error.script_execution", &[("script", script_name), ("code", &code)], lang)
            }
            // ... 其他错误类型
        }
    }
    
    /// 获取恢复建议
    pub fn recovery_suggestions(&self, lang: Language) -> Vec<String> {
        match self {
            Self::FileOperationError { path, source } => {
                match source.kind() {
                    std::io::ErrorKind::PermissionDenied => vec![
                        t("recovery.check_permissions", &[("path", path)], lang),
                        t("recovery.run_as_admin", &[], lang),
                    ],
                    std::io::ErrorKind::NotFound => vec![
                        t("recovery.create_directory", &[("path", path)], lang),
                        t("recovery.check_path", &[("path", path)], lang),
                    ],
                    _ => vec![t("recovery.retry_operation", &[], lang)],
                }
            }
            Self::NetworkError { .. } => vec![
                t("recovery.check_connection", &[], lang),
                t("recovery.check_proxy", &[], lang),
                t("recovery.retry_later", &[], lang),
            ],
            // ... 其他恢复建议
        }
    }
    
    /// 是否可以自动恢复
    pub fn is_recoverable(&self) -> bool {
        matches!(self, 
            Self::NetworkError { .. } |
            Self::FileOperationError { .. } |
            Self::ConfigError { .. }
        )
    }
}
```

#### 1.2.3 Graceful Error Recovery 机制

**重试策略** (`src/recovery.rs`)：
```rust
use std::time::Duration;
use std::thread;

pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_factor: 2.0,
        }
    }
}

/// 带重试的操作执行器
pub fn retry_with_backoff<T, F>(
    operation: F,
    config: &RetryConfig,
) -> Result<T>
where
    F: Fn() -> Result<T>,
{
    let mut delay = config.initial_delay;
    
    for attempt in 1..=config.max_attempts {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) if attempt == config.max_attempts => return Err(e),
            Err(e) if !e.is_recoverable() => return Err(e),
            Err(_) => {
                log_println!("Attempt {} failed, retrying in {:?}", attempt, delay);
                thread::sleep(delay);
                delay = std::cmp::min(
                    Duration::from_millis((delay.as_millis() as f64 * config.backoff_factor) as u64),
                    config.max_delay,
                );
            }
        }
    }
    
    unreachable!()
}
```

**自动恢复策略**：
```rust
/// 自动恢复处理器
pub struct RecoveryHandler {
    config: RetryConfig,
    user_lang: Language,
}

impl RecoveryHandler {
    pub fn new(config: RetryConfig, user_lang: Language) -> Self {
        Self { config, user_lang }
    }
    
    /// 处理错误并尝试恢复
    pub fn handle_error(&self, error: &GeekToolsError) -> RecoveryAction {
        match error {
            GeekToolsError::FileOperationError { path, source } => {
                match source.kind() {
                    std::io::ErrorKind::NotFound => {
                        // 尝试创建缺失的目录
                        if let Some(parent) = Path::new(path).parent() {
                            if fileio::create_dir(parent).is_ok() {
                                return RecoveryAction::Retry;
                            }
                        }
                        RecoveryAction::ShowSuggestions(error.recovery_suggestions(self.user_lang))
                    }
                    _ => RecoveryAction::ShowSuggestions(error.recovery_suggestions(self.user_lang))
                }
            }
            GeekToolsError::NetworkError { .. } => {
                RecoveryAction::RetryWithBackoff(self.config.clone())
            }
            _ => RecoveryAction::ShowSuggestions(error.recovery_suggestions(self.user_lang))
        }
    }
}

#[derive(Debug)]
pub enum RecoveryAction {
    Retry,
    RetryWithBackoff(RetryConfig),
    ShowSuggestions(Vec<String>),
    Exit,
}
```

### 1.3 集成到现有代码

**模块级错误处理** (`src/fileio.rs` 改进)：
```rust
use crate::errors::{GeekToolsError, Result};

pub fn read(path: impl AsRef<Path>) -> Result<String> {
    fs::read_to_string(&path).map_err(|e| GeekToolsError::FileOperationError {
        path: path.as_ref().display().to_string(),
        source: e,
    })
}

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
```

---

## 2. 日志系统优化

### 2.1 当前状态分析

**现有问题**：
- 日志文件可能无限增长，缺乏轮转机制
- 日志级别控制不足，无法按需调整详细程度
- 缺乏日志压缩和自动清理机制
- 日志格式不统一，缺少结构化信息

### 2.2 技术实施方案

#### 2.2.1 日志级别和结构化日志

**新增依赖**：
```toml
[dependencies]
log = "0.4"
env_logger = "0.10"
serde_json = "1.0"  # 已存在
chrono = { version = "0.4", features = ["clock"] }  # 已存在
```

**日志级别定义** (`src/logging.rs`)：
```rust
use log::{Level, Record};
use chrono::{DateTime, Local};
use serde_json::{json, Value};
use std::io::Write;

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
```

#### 2.2.2 日志轮转机制

**日志文件管理** (`src/logging.rs`)：
```rust
use std::path::{Path, PathBuf};
use std::fs::{File, Metadata};
use std::sync::{Arc, Mutex};
use flate2::write::GzEncoder;
use flate2::Compression;

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

pub struct RotatingLogger {
    base_path: PathBuf,
    current_file: Arc<Mutex<File>>,
    config: LogRotationConfig,
    current_size: Arc<Mutex<u64>>,
}

impl RotatingLogger {
    pub fn new(base_path: PathBuf, config: LogRotationConfig) -> std::io::Result<Self> {
        let current_file = Self::create_log_file(&base_path)?;
        let file_size = current_file.metadata()?.len();
        
        Ok(Self {
            base_path,
            current_file: Arc::new(Mutex::new(current_file)),
            config,
            current_size: Arc::new(Mutex::new(file_size)),
        })
    }
    
    fn create_log_file(base_path: &Path) -> std::io::Result<File> {
        if let Some(parent) = base_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        File::create(base_path)
    }
    
    pub fn write(&self, entry: &LogEntry) -> std::io::Result<()> {
        let formatted = entry.to_formatted_string();
        let bytes = formatted.as_bytes();
        
        {
            let mut size = self.current_size.lock().unwrap();
            if *size + bytes.len() as u64 > self.config.max_file_size {
                drop(size);
                self.rotate()?;
            }
        }
        
        {
            let mut file = self.current_file.lock().unwrap();
            file.write_all(bytes)?;
            file.write_all(b"\n")?;
            file.flush()?;
        }
        
        {
            let mut size = self.current_size.lock().unwrap();
            *size += bytes.len() as u64 + 1;
        }
        
        Ok(())
    }
    
    fn rotate(&self) -> std::io::Result<()> {
        // 关闭当前文件
        drop(self.current_file.lock().unwrap());
        
        // 重命名文件
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let rotated_path = self.base_path.with_extension(format!("log.{}", timestamp));
        std::fs::rename(&self.base_path, &rotated_path)?;
        
        // 压缩旧文件（如果启用）
        if self.config.compress_old_logs {
            self.compress_file(&rotated_path)?;
        }
        
        // 创建新文件
        let new_file = Self::create_log_file(&self.base_path)?;
        *self.current_file.lock().unwrap() = new_file;
        *self.current_size.lock().unwrap() = 0;
        
        // 清理旧文件
        self.cleanup_old_logs()?;
        
        Ok(())
    }
    
    fn compress_file(&self, path: &Path) -> std::io::Result<()> {
        let input = std::fs::read(path)?;
        let compressed_path = path.with_extension("log.gz");
        
        let file = File::create(&compressed_path)?;
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(&input)?;
        encoder.finish()?;
        
        // 删除原文件
        std::fs::remove_file(path)?;
        
        Ok(())
    }
    
    fn cleanup_old_logs(&self) -> std::io::Result<()> {
        // 实现基于文件数量和时间的清理逻辑
        // ... 省略具体实现
        Ok(())
    }
}
```

#### 2.2.3 配置式日志级别控制

**配置集成** (`src/main.rs` 中的配置结构扩展)：
```rust
#[derive(Deserialize, Clone)]
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

#[derive(Deserialize, Clone)]
pub struct Config {
    // ... 现有字段
    pub logging: LoggingConfig,
}
```

### 2.3 统一日志宏

**替换现有的 log_println! 宏**：
```rust
use log::{error, warn, info, debug, trace};

// 保留兼容性的同时添加新的日志功能
macro_rules! log_with_metadata {
    ($level:ident, $msg:expr) => {
        $level!("{}", $msg);
    };
    ($level:ident, $msg:expr, $($key:expr => $value:expr),+) => {
        $level!("{} [{}]", $msg, 
            vec![$(format!("{}={}", $key, $value)),+].join(", ")
        );
    };
}

// 使用示例
// log_with_metadata!(info, "Script executed successfully", "script" => script_name, "duration" => duration);
```

---

## 3. 配置管理增强

### 3.1 当前状态分析

**现有问题**：
- 缺乏配置文件版本兼容性检查
- 没有配置备份和恢复机制
- 配置项验证不充分
- 缺少默认值回退策略

### 3.2 技术实施方案

#### 3.2.1 版本化配置结构

**配置版本管理** (`src/config.rs`)：
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CURRENT_CONFIG_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigFile {
    pub version: u32,
    pub config: Config,
    pub metadata: ConfigMetadata,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigMetadata {
    pub created_at: String,
    pub last_modified: String,
    pub created_by_version: String,
    pub last_modified_by_version: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub language: String,
    pub custom_scripts: Vec<CustomScript>,
    pub plugins: PluginConfig,
    pub logging: LoggingConfig,
    pub security: SecurityConfig,
    pub ui: UiConfig,
}

/// 配置迁移器
pub struct ConfigMigrator;

impl ConfigMigrator {
    pub fn migrate(config_file: ConfigFile) -> Result<ConfigFile> {
        match config_file.version {
            1 => Self::migrate_v1_to_v2(config_file),
            CURRENT_CONFIG_VERSION => Ok(config_file),
            v if v > CURRENT_CONFIG_VERSION => {
                Err(GeekToolsError::ConfigError {
                    message: format!("Configuration version {} is newer than supported version {}", 
                                   v, CURRENT_CONFIG_VERSION)
                })
            }
            v => {
                Err(GeekToolsError::ConfigError {
                    message: format!("Unknown configuration version: {}", v)
                })
            }
        }
    }
    
    fn migrate_v1_to_v2(mut config: ConfigFile) -> Result<ConfigFile> {
        // V1 到 V2 的迁移逻辑
        // 例如：添加新的默认字段，转换旧格式等
        
        config.version = 2;
        config.metadata.last_modified = chrono::Local::now().to_rfc3339();
        config.metadata.last_modified_by_version = env!("CARGO_PKG_VERSION").to_string();
        
        // 添加新字段的默认值
        if config.config.logging.level.is_empty() {
            config.config.logging.level = "INFO".to_string();
        }
        
        Ok(config)
    }
}
```

#### 3.2.2 配置验证系统

**配置验证器** (`src/config.rs`)：
```rust
use std::net::Url;
use std::path::Path;

pub trait Validator<T> {
    fn validate(&self, value: &T) -> Result<()>;
}

pub struct ConfigValidator;

impl ConfigValidator {
    pub fn validate_config(config: &Config) -> Result<()> {
        Self::validate_language(&config.language)?;
        Self::validate_custom_scripts(&config.custom_scripts)?;
        Self::validate_logging_config(&config.logging)?;
        Self::validate_security_config(&config.security)?;
        Ok(())
    }
    
    fn validate_language(language: &str) -> Result<()> {
        match language {
            "en" | "zh" => Ok(()),
            _ => Err(GeekToolsError::ValidationError {
                field: "language".to_string(),
                message: format!("Unsupported language: {}. Supported: en, zh", language),
            }),
        }
    }
    
    fn validate_custom_scripts(scripts: &[CustomScript]) -> Result<()> {
        for (index, script) in scripts.iter().enumerate() {
            if script.name.trim().is_empty() {
                return Err(GeekToolsError::ValidationError {
                    field: format!("custom_scripts[{}].name", index),
                    message: "Script name cannot be empty".to_string(),
                });
            }
            
            if let Some(url) = &script.url {
                Url::parse(url).map_err(|_| GeekToolsError::ValidationError {
                    field: format!("custom_scripts[{}].url", index),
                    message: format!("Invalid URL: {}", url),
                })?;
            }
            
            if let Some(path) = &script.file_path {
                if !Path::new(path).exists() {
                    return Err(GeekToolsError::ValidationError {
                        field: format!("custom_scripts[{}].file_path", index),
                        message: format!("File does not exist: {}", path),
                    });
                }
            }
        }
        Ok(())
    }
    
    fn validate_logging_config(logging: &LoggingConfig) -> Result<()> {
        match logging.level.as_str() {
            "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE" => Ok(()),
            _ => Err(GeekToolsError::ValidationError {
                field: "logging.level".to_string(),
                message: format!("Invalid log level: {}. Valid levels: ERROR, WARN, INFO, DEBUG, TRACE", 
                               logging.level),
            }),
        }
    }
    
    fn validate_security_config(security: &SecurityConfig) -> Result<()> {
        if security.max_script_execution_time_seconds == 0 {
            return Err(GeekToolsError::ValidationError {
                field: "security.max_script_execution_time_seconds".to_string(),
                message: "Script execution timeout must be greater than 0".to_string(),
            });
        }
        
        if security.allowed_domains.is_empty() && security.block_all_network {
            return Err(GeekToolsError::ValidationError {
                field: "security".to_string(),
                message: "Cannot block all network access without specifying allowed domains".to_string(),
            });
        }
        
        Ok(())
    }
}
```

#### 3.2.3 配置备份和恢复

**备份管理器** (`src/config.rs`)：
```rust
use std::fs;
use chrono::Local;

pub struct ConfigBackupManager {
    backup_dir: PathBuf,
    max_backups: usize,
}

impl ConfigBackupManager {
    pub fn new(backup_dir: PathBuf, max_backups: usize) -> Self {
        Self { backup_dir, max_backups }
    }
    
    /// 创建配置备份
    pub fn create_backup(&self, config_path: &Path) -> Result<PathBuf> {
        if !self.backup_dir.exists() {
            fs::create_dir_all(&self.backup_dir).map_err(|e| GeekToolsError::FileOperationError {
                path: self.backup_dir.display().to_string(),
                source: e,
            })?;
        }
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_filename = format!("config_backup_{}.json", timestamp);
        let backup_path = self.backup_dir.join(backup_filename);
        
        fs::copy(config_path, &backup_path).map_err(|e| GeekToolsError::FileOperationError {
            path: backup_path.display().to_string(),
            source: e,
        })?;
        
        self.cleanup_old_backups()?;
        
        Ok(backup_path)
    }
    
    /// 从备份恢复配置
    pub fn restore_from_backup(&self, backup_path: &Path, target_path: &Path) -> Result<()> {
        if !backup_path.exists() {
            return Err(GeekToolsError::FileOperationError {
                path: backup_path.display().to_string(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "Backup file not found"),
            });
        }
        
        // 验证备份文件
        let backup_content = fs::read_to_string(backup_path).map_err(|e| GeekToolsError::FileOperationError {
            path: backup_path.display().to_string(),
            source: e,
        })?;
        
        let config_file: ConfigFile = serde_json::from_str(&backup_content)
            .map_err(|e| GeekToolsError::ConfigError {
                message: format!("Invalid backup file format: {}", e),
            })?;
        
        // 验证配置
        ConfigValidator::validate_config(&config_file.config)?;
        
        // 创建当前配置的备份
        if target_path.exists() {
            self.create_backup(target_path)?;
        }
        
        // 恢复配置
        fs::copy(backup_path, target_path).map_err(|e| GeekToolsError::FileOperationError {
            path: target_path.display().to_string(),
            source: e,
        })?;
        
        Ok(())
    }
    
    /// 列出所有备份文件
    pub fn list_backups(&self) -> Result<Vec<PathBuf>> {
        if !self.backup_dir.exists() {
            return Ok(vec![]);
        }
        
        let mut backups = vec![];
        let entries = fs::read_dir(&self.backup_dir).map_err(|e| GeekToolsError::FileOperationError {
            path: self.backup_dir.display().to_string(),
            source: e,
        })?;
        
        for entry in entries {
            let entry = entry.map_err(|e| GeekToolsError::FileOperationError {
                path: self.backup_dir.display().to_string(),
                source: e,
            })?;
            
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(filename) = path.file_name() {
                    if filename.to_string_lossy().starts_with("config_backup_") {
                        backups.push(path);
                    }
                }
            }
        }
        
        // 按时间排序（最新的在前）
        backups.sort_by(|a, b| {
            b.metadata().unwrap().modified().unwrap()
                .cmp(&a.metadata().unwrap().modified().unwrap())
        });
        
        Ok(backups)
    }
    
    fn cleanup_old_backups(&self) -> Result<()> {
        let backups = self.list_backups()?;
        
        if backups.len() > self.max_backups {
            for backup in backups.iter().skip(self.max_backups) {
                fs::remove_file(backup).map_err(|e| GeekToolsError::FileOperationError {
                    path: backup.display().to_string(),
                    source: e,
                })?;
            }
        }
        
        Ok(())
    }
}
```

### 3.3 配置管理器重构

**统一配置管理** (`src/config.rs`)：
```rust
pub struct ConfigManager {
    config_path: PathBuf,
    backup_manager: ConfigBackupManager,
    current_config: Arc<RwLock<Config>>,
}

impl ConfigManager {
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let backup_dir = config_path.parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        
        let backup_manager = ConfigBackupManager::new(backup_dir, 5);
        
        let config = Self::load_or_create_config(&config_path)?;
        
        Ok(Self {
            config_path,
            backup_manager,
            current_config: Arc::new(RwLock::new(config)),
        })
    }
    
    fn load_or_create_config(config_path: &Path) -> Result<Config> {
        if config_path.exists() {
            Self::load_config(config_path)
        } else {
            let default_config = Self::create_default_config();
            Self::save_config_file(config_path, &default_config)?;
            Ok(default_config.config)
        }
    }
    
    fn load_config(config_path: &Path) -> Result<Config> {
        let content = fs::read_to_string(config_path).map_err(|e| GeekToolsError::FileOperationError {
            path: config_path.display().to_string(),
            source: e,
        })?;
        
        let config_file: ConfigFile = serde_json::from_str(&content)
            .map_err(|e| GeekToolsError::ConfigError {
                message: format!("Failed to parse config file: {}", e),
            })?;
        
        // 迁移配置版本
        let migrated_config = ConfigMigrator::migrate(config_file)?;
        
        // 验证配置
        ConfigValidator::validate_config(&migrated_config.config)?;
        
        // 如果版本发生变化，保存迁移后的配置
        if migrated_config.version != config_file.version {
            Self::save_config_file(config_path, &migrated_config)?;
        }
        
        Ok(migrated_config.config)
    }
    
    fn create_default_config() -> ConfigFile {
        ConfigFile {
            version: CURRENT_CONFIG_VERSION,
            config: Config::default(),
            metadata: ConfigMetadata {
                created_at: Local::now().to_rfc3339(),
                last_modified: Local::now().to_rfc3339(),
                created_by_version: env!("CARGO_PKG_VERSION").to_string(),
                last_modified_by_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
    
    pub fn get_config(&self) -> Arc<RwLock<Config>> {
        Arc::clone(&self.current_config)
    }
    
    pub fn update_config<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut Config) -> Result<()>,
    {
        // 创建备份
        self.backup_manager.create_backup(&self.config_path)?;
        
        {
            let mut config = self.current_config.write().unwrap();
            updater(&mut config)?;
            
            // 验证更新后的配置
            ConfigValidator::validate_config(&config)?;
        }
        
        // 保存配置
        self.save_current_config()?;
        
        Ok(())
    }
    
    fn save_current_config(&self) -> Result<()> {
        let config = self.current_config.read().unwrap().clone();
        let config_file = ConfigFile {
            version: CURRENT_CONFIG_VERSION,
            config,
            metadata: ConfigMetadata {
                created_at: "".to_string(), // 保留原有创建时间
                last_modified: Local::now().to_rfc3339(),
                created_by_version: "".to_string(), // 保留原有版本
                last_modified_by_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        
        Self::save_config_file(&self.config_path, &config_file)
    }
    
    fn save_config_file(path: &Path, config_file: &ConfigFile) -> Result<()> {
        let content = serde_json::to_string_pretty(config_file)
            .map_err(|e| GeekToolsError::ConfigError {
                message: format!("Failed to serialize config: {}", e),
            })?;
        
        crate::fileio::write(path, &content)
    }
}
```

---

## 4. 实施计划和时间表

### 4.1 第一阶段：错误处理机制 (1-2周)
1. **Week 1**:
   - 添加 `thiserror` 和 `anyhow` 依赖
   - 创建 `src/errors.rs` 模块
   - 定义统一错误类型 `GeekToolsError`
   - 实现错误转换和用户友好消息

2. **Week 2**:
   - 实现重试和恢复机制
   - 更新 `fileio.rs` 使用新错误类型
   - 逐步迁移其他模块的错误处理

### 4.2 第二阶段：日志系统优化 (1-2周)
1. **Week 1**:
   - 添加 `log` 和 `env_logger` 依赖
   - 创建 `src/logging.rs` 模块
   - 实现结构化日志和轮转机制

2. **Week 2**:
   - 集成配置式日志级别控制
   - 替换现有日志宏
   - 实现日志压缩和清理

### 4.3 第三阶段：配置管理增强 (2-3周)
1. **Week 1-2**:
   - 创建 `src/config.rs` 模块
   - 实现版本化配置和迁移机制
   - 添加配置验证系统

2. **Week 3**:
   - 实现配置备份和恢复功能
   - 重构现有配置管理逻辑
   - 添加配置管理CLI命令

### 4.4 第四阶段：集成测试和文档 (1周)
1. 编写单元测试和集成测试
2. 更新用户文档和开发者文档
3. 性能测试和优化

---

## 5. 测试策略

### 5.1 单元测试
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_error_recovery_suggestions() {
        let error = GeekToolsError::FileOperationError {
            path: "/nonexistent/path".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
        };
        
        let suggestions = error.recovery_suggestions(Language::English);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("create")));
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
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        config.language = "invalid".to_string();
        
        let result = ConfigValidator::validate_config(&config);
        assert!(result.is_err());
        
        if let Err(GeekToolsError::ValidationError { field, message }) = result {
            assert_eq!(field, "language");
            assert!(message.contains("Unsupported language"));
        }
    }
    
    #[test]
    fn test_config_backup_restore() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let backup_dir = temp_dir.path().join("backups");
        
        let backup_manager = ConfigBackupManager::new(backup_dir, 5);
        
        // 创建测试配置文件
        let config = ConfigFile {
            version: CURRENT_CONFIG_VERSION,
            config: Config::default(),
            metadata: ConfigMetadata {
                created_at: Local::now().to_rfc3339(),
                last_modified: Local::now().to_rfc3339(),
                created_by_version: "test".to_string(),
                last_modified_by_version: "test".to_string(),
            },
        };
        
        let content = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(&config_path, content).unwrap();
        
        // 创建备份
        let backup_path = backup_manager.create_backup(&config_path).unwrap();
        assert!(backup_path.exists());
        
        // 修改原配置
        std::fs::write(&config_path, "modified").unwrap();
        
        // 恢复备份
        backup_manager.restore_from_backup(&backup_path, &config_path).unwrap();
        
        // 验证恢复
        let restored_content = std::fs::read_to_string(&config_path).unwrap();
        let restored_config: ConfigFile = serde_json::from_str(&restored_content).unwrap();
        assert_eq!(restored_config.version, CURRENT_CONFIG_VERSION);
    }
}
```

### 5.2 集成测试
- 端到端错误处理流程测试
- 日志轮转在真实负载下的测试
- 配置迁移的兼容性测试
- 并发访问配置文件的安全性测试

---

## 6. 性能考虑

### 6.1 内存优化
- 使用 `Arc<RwLock<T>>` 避免大量克隆
- 日志异步写入避免阻塞主线程
- 配置延迟序列化减少内存占用

### 6.2 I/O优化
- 批量日志写入减少系统调用
- 配置文件增量更新
- 备份文件压缩节省磁盘空间

### 6.3 并发安全
- 使用读写锁优化配置读取性能
- 原子操作保证日志写入的线程安全
- 避免死锁的锁获取顺序

---

## 7. 监控和指标

### 7.1 错误监控
- 错误发生频率统计
- 错误恢复成功率
- 用户体验错误影响分析

### 7.2 性能监控
- 日志写入性能
- 配置加载时间
- 内存使用趋势

### 7.3 运维指标
- 日志文件大小和轮转频率
- 配置备份成功率
- 系统资源使用情况

---

这个技术文档提供了 GeekTools 核心稳定性改良的完整实施方案，包括详细的代码示例、测试策略和实施计划。通过这些改进，应用程序将具备更强的错误处理能力、更完善的日志系统和更可靠的配置管理机制。
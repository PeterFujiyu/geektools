use serde::{Deserialize, Deserializer, Serialize};
use serde_json;
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::{Arc, RwLock};
use chrono::Local;
use url::Url;
use crate::errors::{GeekToolsError, Result};
use crate::logging::LoggingConfig;
use crate::plugins::MarketplaceConfig;

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
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default, deserialize_with = "deserialize_custom_scripts")]
    pub custom_scripts: Vec<CustomScript>,
    #[serde(default)]
    pub plugins: PluginConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub marketplace_config: MarketplaceConfig,
}

fn default_language() -> String {
    "en".to_string()
}

fn deserialize_custom_scripts<'de, D>(deserializer: D) -> std::result::Result<Vec<CustomScript>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde_json::Value;
    
    let value = Value::deserialize(deserializer)?;
    
    match value {
        Value::Array(arr) => {
            // New format: array of scripts
            Vec::<CustomScript>::deserialize(Value::Array(arr)).map_err(serde::de::Error::custom)
        }
        Value::Object(_) => {
            // Legacy format: empty object, return empty array
            Ok(Vec::new())
        }
        _ => {
            // Default to empty array for any other format
            Ok(Vec::new())
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CustomScript {
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub file_path: Option<String>,
    pub enabled: bool,
    pub last_updated: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginConfig {
    pub enabled: bool,
    pub auto_update: bool,
    pub allowed_plugins: Vec<String>,
    pub plugin_directory: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SecurityConfig {
    pub max_script_execution_time_seconds: u64,
    pub allow_network_access: bool,
    pub allowed_domains: Vec<String>,
    pub block_all_network: bool,
    pub require_confirmation_for_custom_scripts: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UiConfig {
    pub theme: String,
    pub show_timestamps: bool,
    pub max_output_lines: usize,
    pub auto_clear_output: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            custom_scripts: Vec::new(),
            plugins: PluginConfig::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
            ui: UiConfig::default(),
            marketplace_config: MarketplaceConfig::default(),
        }
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_update: false,
            allowed_plugins: Vec::new(),
            plugin_directory: None,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_script_execution_time_seconds: 300, // 5 minutes
            allow_network_access: true,
            allowed_domains: Vec::new(),
            block_all_network: false,
            require_confirmation_for_custom_scripts: true,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            show_timestamps: true,
            max_output_lines: 1000,
            auto_clear_output: false,
        }
    }
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
        config.version = 2;
        config.metadata.last_modified = Local::now().to_rfc3339();
        config.metadata.last_modified_by_version = env!("CARGO_PKG_VERSION").to_string();
        
        // 添加新字段的默认值
        if config.config.logging.level.is_empty() {
            config.config.logging.level = "INFO".to_string();
        }
        
        // 确保新的配置字段存在
        if config.config.security.max_script_execution_time_seconds == 0 {
            config.config.security.max_script_execution_time_seconds = 300;
        }
        
        Ok(config)
    }
}

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
            "en" | "English" | "zh" | "Chinese" => Ok(()),
            _ => Err(GeekToolsError::ValidationError {
                field: "language".to_string(),
                message: format!("Unsupported language: {}. Supported: en, English, zh, Chinese", language),
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
            let a_modified = a.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            let b_modified = b.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            b_modified.cmp(&a_modified)
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
        
        // Try to parse as new ConfigFile format first
        let config_file = match serde_json::from_str::<ConfigFile>(&content) {
            Ok(config_file) => config_file,
            Err(_) => {
                // If that fails, try to parse as legacy Config format and wrap it
                match serde_json::from_str::<Config>(&content) {
                    Ok(legacy_config) => {
                        // Wrap legacy config in ConfigFile structure
                        ConfigFile {
                            version: 1, // Assume version 1 for legacy configs
                            config: legacy_config,
                            metadata: ConfigMetadata {
                                created_at: Local::now().to_rfc3339(),
                                last_modified: Local::now().to_rfc3339(),
                                created_by_version: "legacy".to_string(),
                                last_modified_by_version: env!("CARGO_PKG_VERSION").to_string(),
                            },
                        }
                    }
                    Err(e) => {
                        return Err(GeekToolsError::ConfigError {
                            message: format!("Failed to parse config file: {}", e),
                        });
                    }
                }
            }
        };
        
        // 迁移配置版本
        let migrated_config = ConfigMigrator::migrate(config_file.clone())?;
        
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
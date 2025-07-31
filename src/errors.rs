use thiserror::Error;
use crate::i18n::{Language, t};

#[derive(Error, Debug)]
pub enum GeekToolsError {
    #[error("File operation failed: {path}")]
    FileOperationError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    
    #[error("Network request failed: {url}")]
    NetworkError {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
    
    #[error("Script execution failed: {script_name}")]
    ScriptExecutionError {
        script_name: String,
        exit_code: Option<i32>,
        #[source]
        source: std::io::Error,
    },
    
    #[error("Plugin error: {plugin_name} - {message}")]
    PluginError {
        plugin_name: String,
        message: String,
    },
    
    #[error("Localization error: {key}")]
    LocalizationError { key: String },
    
    #[error("Permission denied: {operation}")]
    PermissionError { operation: String },
    
    #[error("Validation failed: {field} - {message}")]
    ValidationError {
        field: String,
        message: String,
    },
}

pub type Result<T> = std::result::Result<T, GeekToolsError>;

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
            Self::PluginError { plugin_name, message } => {
                t("error.plugin", &[("plugin", plugin_name), ("message", message)], lang)
            }
            Self::LocalizationError { key } => {
                t("error.localization", &[("key", key)], lang)
            }
            Self::PermissionError { operation } => {
                t("error.permission", &[("operation", operation)], lang)
            }
            Self::ValidationError { field, message } => {
                t("error.validation", &[("field", field), ("message", message)], lang)
            }
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
            Self::ConfigError { .. } => vec![
                t("recovery.check_config_syntax", &[], lang),
                t("recovery.restore_backup", &[], lang),
            ],
            Self::ScriptExecutionError { .. } => vec![
                t("recovery.check_script_permissions", &[], lang),
                t("recovery.check_dependencies", &[], lang),
            ],
            Self::PluginError { .. } => vec![
                t("recovery.reinstall_plugin", &[], lang),
                t("recovery.check_plugin_compatibility", &[], lang),
            ],
            Self::LocalizationError { .. } => vec![
                t("recovery.check_language_files", &[], lang),
                t("recovery.reset_language", &[], lang),
            ],
            Self::PermissionError { .. } => vec![
                t("recovery.run_as_admin", &[], lang),
                t("recovery.check_file_permissions", &[], lang),
            ],
            Self::ValidationError { .. } => vec![
                t("recovery.check_input_format", &[], lang),
                t("recovery.refer_to_documentation", &[], lang),
            ],
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

// Implement From traits for common error types
impl From<std::io::Error> for GeekToolsError {
    fn from(error: std::io::Error) -> Self {
        Self::FileOperationError {
            path: "unknown".to_string(),
            source: error,
        }
    }
}

impl From<reqwest::Error> for GeekToolsError {
    fn from(error: reqwest::Error) -> Self {
        Self::NetworkError {
            url: error.url().map(|u| u.to_string()).unwrap_or_else(|| "unknown".to_string()),
            source: error,
        }
    }
}

impl From<serde_json::Error> for GeekToolsError {
    fn from(error: serde_json::Error) -> Self {
        Self::ConfigError {
            message: format!("JSON parsing error: {}", error),
        }
    }
}

impl From<String> for GeekToolsError {
    fn from(error: String) -> Self {
        Self::ConfigError {
            message: error,
        }
    }
}
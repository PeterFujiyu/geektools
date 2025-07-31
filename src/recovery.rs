use std::time::Duration;
use std::thread;
use std::path::Path;
use crate::errors::{GeekToolsError, Result};
use crate::i18n::Language;

#[derive(Debug, Clone)]
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
                log::info!("Attempt {} failed, retrying in {:?}", attempt, delay);
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
                            if crate::fileio::create_dir(parent).is_ok() {
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

/// 带恢复机制的操作执行器
pub fn execute_with_recovery<T, F>(
    operation: F,
    recovery_handler: &RecoveryHandler,
    max_recovery_attempts: u32,
) -> Result<T>
where
    F: Fn() -> Result<T>,
{
    let mut recovery_attempts = 0;
    
    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(error) => {
                if recovery_attempts >= max_recovery_attempts {
                    return Err(error);
                }
                
                match recovery_handler.handle_error(&error) {
                    RecoveryAction::Retry => {
                        recovery_attempts += 1;
                        log::info!("Attempting recovery, attempt {}/{}", recovery_attempts, max_recovery_attempts);
                        continue;
                    }
                    RecoveryAction::RetryWithBackoff(config) => {
                        recovery_attempts += 1;
                        log::info!("Attempting recovery with backoff, attempt {}/{}", recovery_attempts, max_recovery_attempts);
                        
                        return retry_with_backoff(operation, &config);
                    }
                    RecoveryAction::ShowSuggestions(suggestions) => {
                        log::warn!("Recovery suggestions for error: {}", error);
                        for suggestion in suggestions {
                            log::warn!("  - {}", suggestion);
                        }
                        return Err(error);
                    }
                    RecoveryAction::Exit => {
                        return Err(error);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_retry_with_backoff_success() {
        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);
        
        let operation = move || {
            let mut count = attempt_count_clone.lock().unwrap();
            *count += 1;
            if *count < 3 {
                Err(GeekToolsError::NetworkError {
                    url: "test".to_string(),
                    source: reqwest::Error::from(reqwest::ErrorKind::Request),
                })
            } else {
                Ok("success")
            }
        };
        
        let config = RetryConfig::default();
        let result = retry_with_backoff(operation, &config);
        
        assert!(result.is_ok());
        assert_eq!(*attempt_count.lock().unwrap(), 3);
    }
    
    #[test]
    fn test_retry_with_backoff_max_attempts() {
        let operation = || {
            Err(GeekToolsError::NetworkError {
                url: "test".to_string(),
                source: reqwest::Error::from(reqwest::ErrorKind::Request),
            })
        };
        
        let config = RetryConfig {
            max_attempts: 2,
            ..Default::default()
        };
        
        let result = retry_with_backoff(operation, &config);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_non_recoverable_error() {
        let operation = || {
            Err(GeekToolsError::ValidationError {
                field: "test".to_string(),
                message: "test error".to_string(),
            })
        };
        
        let config = RetryConfig::default();
        let result = retry_with_backoff(operation, &config);
        
        // Should fail immediately for non-recoverable errors
        assert!(result.is_err());
    }
}
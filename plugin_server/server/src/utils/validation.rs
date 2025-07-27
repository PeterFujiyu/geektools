use regex::Regex;
use std::collections::HashSet;
use lazy_static::lazy_static;

lazy_static! {
    static ref PLUGIN_ID_REGEX: Regex = Regex::new(r"^[a-z0-9_-]+$").unwrap();
    static ref VERSION_REGEX: Regex = Regex::new(r"^\d+\.\d+\.\d+(-[a-zA-Z0-9]+)?$").unwrap();
    static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
}

pub fn validate_plugin_id_regex(id: &str) -> bool {
    PLUGIN_ID_REGEX.is_match(id)
}

pub fn validate_version_regex(version: &str) -> bool {
    VERSION_REGEX.is_match(version)
}

pub fn validate_username_regex(username: &str) -> bool {
    USERNAME_REGEX.is_match(username)
}

pub fn validate_plugin_id(id: &str) -> Result<(), String> {
    if id.len() < 3 || id.len() > 50 {
        return Err("Plugin ID must be between 3 and 50 characters".to_string());
    }

    let re = Regex::new(r"^[a-z0-9_-]+$").unwrap();
    if !re.is_match(id) {
        return Err("Plugin ID can only contain lowercase letters, numbers, underscores, and hyphens".to_string());
    }

    Ok(())
}

pub fn validate_version(version: &str) -> Result<(), String> {
    let re = Regex::new(r"^\d+\.\d+\.\d+(-[a-zA-Z0-9]+)?$").unwrap();
    if !re.is_match(version) {
        return Err("Version must follow semantic versioning (e.g., 1.0.0 or 1.0.0-beta)".to_string());
    }

    Ok(())
}

pub fn validate_tags(tags: &[String]) -> Result<(), String> {
    if tags.is_empty() {
        return Err("At least one tag is required".to_string());
    }

    if tags.len() > 10 {
        return Err("Maximum 10 tags allowed".to_string());
    }

    let mut unique_tags = HashSet::new();
    for tag in tags {
        if tag.len() > 50 {
            return Err("Tag length cannot exceed 50 characters".to_string());
        }

        if !unique_tags.insert(tag.to_lowercase()) {
            return Err(format!("Duplicate tag: {}", tag));
        }
    }

    Ok(())
}

pub fn validate_script_file(filename: &str) -> Result<(), String> {
    if filename.is_empty() {
        return Err("Script filename cannot be empty".to_string());
    }

    let allowed_extensions = vec![".sh", ".py", ".js", ".rb", ".pl", ".php"];
    let has_valid_extension = allowed_extensions.iter().any(|ext| filename.ends_with(ext));

    if !has_valid_extension {
        return Err(format!(
            "Script file must have one of these extensions: {}",
            allowed_extensions.join(", ")
        ));
    }

    // Check for potentially dangerous filenames
    let dangerous_patterns = vec!["../", "./", "~", "\\"];
    for pattern in dangerous_patterns {
        if filename.contains(pattern) {
            return Err("Script filename contains invalid characters".to_string());
        }
    }

    Ok(())
}

pub fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_plugin_id() {
        assert!(validate_plugin_id("my-plugin").is_ok());
        assert!(validate_plugin_id("my_plugin_123").is_ok());
        assert!(validate_plugin_id("ab").is_err()); // too short
        assert!(validate_plugin_id("My-Plugin").is_err()); // uppercase
        assert!(validate_plugin_id("my plugin").is_err()); // space
    }

    #[test]
    fn test_validate_version() {
        assert!(validate_version("1.0.0").is_ok());
        assert!(validate_version("1.0.0-beta").is_ok());
        assert!(validate_version("1.0").is_err()); // incomplete
        assert!(validate_version("v1.0.0").is_err()); // prefix
    }

    #[test]
    fn test_validate_tags() {
        assert!(validate_tags(&["tag1".to_string(), "tag2".to_string()]).is_ok());
        assert!(validate_tags(&[]).is_err()); // empty
        assert!(validate_tags(&["tag1".to_string(), "tag1".to_string()]).is_err()); // duplicate
    }

    #[test]
    fn test_validate_script_file() {
        assert!(validate_script_file("script.sh").is_ok());
        assert!(validate_script_file("script.py").is_ok());
        assert!(validate_script_file("script.txt").is_err()); // invalid extension
        assert!(validate_script_file("../script.sh").is_err()); // path traversal
    }
}
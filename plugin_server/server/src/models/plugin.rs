use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: String,
    pub current_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub downloads: i32,
    pub rating: f64,
    pub status: PluginStatus,
    pub min_geektools_version: Option<String>,
    pub homepage_url: Option<String>,
    pub repository_url: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "plugin_status", rename_all = "lowercase")]
pub enum PluginStatus {
    Active,
    Deprecated,
    Banned,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PluginVersion {
    pub id: i32,
    pub plugin_id: String,
    pub version: String,
    pub changelog: Option<String>,
    pub file_path: String,
    pub file_size: i64,
    pub file_hash: String,
    pub created_at: DateTime<Utc>,
    pub downloads: i32,
    pub is_stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PluginScript {
    pub id: i32,
    pub plugin_id: String,
    pub version: String,
    pub script_name: String,
    pub script_file: String,
    pub description: Option<String>,
    pub is_executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PluginTag {
    pub id: i32,
    pub plugin_id: String,
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PluginDependency {
    pub id: i32,
    pub plugin_id: String,
    pub dependency_id: String,
    pub min_version: Option<String>,
}

// Request/Response DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreatePluginRequest {
    #[validate(length(min = 3, max = 50))]
    pub id: String,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub author: String,
    pub version: String,
    pub min_geektools_version: Option<String>,
    pub homepage_url: Option<String>,
    pub repository_url: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    #[validate(length(min = 1))]
    pub scripts: Vec<PluginScriptInfo>,
    pub dependencies: Vec<PluginDependencyInfo>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct PluginScriptInfo {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1, max = 255))]
    pub file: String,
    pub description: Option<String>,
    pub executable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginDependencyInfo {
    pub id: String,
    pub min_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginListResponse {
    pub plugins: Vec<PluginSummary>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: String,
    pub current_version: String,
    pub downloads: i32,
    pub rating: f64,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginDetailResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: String,
    pub current_version: String,
    pub downloads: i32,
    pub rating: f64,
    pub tags: Vec<String>,
    pub min_geektools_version: Option<String>,
    pub homepage_url: Option<String>,
    pub repository_url: Option<String>,
    pub license: Option<String>,
    pub versions: Vec<PluginVersionInfo>,
    pub scripts: Vec<PluginScriptInfo>,
    pub dependencies: Vec<PluginDependencyInfo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginVersionInfo {
    pub version: String,
    pub changelog: Option<String>,
    pub file_size: i64,
    pub created_at: DateTime<Utc>,
    pub downloads: i32,
    pub is_stable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: i32,
    pub limit: i32,
    pub total: i64,
    pub pages: i32,
}

#[derive(Debug, Deserialize)]
pub struct PluginSearchQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub search: Option<String>,
    pub tag: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub plugin_id: String,
    pub version: String,
    pub upload_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginStatsResponse {
    pub total_downloads: i32,
    pub weekly_downloads: i32,
    pub monthly_downloads: i32,
    pub version_distribution: std::collections::HashMap<String, i32>,
    pub download_trend: Vec<DownloadTrendItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadTrendItem {
    pub date: String,
    pub downloads: i32,
}

impl Default for PluginStatus {
    fn default() -> Self {
        PluginStatus::Active
    }
}
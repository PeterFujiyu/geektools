use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ipnetwork::IpNetwork;
use validator::Validate;

#[derive(Debug, Clone, PartialEq)]
pub enum UserRole {
    User,
    Admin,
}

impl From<String> for UserRole {
    fn from(s: String) -> Self {
        match s.as_str() {
            "admin" => UserRole::Admin,
            _ => UserRole::User,
        }
    }
}

impl From<UserRole> for String {
    fn from(role: UserRole) -> Self {
        match role {
            UserRole::Admin => "admin".to_string(),
            UserRole::User => "user".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserLoginActivity {
    pub id: i32,
    pub user_id: i32,
    pub email: String,
    pub ip_address: Option<IpNetwork>,
    pub user_agent: Option<String>,
    pub login_time: DateTime<Utc>,
    pub logout_time: Option<DateTime<Utc>>,
    pub session_duration: Option<i32>,
    pub login_method: String,
    pub is_successful: bool,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AdminSqlLog {
    pub id: i32,
    pub admin_user_id: i32,
    pub admin_email: String,
    pub sql_query: String,
    pub execution_time_ms: Option<i32>,
    pub rows_affected: Option<i32>,
    pub is_successful: bool,
    pub error_message: Option<String>,
    pub ip_address: Option<IpNetwork>,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserProfileChange {
    pub id: i32,
    pub user_id: i32,
    pub changed_by_user_id: i32,
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub change_reason: Option<String>,
    pub ip_address: Option<IpNetwork>,
    pub changed_at: DateTime<Utc>,
}

// Request/Response DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateUserEmailRequest {
    pub user_id: i32,
    #[validate(email)]
    pub new_email: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ExecuteSqlRequest {
    #[validate(length(min = 1, max = 10000))]
    pub sql_query: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SqlExecutionResult {
    pub is_successful: bool,
    pub rows_affected: Option<i32>,
    pub execution_time_ms: i32,
    pub data: Option<Vec<serde_json::Value>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminDashboardStats {
    pub total_users: i64,
    pub total_plugins: i64,
    pub total_downloads: i64,
    pub active_sessions: i64,
    pub recent_logins: Vec<UserLoginActivity>,
    pub recent_sql_executions: Vec<AdminSqlLog>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserManagementInfo {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub login_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct AdminPaginationQuery {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,
    #[validate(range(min = 5, max = 100))]
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct DeletePluginRequest {
    pub plugin_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct BanUserRequest {
    pub user_id: i32,
    pub reason: Option<String>,
    pub ban_duration_days: Option<i32>, // None for permanent ban
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UnbanUserRequest {
    pub user_id: i32,
    pub reason: Option<String>,
}
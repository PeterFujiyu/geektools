use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use validator::Validate;

use crate::{
    handlers::{success_response, success_response_with_message, AppError, Result},
    middleware::auth::get_user_from_token,
    models::{AdminPaginationQuery, ExecuteSqlRequest, UpdateUserEmailRequest, DeletePluginRequest, BanUserRequest, UnbanUserRequest},
    services::AppState,
};

// Admin authentication middleware helper
async fn require_admin(headers: &HeaderMap, state: &AppState) -> Result<(i32, String)> {
    let user = get_user_from_token(headers, &state.auth_service).await?;
    
    if !state.admin_service.is_admin(user.id).await.map_err(|e| {
        AppError::Internal(format!("Failed to check admin status: {}", e))
    })? {
        return Err(AppError::Forbidden("需要管理员权限".to_string()));
    }

    Ok((user.id, user.email))
}

// Get client IP address
fn get_client_ip(headers: &HeaderMap) -> Option<std::net::IpAddr> {
    headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|ip| ip.trim().parse().ok())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .and_then(|ip| ip.parse().ok())
        })
}

// Get admin dashboard statistics
pub async fn get_dashboard_stats(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let (_admin_id, _admin_email) = require_admin(&headers, &state).await?;

    let stats = state
        .admin_service
        .get_dashboard_stats()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get dashboard stats: {}", e)))?;

    Ok(success_response(stats))
}

// Get users for management
pub async fn get_users_for_management(
    headers: HeaderMap,
    Query(pagination): Query<AdminPaginationQuery>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let (_admin_id, _admin_email) = require_admin(&headers, &state).await?;

    pagination.validate()?;

    let (users, total_count) = state
        .admin_service
        .get_users_for_management(pagination)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get users: {}", e)))?;

    let response = serde_json::json!({
        "users": users,
        "total_count": total_count
    });

    Ok(success_response(response))
}

// Update user email
pub async fn update_user_email(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUserEmailRequest>,
) -> Result<Json<serde_json::Value>> {
    let (admin_id, _admin_email) = require_admin(&headers, &state).await?;

    payload.validate()?;

    let ip_address = get_client_ip(&headers);

    state
        .admin_service
        .update_user_email(admin_id, payload, ip_address)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update user email: {}", e)))?;

    Ok(success_response_with_message(
        serde_json::json!({}),
        "用户邮箱更新成功",
    ))
}

// Execute SQL query
pub async fn execute_sql(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<ExecuteSqlRequest>,
) -> Result<Json<serde_json::Value>> {
    let (admin_id, admin_email) = require_admin(&headers, &state).await?;

    payload.validate()?;

    let ip_address = get_client_ip(&headers);

    let result = state
        .admin_service
        .execute_sql(admin_id, &admin_email, payload, ip_address)
        .await
        .map_err(|e| AppError::BadRequest(format!("SQL execution failed: {}", e)))?;

    Ok(success_response(result))
}

// Get user login activities
pub async fn get_user_login_activities(
    headers: HeaderMap,
    Query(params): Query<serde_json::Value>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let (_admin_id, _admin_email) = require_admin(&headers, &state).await?;

    let user_id = params
        .get("user_id")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let pagination = AdminPaginationQuery {
        page: params
            .get("page")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        limit: params
            .get("limit")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
    };

    pagination.validate()?;

    let (activities, total_count) = state
        .admin_service
        .get_user_login_activities(user_id, pagination)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get login activities: {}", e)))?;

    let response = serde_json::json!({
        "activities": activities,
        "total_count": total_count
    });

    Ok(success_response(response))
}

// Get recent login activities (for dashboard)
pub async fn get_recent_logins(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let (_admin_id, _admin_email) = require_admin(&headers, &state).await?;

    let pagination = AdminPaginationQuery {
        page: Some(1),
        limit: Some(20),
    };

    let (activities, _) = state
        .admin_service
        .get_user_login_activities(None, pagination)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get recent logins: {}", e)))?;

    Ok(success_response(activities))
}

// Delete plugin
pub async fn delete_plugin(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<DeletePluginRequest>,
) -> Result<Json<serde_json::Value>> {
    let (admin_id, _admin_email) = require_admin(&headers, &state).await?;

    payload.validate()?;

    let ip_address = get_client_ip(&headers);

    state
        .admin_service
        .delete_plugin(admin_id, payload, ip_address)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete plugin: {}", e)))?;

    Ok(success_response_with_message(
        serde_json::json!({}),
        "插件删除成功",
    ))
}

// Ban user
pub async fn ban_user(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<BanUserRequest>,
) -> Result<Json<serde_json::Value>> {
    let (admin_id, _admin_email) = require_admin(&headers, &state).await?;

    payload.validate()?;

    let ip_address = get_client_ip(&headers);

    state
        .admin_service
        .ban_user(admin_id, payload, ip_address)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to ban user: {}", e)))?;

    Ok(success_response_with_message(
        serde_json::json!({}),
        "用户封禁成功",
    ))
}

// Unban user
pub async fn unban_user(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<UnbanUserRequest>,
) -> Result<Json<serde_json::Value>> {
    let (admin_id, _admin_email) = require_admin(&headers, &state).await?;

    payload.validate()?;

    let ip_address = get_client_ip(&headers);

    state
        .admin_service
        .unban_user(admin_id, payload, ip_address)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to unban user: {}", e)))?;

    Ok(success_response_with_message(
        serde_json::json!({}),
        "用户解封成功",
    ))
}

// Get plugins for management
pub async fn get_plugins_for_management(
    headers: HeaderMap,
    Query(pagination): Query<AdminPaginationQuery>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let (_admin_id, _admin_email) = require_admin(&headers, &state).await?;

    pagination.validate()?;

    let (plugins, total_count) = state
        .admin_service
        .get_plugins_for_management(pagination)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get plugins: {}", e)))?;

    let response = serde_json::json!({
        "plugins": plugins,
        "total_count": total_count
    });

    Ok(success_response(response))
}
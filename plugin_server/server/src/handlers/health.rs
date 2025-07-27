use axum::{extract::State, Json};
use serde_json::json;

use crate::{
    handlers::{success_response, Result},
    services::AppState,
};

pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    // Check database connection with a timeout
    let db_status = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sqlx::query("SELECT 1 as test").fetch_one(&state.db_pool)
    ).await {
        Ok(Ok(_)) => "healthy",
        Ok(Err(e)) => {
            tracing::warn!("Database query failed: {:?}", e);
            "unhealthy"
        },
        Err(_) => {
            tracing::warn!("Database query timeout");
            "timeout"
        }
    };

    let response = json!({
        "status": if db_status == "healthy" { "healthy" } else { "unhealthy" },
        "timestamp": chrono::Utc::now(),
        "version": env!("CARGO_PKG_VERSION"),
        "services": {
            "database": db_status,
            "storage": "healthy"
        }
    });

    Ok(success_response(response))
}

pub async fn metrics(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    // Get some basic metrics
    let total_plugins = state
        .plugin_service
        .count_plugins(None, None)
        .await?;

    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db_pool)
        .await?;

    let total_downloads = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(downloads), 0) FROM plugins"
    )
    .fetch_one(&state.db_pool)
    .await?;

    // Calculate this week's new plugins
    let weekly_new = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM plugins WHERE created_at >= NOW() - INTERVAL '7 days'"
    )
    .fetch_one(&state.db_pool)
    .await?;

    let response = json!({
        "total_plugins": total_plugins,
        "total_users": total_users,
        "total_downloads": total_downloads,
        "weekly_new": weekly_new,
        "uptime_seconds": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    Ok(success_response(response))
}
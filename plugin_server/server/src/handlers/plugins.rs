use axum::{
    extract::{Multipart, Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use crate::{
    handlers::{success_response, success_response_with_message, AppError, Result},
    middleware::auth::Claims,
    models::{
        CreateRatingRequest, PaginationInfo, PluginListResponse,
        PluginSearchQuery,
    },
    services::AppState,
};

pub async fn list_plugins(
    State(state): State<AppState>,
    Query(query): Query<PluginSearchQuery>,
) -> Result<Json<serde_json::Value>> {
    // Simplified version for debugging
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * limit;

    // Try to get real plugins from database
    let plugins = match state
        .plugin_service
        .search_plugins(
            query.search.as_deref(),
            query.tag.as_deref(),
            query.sort.as_deref().unwrap_or("downloads"),
            query.order.as_deref().unwrap_or("desc"),
            limit,
            offset,
        )
        .await {
            Ok(plugins) => plugins,
            Err(e) => {
                tracing::error!("Failed to search plugins: {:?}", e);
                vec![]
            }
        };

    let total = match state
        .plugin_service
        .count_plugins(query.search.as_deref(), query.tag.as_deref())
        .await {
            Ok(count) => count,
            Err(e) => {
                tracing::error!("Failed to count plugins: {:?}", e);
                0i64
            }
        };

    let pagination = PaginationInfo {
        page,
        limit,
        total,
        pages: 0,
    };

    let response = PluginListResponse {
        plugins,
        pagination,
    };

    Ok(success_response(response))
}

pub async fn get_plugin(
    State(state): State<AppState>,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let plugin = state
        .plugin_service
        .get_plugin_detail(&plugin_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Plugin not found".to_string()))?;

    Ok(success_response(plugin))
}

pub async fn upload_plugin(
    State(state): State<AppState>,
    claims: Claims,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    while let Some(field) = multipart.next_field().await.map_err(|_| {
        AppError::BadRequest("Invalid multipart data".to_string())
    })? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "plugin_file" {
            let filename = field.file_name()
                .ok_or_else(|| AppError::BadRequest("No filename provided".to_string()))?
                .to_string();

            if !filename.ends_with(".tar.gz") {
                return Err(AppError::BadRequest(
                    "File must be a .tar.gz archive".to_string(),
                ));
            }

            let data = field.bytes().await.map_err(|_| {
                AppError::BadRequest("Failed to read file data".to_string())
            })?;

            if data.len() > 100 * 1024 * 1024 {
                return Err(AppError::BadRequest("File too large".to_string()));
            }

            let upload_id = Uuid::new_v4().to_string();
            let result = state
                .plugin_service
                .upload_plugin(data.to_vec(), claims.user_id, &upload_id)
                .await?;

            return Ok(success_response_with_message(
                result,
                "Plugin uploaded successfully",
            ));
        }
    }

    Err(AppError::BadRequest("No plugin file provided".to_string()))
}

// Temporary upload endpoint without authentication for testing
pub async fn upload_plugin_temp(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    while let Some(field) = multipart.next_field().await.map_err(|_| {
        AppError::BadRequest("Invalid multipart data".to_string())
    })? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "plugin_file" {
            let filename = field.file_name()
                .ok_or_else(|| AppError::BadRequest("No filename provided".to_string()))?
                .to_string();

            if !filename.ends_with(".tar.gz") {
                return Err(AppError::BadRequest(
                    "File must be a .tar.gz archive".to_string(),
                ));
            }

            let data = field.bytes().await.map_err(|_| {
                AppError::BadRequest("Failed to read file data".to_string())
            })?;

            let upload_id = uuid::Uuid::new_v4().to_string();
            let result = state
                .plugin_service
                .upload_plugin(data.to_vec(), 1, &upload_id) // Use temp user_id = 1
                .await?;

            return Ok(success_response_with_message(
                result,
                "Plugin uploaded successfully",
            ));
        }
    }

    Err(AppError::BadRequest("No plugin file provided".to_string()))
}

pub async fn download_plugin(
    State(state): State<AppState>,
    Path(plugin_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response> {
    let version = params.get("version").map(|s| s.as_str());
    
    let (file_path, filename) = state
        .plugin_service
        .get_download_info(&plugin_id, version)
        .await?
        .ok_or_else(|| AppError::NotFound("Plugin version not found".to_string()))?;

    // Increment download count
    state
        .plugin_service
        .increment_download_count(&plugin_id, version)
        .await?;

    // Read file
    let file_data = tokio::fs::read(&file_path).await.map_err(|_| {
        AppError::NotFound("Plugin file not found".to_string())
    })?;

    let headers = [
        (header::CONTENT_TYPE, "application/gzip"),
        (
            header::CONTENT_DISPOSITION,
            &format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    Ok((headers, file_data).into_response())
}

pub async fn get_plugin_stats(
    State(state): State<AppState>,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let stats = state
        .plugin_service
        .get_plugin_stats(&plugin_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Plugin not found".to_string()))?;

    Ok(success_response(stats))
}

pub async fn get_plugin_ratings(
    State(state): State<AppState>,
    Path(plugin_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let page: i32 = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let limit: i32 = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(20);

    let ratings = state
        .plugin_service
        .get_plugin_ratings(&plugin_id, page, limit)
        .await?;

    Ok(success_response(ratings))
}

pub async fn create_rating(
    State(state): State<AppState>,
    Path(plugin_id): Path<String>,
    claims: Claims,
    Json(payload): Json<CreateRatingRequest>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;

    let rating = state
        .plugin_service
        .create_or_update_rating(&plugin_id, claims.user_id, payload.rating, payload.review)
        .await?;

    Ok(success_response_with_message(
        rating,
        "Rating created successfully",
    ))
}
use axum::{extract::{Query, State}, Json};
use serde_json::json;
use std::collections::HashMap;

use crate::{
    handlers::{success_response, Result},
    services::AppState,
};

pub async fn advanced_search(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let query = payload.get("query").and_then(|v| v.as_str()).unwrap_or("");
    let page = payload.get("pagination")
        .and_then(|p| p.get("page"))
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as i32;
    let limit = payload.get("pagination")
        .and_then(|p| p.get("limit"))
        .and_then(|v| v.as_i64())
        .unwrap_or(20) as i32;

    let filters = payload.get("filters");
    let tag = filters
        .and_then(|f| f.get("tags"))
        .and_then(|t| t.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str());

    let sort_field = payload.get("sort")
        .and_then(|s| s.get("field"))
        .and_then(|v| v.as_str())
        .unwrap_or("downloads");
    let sort_order = payload.get("sort")
        .and_then(|s| s.get("order"))
        .and_then(|v| v.as_str())
        .unwrap_or("desc");

    let offset = (page - 1) * limit;

    let plugins = state
        .plugin_service
        .search_plugins(
            if query.is_empty() { None } else { Some(query) },
            tag,
            sort_field,
            sort_order,
            limit,
            offset,
        )
        .await?;

    let total = state
        .plugin_service
        .count_plugins(
            if query.is_empty() { None } else { Some(query) },
            tag,
        )
        .await?;

    let response = json!({
        "plugins": plugins,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "pages": ((total as f64) / (limit as f64)).ceil() as i32
        }
    });

    Ok(success_response(response))
}

pub async fn search_suggestions(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    
    if query.len() < 2 {
        return Ok(success_response(json!({
            "suggestions": []
        })));
    }

    let suggestions = state
        .plugin_service
        .get_search_suggestions(query)
        .await?;

    Ok(success_response(json!({
        "suggestions": suggestions
    })))
}
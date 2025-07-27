use axum::{extract::State, Json};
use validator::Validate;

use crate::{
    handlers::{success_response, success_response_with_message, AppError, Result},
    models::{LoginRequest, RegisterRequest, SendVerificationCodeRequest, VerifyCodeRequest, SendCodeResponse},
    services::AppState,
};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;

    // Check if username or email already exists
    if state.auth_service.user_exists(&payload.username, &payload.email).await? {
        return Err(AppError::BadRequest(
            "Username or email already exists".to_string(),
        ));
    }

    let user = state
        .auth_service
        .register_user(payload.username, payload.email, payload.password, payload.display_name)
        .await?;

    Ok(success_response_with_message(
        user,
        "User registered successfully",
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;

    let login_response = state
        .auth_service
        .authenticate(&payload.username, &payload.password)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    Ok(success_response(login_response))
}

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let refresh_token = payload
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Refresh token required".to_string()))?;

    let login_response = state
        .auth_service
        .refresh_token(refresh_token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid refresh token".to_string()))?;

    Ok(success_response(login_response))
}

// Email verification code endpoints
pub async fn send_verification_code(
    State(state): State<AppState>,
    Json(payload): Json<SendVerificationCodeRequest>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;

    match state.auth_service.send_verification_code(payload.email).await {
        Ok(code) => {
            // Since SMTP is not configured, return the code in response
            let response = SendCodeResponse {
                message: "验证码已生成，请查看下方显示的验证码".to_string(),
                code: Some(code), // Return code when SMTP not configured
            };
            Ok(success_response(response))
        }
        Err(e) => Err(AppError::Internal(format!("Failed to send verification code: {}", e))),
    }
}

pub async fn verify_code_and_login(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<VerifyCodeRequest>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;

    // Extract IP address and user agent
    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|ip| ip.trim().parse().ok())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .and_then(|ip| ip.parse().ok())
        });

    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok());

    match state.auth_service.verify_code_and_auth(payload.email, payload.code, ip_address, user_agent).await {
        Ok(auth_response) => Ok(success_response_with_message(
            auth_response,
            "登录成功",
        )),
        Err(e) => Err(AppError::BadRequest(e.to_string())),
    }
}
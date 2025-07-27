use axum::{
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct RateLimitEntry {
    last_request: Instant,
    request_count: u32,
}

pub struct RateLimitService {
    limiters: Arc<Mutex<HashMap<String, RateLimitEntry>>>,
    max_requests: u32,
    window_duration: Duration,
}

impl RateLimitService {
    pub fn new() -> Self {
        Self {
            limiters: Arc::new(Mutex::new(HashMap::new())),
            max_requests: 10,
            window_duration: Duration::from_secs(60),
        }
    }

    pub async fn check_rate_limit(
        &self,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        request: Request<axum::body::Body>,
        next: Next,
    ) -> Result<Response, impl IntoResponse> {
        let ip = addr.ip().to_string();
        let now = Instant::now();
        
        let mut limiters = self.limiters.lock().unwrap();
        let entry = limiters.entry(ip.clone()).or_insert_with(|| {
            RateLimitEntry {
                last_request: now,
                request_count: 0,
            }
        });

        // Reset counter if window has passed
        if now.duration_since(entry.last_request) > self.window_duration {
            entry.request_count = 0;
            entry.last_request = now;
        }

        entry.request_count += 1;

        if entry.request_count > self.max_requests {
            let error_response = Json(json!({
                "success": false,
                "error": "Rate limit exceeded"
            }));
            
            Err((StatusCode::TOO_MANY_REQUESTS, error_response))
        } else {
            drop(limiters); // Release the lock before calling next
            Ok(next.run(request).await)
        }
    }
}
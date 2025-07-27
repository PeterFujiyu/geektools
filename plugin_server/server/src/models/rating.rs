use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PluginRating {
    pub id: i32,
    pub plugin_id: String,
    pub user_id: i32,
    pub rating: i32,
    pub review: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateRatingRequest {
    #[validate(range(min = 1, max = 5))]
    pub rating: i32,
    #[validate(length(max = 1000))]
    pub review: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RatingResponse {
    pub id: i32,
    pub plugin_id: String,
    pub user_id: i32,
    pub username: String,
    pub rating: i32,
    pub review: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RatingStatsResponse {
    pub average_rating: f64,
    pub total_ratings: i32,
    pub rating_distribution: RatingDistribution,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RatingDistribution {
    pub five_star: i32,
    pub four_star: i32,
    pub three_star: i32,
    pub two_star: i32,
    pub one_star: i32,
}
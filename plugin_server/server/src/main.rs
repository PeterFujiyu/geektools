mod models;
mod handlers;
mod services;
mod utils;
mod middleware;

use axum::{
    extract::DefaultBodyLimit,
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    routing::{get, post},
    Router,
};
use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use handlers::{auth, plugins, search, health, admin};
use services::AppState;
use utils::config::Config;

#[derive(Parser)]
#[command(name = "geektools-marketplace-server")]
#[command(about = "GeekTools Plugin Marketplace Server")]
struct Cli {
    #[arg(short, long, default_value = "config/config.yaml")]
    config: String,
    
    #[arg(short, long)]
    port: Option<u16>,
    
    #[arg(short, long)]
    database_url: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Load configuration
    let config = Config::from_file(&cli.config)?;
    
    // Override config with CLI arguments
    let port = cli.port.unwrap_or(config.server.port);
    let database_url = cli.database_url.unwrap_or(config.database.url.clone());

    info!("Starting GeekTools Marketplace Server on port {}", port);

    // Connect to database
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Create application state
    let state = AppState::new(pool, config).await?;

    // Build application router
    let app = create_app(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("Server listening on http://0.0.0.0:{}", port);
    
    axum::serve(listener, app).await?;

    Ok(())
}

fn create_app(state: AppState) -> Router {
    // CORS configuration - Allow specific origins with credentials
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:8080".parse::<HeaderValue>().unwrap())
        .allow_origin("http://127.0.0.1:8080".parse::<HeaderValue>().unwrap())
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_origin("http://127.0.0.1:3000".parse::<HeaderValue>().unwrap())
        .allow_origin("null".parse::<HeaderValue>().unwrap()) // For file:// protocol
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE])
        .allow_credentials(true);

    // API routes
    let api_routes = Router::new()
        // Auth routes
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh_token))
        .route("/auth/send-code", post(auth::send_verification_code))
        .route("/auth/verify-code", post(auth::verify_code_and_login))
        
        // Plugin routes
        .route("/plugins", get(plugins::list_plugins))
        .route("/plugins", post(plugins::upload_plugin))
        .route("/plugins/upload", post(plugins::upload_plugin_temp)) // Temporary endpoint without auth
        .route("/plugins/:id", get(plugins::get_plugin))
        .route("/plugins/:id/download", get(plugins::download_plugin))
        .route("/plugins/:id/stats", get(plugins::get_plugin_stats))
        .route("/plugins/:id/ratings", get(plugins::get_plugin_ratings))
        .route("/plugins/:id/ratings", post(plugins::create_rating))
        
        // Search routes
        .route("/search", post(search::advanced_search))
        .route("/search/suggestions", get(search::search_suggestions))
        
        // Health check
        .route("/health", get(health::health_check))
        .route("/metrics", get(health::metrics))
        
        // Admin routes
        .route("/admin/dashboard", get(admin::get_dashboard_stats))
        .route("/admin/users", get(admin::get_users_for_management))
        .route("/admin/users/update-email", post(admin::update_user_email))
        .route("/admin/users/ban", post(admin::ban_user))
        .route("/admin/users/unban", post(admin::unban_user))
        .route("/admin/plugins", get(admin::get_plugins_for_management))
        .route("/admin/plugins/delete", post(admin::delete_plugin))
        .route("/admin/sql/execute", post(admin::execute_sql))
        .route("/admin/login-activities", get(admin::get_user_login_activities))
        .route("/admin/recent-logins", get(admin::get_recent_logins))
        
        .with_state(state);

    // Main application
    Router::new()
        .nest("/api/v1", api_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(cors)
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB max file size
        )
}
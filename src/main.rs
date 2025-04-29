use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use std::{sync::Arc, path::PathBuf, net::SocketAddr};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod models;
mod orders;
mod price;
mod swap;
mod utils;
mod wallet;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Solana Wallet API server");

    // Initialize application state
    let app_state = Arc::new(models::AppState::new());

    // Start the background task for checking limit orders
    let orders_task = {
        let app_state = app_state.clone();
        tokio::spawn(async move {
            orders::monitor_limit_orders(app_state).await;
        })
    };

    // Build our application with routes
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // API Router
    let api_router = Router::new()
        .route("/health", get(api::health_check))
        .route("/generate_wallet", post(api::generate_wallet))
        .route("/import_wallet", post(api::import_wallet))
        .route("/get_balances", get(api::get_balances))
        .route("/get_prices", get(api::get_prices))
        .route("/swap_token", post(api::swap_token))
        .route("/set_limit_order", post(api::set_limit_order))
        .route("/list_limit_orders", get(api::list_limit_orders))
        .route("/cancel_limit_order", post(api::cancel_limit_order))
        .with_state(app_state)
        .layer(cors);

    // Serve static files from the "static" directory
    let static_service = ServeDir::new("static");

    // Build the complete app by nesting routers
    let app = Router::new()
        .nest("/", api_router)  // API endpoints at the root
        .fallback_service(static_service);  // Serve static files for unmatched routes

    // Get host and port from environment variables or use defaults
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .map(|p| p.parse::<u16>().unwrap_or(3000))
        .unwrap_or(3000);
    
    // Create socket address
    let addr = SocketAddr::from((host.parse::<std::net::IpAddr>().unwrap_or_else(|_| {
        "127.0.0.1".parse().unwrap() 
    }), port));
    
    info!("Server listening on {}", addr);
    info!("Web interface available at http://{}:{}", 
          host, port);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    // This won't be reached in normal operation
    orders_task.abort();
    Ok(())
} 
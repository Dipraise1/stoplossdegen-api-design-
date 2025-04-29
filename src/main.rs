use axum::{
    routing::{get, post},
    extract::{Extension, Json},
    Router, response::{IntoResponse},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use std::path::PathBuf;

// Our application state
#[derive(Clone)]
struct AppState {
    counter: Arc<Mutex<i32>>,
}

// Request for our increment endpoint
#[derive(Deserialize)]
struct IncrementRequest {
    value: i32,
}

// Response for our endpoints
#[derive(Serialize)]
struct CounterResponse {
    counter: i32,
}

// Health check handler
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// Handler for incrementing our counter
async fn increment(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<IncrementRequest>,
) -> impl IntoResponse {
    let mut counter = state.counter.lock().unwrap();
    *counter += req.value;
    
    Json(CounterResponse {
        counter: *counter,
    })
}

// Handler for decrementing our counter
async fn decrement(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<IncrementRequest>,
) -> impl IntoResponse {
    let mut counter = state.counter.lock().unwrap();
    *counter -= req.value;
    
    Json(CounterResponse {
        counter: *counter,
    })
}

// Handler for getting the current counter value
async fn get_counter(
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    let counter = state.counter.lock().unwrap();
    
    Json(CounterResponse {
        counter: *counter,
    })
}

#[tokio::main]
async fn main() {
    // Initialize application state
    let app_state = Arc::new(AppState {
        counter: Arc::new(Mutex::new(0)),
    });

    // Create CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Get the static directory path
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");
    println!("Serving static files from: {}", static_dir.display());

    // Build our application with routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/increment", post(increment))
        .route("/decrement", post(decrement))
        .route("/counter", get(get_counter))
        .layer(Extension(app_state))
        .layer(cors)
        // Serve static files from the static directory
        .nest_service("/", ServeDir::new(static_dir));

    // Define our address
    let addr = SocketAddr::from(([127, 0, 0, 1], 3301));
    println!("Server running on http://{}", addr);

    // Start the server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

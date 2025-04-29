use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

#[derive(Serialize, Deserialize)]
struct ImportWalletRequest {
    private_key: Option<String>,
    mnemonic: Option<String>,
}

#[derive(Default)]
struct AppState {
    wallet_count: Mutex<usize>,
}

async fn import_wallet(Json(payload): Json<ImportWalletRequest>) -> Json<Value> {
    Json(json!({
        "success": true,
        "data": {
            "pubkey": "simulated_public_key"
        }
    }))
}

async fn get_balances() -> Json<Value> {
    Json(json!({
        "success": true,
        "data": [
            {
                "mint": "So11111111111111111111111111111111111111112",
                "symbol": "SOL",
                "amount": 1000000000,
                "decimals": 9,
                "ui_amount": 1.0
            },
            {
                "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "symbol": "USDC",
                "amount": 1000000,
                "decimals": 6,
                "ui_amount": 1.0
            }
        ]
    }))
}

async fn get_prices() -> Json<Value> {
    Json(json!({
        "success": true,
        "data": [
            {
                "mint": "So11111111111111111111111111111111111111112",
                "symbol": "SOL",
                "price_usd": 100.5,
                "last_updated": "2023-05-29T15:30:00Z"
            },
            {
                "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "symbol": "USDC",
                "price_usd": 1.0,
                "last_updated": "2023-05-29T15:30:00Z"
            }
        ]
    }))
}

async fn swap_token() -> Json<Value> {
    Json(json!({
        "success": true,
        "data": {
            "transaction_signature": "simulated_transaction",
            "source_amount": 0.1,
            "target_amount": 10.05,
            "fee": 0.0,
            "success": true,
            "timestamp": "2023-05-29T15:31:00Z"
        }
    }))
}

async fn set_limit_order() -> Json<Value> {
    Json(json!({
        "success": true,
        "data": {
            "id": "order123",
            "source_token": "So11111111111111111111111111111111111111112",
            "target_token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "amount": 0.1,
            "price_target": 102.5,
            "order_type": "Buy",
            "status": "Active",
            "created_at": "2023-05-29T15:32:00Z",
            "updated_at": "2023-05-29T15:32:00Z",
            "expiry_time": null,
            "slippage": 0.5,
            "transaction_signature": null
        }
    }))
}

async fn list_limit_orders() -> Json<Value> {
    Json(json!({
        "success": true,
        "data": [
            {
                "id": "order123",
                "source_token": "So11111111111111111111111111111111111111112",
                "target_token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "amount": 0.1,
                "price_target": 102.5,
                "order_type": "Buy",
                "status": "Active",
                "created_at": "2023-05-29T15:32:00Z",
                "updated_at": "2023-05-29T15:32:00Z",
                "expiry_time": null,
                "slippage": 0.5,
                "transaction_signature": null
            }
        ]
    }))
}

async fn cancel_limit_order() -> Json<Value> {
    Json(json!({
        "success": true,
        "data": {
            "id": "order123",
            "source_token": "So11111111111111111111111111111111111111112",
            "target_token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "amount": 0.1,
            "price_target": 102.5,
            "order_type": "Buy",
            "status": "Cancelled",
            "created_at": "2023-05-29T15:32:00Z",
            "updated_at": "2023-05-29T15:33:00Z",
            "expiry_time": null,
            "slippage": 0.5,
            "transaction_signature": null
        }
    }))
}

#[tokio::main]
async fn main() {
    // Create app state
    let shared_state = Arc::new(AppState::default());

    // CORS configuration
    let cors = CorsLayer::new().allow_origin(Any);

    // Build our application with routes
    let app = Router::new()
        .route("/import_wallet", post(import_wallet))
        .route("/get_balances", get(get_balances))
        .route("/get_prices", get(get_prices))
        .route("/swap_token", post(swap_token))
        .route("/set_limit_order", post(set_limit_order))
        .route("/list_limit_orders", get(list_limit_orders))
        .route("/cancel_limit_order", post(cancel_limit_order))
        .layer(cors)
        .with_state(shared_state);

    // Run our application as a server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on {}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
} 
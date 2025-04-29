use axum::{
    extract::{Json, Extension},
    response::IntoResponse,
};
use std::sync::Arc;

use crate::api;
use crate::models::{
    AppState, CancelOrderRequest, ImportWalletRequest, LimitOrderRequest, SwapRequest,
};

// Simple handler for health check
pub async fn health() -> impl IntoResponse {
    (axum::http::StatusCode::OK, "OK")
}

// Handler for wallet generation
pub async fn generate_wallet(
    Extension(app_state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    let app_state = app_state;
    
    // Generate a new wallet
    match crate::wallet::generate_new_wallet() {
        Ok((wallet, mnemonic)) => {
            let pubkey = wallet.pubkey.to_string();
            
            // Store the wallet in app state
            let mut wallets = app_state.wallets.lock().unwrap();
            wallets.insert(pubkey.clone(), wallet);
            
            // Return both the pubkey and mnemonic
            let response = crate::models::CreateWalletResponse {
                pubkey,
                mnemonic,
            };
            
            crate::utils::build_success_response(response)
        }
        Err(err) => {
            crate::utils::build_error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to generate wallet: {}", err)
            )
        }
    }
}

// Handler for wallet import
pub async fn import_wallet(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(req): Json<ImportWalletRequest>,
) -> impl IntoResponse {
    let app_state = app_state;
    
    // Import wallet based on the type of key provided
    let wallet_result = if let Some(private_key) = req.private_key {
        crate::wallet::import_from_private_key(&private_key)
    } else if let Some(mnemonic) = req.mnemonic {
        crate::wallet::import_from_mnemonic(&mnemonic)
    } else {
        return crate::utils::build_error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "Either private_key or mnemonic must be provided"
        );
    };
    
    // Handle the import result
    match wallet_result {
        Ok(wallet) => {
            let pubkey = wallet.pubkey.to_string();
            
            // Store the wallet in app state
            let mut wallets = app_state.wallets.lock().unwrap();
            wallets.insert(pubkey.clone(), wallet);
            
            crate::utils::build_success_response(serde_json::json!({
                "pubkey": pubkey
            }))
        }
        Err(err) => {
            crate::utils::build_error_response(
                axum::http::StatusCode::BAD_REQUEST,
                &format!("Failed to import wallet: {}", err)
            )
        }
    }
}

// Handler for getting balances
pub async fn get_balances(
    Extension(app_state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    let app_state = app_state;
    
    // Get the wallets (for now, just use the first one if any)
    let wallets = app_state.wallets.lock().unwrap();
    
    if wallets.is_empty() {
        return crate::utils::build_error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "No wallet imported"
        );
    }
    
    // Use the first wallet
    let wallet = wallets.values().next().unwrap();
    
    // Get balances
    match crate::wallet::get_token_balances(wallet).await {
        Ok(balances) => crate::utils::build_success_response(balances),
        Err(err) => {
            crate::utils::build_error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get balances: {}", err)
            )
        }
    }
}

// Handler for getting prices
pub async fn get_prices(
    Extension(app_state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    let app_state = app_state;
    
    // Update prices first
    if let Err(err) = crate::price::update_prices(app_state.clone()).await {
        return crate::utils::build_error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to update prices: {}", err)
        );
    }
    
    // Get prices from app state
    let price_map = app_state.token_prices.lock().unwrap();
    
    // Convert to a Vec of TokenPrice for the response
    let prices = price_map
        .iter()
        .map(|(mint, price)| {
            serde_json::json!({
                "mint": mint,
                "symbol": crate::wallet::KnownTokens::get_symbol(mint),
                "price_usd": price,
                "last_updated": chrono::Utc::now().to_rfc3339()
            })
        })
        .collect::<Vec<_>>();
    
    crate::utils::build_success_response(prices)
}

// Handler for swapping tokens
pub async fn swap_token(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<SwapRequest>,
) -> impl IntoResponse {
    let app_state = app_state;
    
    // Validate the request
    if let Err(err) = crate::utils::validate_amount(request.amount) {
        return crate::utils::build_error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &err.to_string()
        );
    }
    
    // Get the wallet
    let wallets = app_state.wallets.lock().unwrap();
    
    if wallets.is_empty() {
        return crate::utils::build_error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "No wallet imported"
        );
    }
    
    // Use the first wallet
    let wallet = wallets.values().next().unwrap();
    
    // Check if the wallet has sufficient balance
    match crate::wallet::has_sufficient_balance(wallet, &request.source_token, request.amount).await {
        Ok(has_balance) => {
            if !has_balance {
                return crate::utils::build_error_response(
                    axum::http::StatusCode::BAD_REQUEST,
                    &format!(
                        "Insufficient balance of {} to execute swap", 
                        crate::wallet::KnownTokens::get_symbol(&request.source_token)
                    )
                );
            }
        },
        Err(err) => {
            return crate::utils::build_error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to check balance: {}", err)
            );
        }
    }
    
    // Execute the swap
    match crate::swap::execute_swap(wallet, &request).await {
        Ok(result) => crate::utils::build_success_response(result),
        Err(err) => {
            crate::utils::build_error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to execute swap: {}", err)
            )
        }
    }
}

// Handler for setting limit orders
pub async fn set_limit_order(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<LimitOrderRequest>,
) -> impl IntoResponse {
    let app_state = app_state;
    
    if request.price_target <= 0.0 {
        return crate::utils::build_error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "Price target must be greater than zero"
        );
    }
    
    match crate::orders::create_limit_order(app_state, request).await {
        Ok(order) => crate::utils::build_success_response(order),
        Err(err) => {
            crate::utils::build_error_response(
                axum::http::StatusCode::BAD_REQUEST,
                &format!("Failed to create limit order: {}", err)
            )
        }
    }
}

// Handler for listing limit orders
pub async fn list_limit_orders(
    Extension(app_state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    let app_state = app_state;
    let orders = crate::orders::get_limit_orders(app_state);
    crate::utils::build_success_response(orders)
}

// Handler for canceling limit orders
pub async fn cancel_limit_order(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<CancelOrderRequest>,
) -> impl IntoResponse {
    let app_state = app_state;
    
    match crate::orders::cancel_limit_order(app_state, &request.order_id) {
        Ok(order) => crate::utils::build_success_response(order),
        Err(err) => {
            crate::utils::build_error_response(
                axum::http::StatusCode::BAD_REQUEST,
                &err.to_string()
            )
        }
    }
} 
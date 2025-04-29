use crate::models::{
    AppState, CancelOrderRequest, ImportWalletRequest, LimitOrderRequest, SwapRequest, CreateWalletResponse,
};
use crate::orders;
use crate::price;
use crate::swap;
use crate::utils;
use crate::wallet;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use tracing::{error, info};

// Handler for health check
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// Handler for generating a new wallet
pub async fn generate_wallet(
    state: State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Generating new wallet");
    
    let app_state = state.0;
    
    // Generate a new wallet
    match wallet::generate_new_wallet() {
        Ok((wallet, mnemonic)) => {
            let pubkey = wallet.pubkey.to_string();
            
            // Store the wallet in app state
            let mut wallets = app_state.wallets.lock().unwrap();
            wallets.insert(pubkey.clone(), wallet);
            
            info!("Wallet generated successfully: {}", pubkey);
            
            // Return both the pubkey and mnemonic (IMPORTANT: In a real app, ensure mnemonic is transmitted securely)
            let response = CreateWalletResponse {
                pubkey,
                mnemonic,
            };
            
            (StatusCode::OK, utils::build_success_response(response))
        }
        Err(err) => {
            error!("Failed to generate wallet: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                utils::build_error_response(&format!("Failed to generate wallet: {}", err)),
            )
        }
    }
}

// Handler for importing a wallet
pub async fn import_wallet(
    state: State<Arc<AppState>>,
    Json(request): Json<ImportWalletRequest>,
) -> impl IntoResponse {
    info!("Importing wallet");
    
    let app_state = state.0;
    
    // Import wallet based on the type of key provided
    let wallet_result = if let Some(private_key) = request.private_key {
        wallet::import_from_private_key(&private_key)
    } else if let Some(mnemonic) = request.mnemonic {
        wallet::import_from_mnemonic(&mnemonic)
    } else {
        return (
            StatusCode::BAD_REQUEST,
            utils::build_error_response("Either private_key or mnemonic must be provided"),
        );
    };
    
    // Handle the import result
    match wallet_result {
        Ok(wallet) => {
            let pubkey = wallet.pubkey.to_string();
            
            // Store the wallet in app state
            let mut wallets = app_state.wallets.lock().unwrap();
            wallets.insert(pubkey.clone(), wallet);
            
            info!("Wallet imported successfully: {}", pubkey);
            
            (
                StatusCode::OK,
                utils::build_success_response(serde_json::json!({
                    "pubkey": pubkey
                })),
            )
        }
        Err(err) => {
            error!("Failed to import wallet: {}", err);
            (
                StatusCode::BAD_REQUEST,
                utils::build_error_response(&format!("Failed to import wallet: {}", err)),
            )
        }
    }
}

// Handler for getting wallet balances
pub async fn get_balances(state: State<Arc<AppState>>) -> impl IntoResponse {
    info!("Getting wallet balances");
    
    let app_state = state.0;
    
    // Get the wallets (for now, just use the first one if any)
    let wallets = app_state.wallets.lock().unwrap();
    
    if wallets.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            utils::build_error_response("No wallet imported"),
        );
    }
    
    // Use the first wallet
    let wallet = wallets.values().next().unwrap();
    
    // Get balances
    match wallet::get_token_balances(wallet).await {
        Ok(balances) => (StatusCode::OK, utils::build_success_response(balances)),
        Err(err) => {
            error!("Failed to get balances: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                utils::build_error_response(&format!("Failed to get balances: {}", err)),
            )
        }
    }
}

// Handler for getting token prices
pub async fn get_prices(
    state: State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Getting token prices");
    
    let app_state = state.0;
    
    // Update prices first
    if let Err(err) = price::update_prices(app_state.clone()).await {
        error!("Failed to update prices: {}", err);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            utils::build_error_response(&format!("Failed to update prices: {}", err)),
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
                "symbol": wallet::KnownTokens::get_symbol(mint),
                "price_usd": price,
                "last_updated": chrono::Utc::now().to_rfc3339()
            })
        })
        .collect::<Vec<_>>();
    
    (StatusCode::OK, utils::build_success_response(prices))
}

// Handler for swapping tokens
pub async fn swap_token(
    state: State<Arc<AppState>>,
    Json(request): Json<SwapRequest>,
) -> impl IntoResponse {
    info!(
        "Swapping {} of {} to {}",
        request.amount, request.source_token, request.target_token
    );
    
    let app_state = state.0;
    
    // Validate the request
    if let Err(err) = utils::validate_amount(request.amount) {
        return (
            StatusCode::BAD_REQUEST,
            utils::build_error_response(&err.to_string()),
        );
    }
    
    // Get the wallet
    let wallets = app_state.wallets.lock().unwrap();
    
    if wallets.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            utils::build_error_response("No wallet imported"),
        );
    }
    
    // Use the first wallet
    let wallet = wallets.values().next().unwrap();
    
    // Check if wallet has sufficient balance before executing the swap
    match wallet::has_sufficient_balance(wallet, &request.source_token, request.amount).await {
        Ok(has_balance) => {
            if !has_balance {
                return (
                    StatusCode::BAD_REQUEST,
                    utils::build_error_response(&format!(
                        "Insufficient balance of {} to execute swap", 
                        wallet::KnownTokens::get_symbol(&request.source_token)
                    )),
                );
            }
        },
        Err(err) => {
            error!("Failed to check balance: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                utils::build_error_response(&format!("Failed to check balance: {}", err)),
            );
        }
    }
    
    // Execute the swap
    match swap::execute_swap(wallet, &request).await {
        Ok(result) => (StatusCode::OK, utils::build_success_response(result)),
        Err(err) => {
            error!("Failed to execute swap: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                utils::build_error_response(&format!("Failed to execute swap: {}", err)),
            )
        }
    }
}

// Handler for setting a limit order
pub async fn set_limit_order(
    state: State<Arc<AppState>>,
    Json(request): Json<LimitOrderRequest>,
) -> impl IntoResponse {
    info!(
        "Setting {} limit order: {} of {} to {} at price {}",
        request.order_type, request.amount, request.source_token, request.target_token, request.price_target
    );
    
    let app_state = state.0;
    
    // Validate the request
    if let Err(err) = utils::validate_amount(request.amount) {
        return (
            StatusCode::BAD_REQUEST,
            utils::build_error_response(&err.to_string()),
        );
    }
    
    if request.price_target <= 0.0 {
        return (
            StatusCode::BAD_REQUEST,
            utils::build_error_response("Price target must be greater than zero"),
        );
    }
    
    // Get the wallets (for now, just check if any exist)
    let wallets_exist = {
        let wallets = app_state.wallets.lock().unwrap();
        !wallets.is_empty()
    };
    
    if !wallets_exist {
        return (
            StatusCode::BAD_REQUEST,
            utils::build_error_response("No wallet imported"),
        );
    }
    
    // Create the limit order
    match orders::create_limit_order(app_state.clone(), request).await {
        Ok(order) => (StatusCode::OK, utils::build_success_response(order)),
        Err(err) => {
            error!("Failed to create limit order: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                utils::build_error_response(&format!("Failed to create limit order: {}", err)),
            )
        }
    }
}

// Handler for listing limit orders
pub async fn list_limit_orders(
    state: State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Listing limit orders");
    
    let app_state = state.0;
    let limit_orders = orders::get_limit_orders(app_state);
    
    (StatusCode::OK, utils::build_success_response(limit_orders))
}

// Handler for cancelling a limit order
pub async fn cancel_limit_order(
    state: State<Arc<AppState>>,
    Json(request): Json<CancelOrderRequest>,
) -> impl IntoResponse {
    info!("Cancelling limit order: {}", request.order_id);
    
    let app_state = state.0;
    match orders::cancel_limit_order(app_state, &request.order_id) {
        Ok(order) => (StatusCode::OK, utils::build_success_response(order)),
        Err(err) => {
            error!("Failed to cancel limit order: {}", err);
            (
                StatusCode::BAD_REQUEST,
                utils::build_error_response(&err.to_string()),
            )
        }
    }
} 
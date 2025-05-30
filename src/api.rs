use crate::models::{
    AppState, CancelOrderRequest, ImportWalletRequest, LimitOrderRequest, SwapRequest, CreateWalletResponse,
};
use crate::orders;
use crate::price;
use crate::swap;
use crate::utils;
use crate::wallet;
use axum::{
    extract::{Json, Extension},
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
    Extension(app_state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Generating new wallet");
    
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
            
            utils::build_success_response(response)
        }
        Err(err) => {
            error!("Failed to generate wallet: {}", err);
            utils::build_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to generate wallet: {}", err)
            )
        }
    }
}

// Handler for importing a wallet
pub async fn import_wallet(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<ImportWalletRequest>,
) -> impl IntoResponse {
    info!("Importing wallet");
    
    // Import wallet based on the type of key provided
    let wallet_result = if let Some(private_key) = request.private_key {
        wallet::import_from_private_key(&private_key)
    } else if let Some(mnemonic) = request.mnemonic {
        wallet::import_from_mnemonic(&mnemonic)
    } else {
        return utils::build_error_response(
            StatusCode::BAD_REQUEST,
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
            
            info!("Wallet imported successfully: {}", pubkey);
            
            utils::build_success_response(serde_json::json!({
                "pubkey": pubkey
            }))
        }
        Err(err) => {
            error!("Failed to import wallet: {}", err);
            utils::build_error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to import wallet: {}", err)
            )
        }
    }
}

// Handler for getting wallet balances
pub async fn get_balances(
    Extension(app_state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Getting wallet balances");
    
    // Get the wallets (for now, just use the first one if any)
    let wallets = app_state.wallets.lock().unwrap();
    
    if wallets.is_empty() {
        return utils::build_error_response(
            StatusCode::BAD_REQUEST,
            "No wallet imported"
        );
    }
    
    // Use the first wallet
    let wallet = wallets.values().next().unwrap();
    
    // Get balances
    match wallet::get_token_balances(wallet).await {
        Ok(balances) => utils::build_success_response(balances),
        Err(err) => {
            error!("Failed to get balances: {}", err);
            utils::build_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get balances: {}", err)
            )
        }
    }
}

// Handler for getting token prices
pub async fn get_prices(
    Extension(app_state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Getting token prices");
    
    // Update prices first
    if let Err(err) = price::update_prices(app_state.clone()).await {
        error!("Failed to update prices: {}", err);
        return utils::build_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
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
                "symbol": wallet::KnownTokens::get_symbol(mint),
                "price_usd": price,
                "last_updated": chrono::Utc::now().to_rfc3339()
            })
        })
        .collect::<Vec<_>>();
    
    utils::build_success_response(prices)
}

// Handler for swapping tokens
pub async fn swap_token(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<SwapRequest>,
) -> impl IntoResponse {
    info!(
        "Swapping {} of {} to {}",
        request.amount, request.source_token, request.target_token
    );
    
    // Validate the request
    if let Err(err) = utils::validate_amount(request.amount) {
        return utils::build_error_response(
            StatusCode::BAD_REQUEST,
            &err.to_string()
        );
    }
    
    // Get the wallet
    let wallets = app_state.wallets.lock().unwrap();
    
    if wallets.is_empty() {
        return utils::build_error_response(
            StatusCode::BAD_REQUEST,
            "No wallet imported"
        );
    }
    
    // Use the first wallet
    let wallet = wallets.values().next().unwrap();
    
    // Check if the wallet has sufficient balance
    match wallet::has_sufficient_balance(wallet, &request.source_token, request.amount).await {
        Ok(has_balance) => {
            if !has_balance {
                return utils::build_error_response(
                    StatusCode::BAD_REQUEST,
                    &format!(
                        "Insufficient balance of {} to execute swap", 
                        wallet::KnownTokens::get_symbol(&request.source_token)
                    )
                );
            }
        },
        Err(err) => {
            error!("Failed to check balance: {}", err);
            return utils::build_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to check balance: {}", err)
            );
        }
    }
    
    // Execute the swap
    match swap::execute_swap(wallet, &request).await {
        Ok(result) => utils::build_success_response(result),
        Err(err) => {
            error!("Failed to execute swap: {}", err);
            utils::build_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to execute swap: {}", err)
            )
        }
    }
}

// Handler for setting a limit order
pub async fn set_limit_order(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<LimitOrderRequest>,
) -> impl IntoResponse {
    info!("Creating limit order: {:?}", request);
    
    if request.price_target <= 0.0 {
        return utils::build_error_response(
            StatusCode::BAD_REQUEST,
            "Price target must be greater than zero"
        );
    }
    
    match orders::create_limit_order(app_state, request).await {
        Ok(order) => utils::build_success_response(order),
        Err(err) => {
            error!("Failed to create limit order: {}", err);
            utils::build_error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to create limit order: {}", err)
            )
        }
    }
}

// Handler for listing limit orders
pub async fn list_limit_orders(
    Extension(app_state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Listing limit orders");
    
    let orders = orders::get_limit_orders(app_state);
    utils::build_success_response(orders)
}

// Handler for canceling a limit order
pub async fn cancel_limit_order(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<CancelOrderRequest>,
) -> impl IntoResponse {
    info!("Canceling limit order: {}", request.order_id);
    
    match orders::cancel_limit_order(app_state, &request.order_id) {
        Ok(order) => utils::build_success_response(order),
        Err(err) => {
            error!("Failed to cancel order: {}", err);
            utils::build_error_response(
                StatusCode::BAD_REQUEST,
                &err.to_string()
            )
        }
    }
} 
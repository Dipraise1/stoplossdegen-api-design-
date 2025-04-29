use crate::models::{AppState, LimitOrder, LimitOrderRequest, OrderStatus, OrderType, SwapRequest};
use crate::price;
use crate::swap;
use anyhow::{anyhow, Result};
use chrono::Utc;
use std::sync::Arc;
use tokio::time;
use tracing::{error, info};
use uuid::Uuid;
use rand;

// Create a new limit order
pub async fn create_limit_order(
    app_state: Arc<AppState>,
    order_request: LimitOrderRequest,
) -> Result<LimitOrder> {
    let now = Utc::now();
    let id = Uuid::new_v4().to_string();
    
    // Validate wallet has enough tokens for the swap
    let wallets = app_state.wallets.lock().unwrap();
    if wallets.is_empty() {
        return Err(anyhow!("No wallets found to execute order"));
    }
    
    // Just use the first wallet for now
    // In a real app, this would be tied to the user who created the order
    let wallet = wallets.values().next().unwrap();
    
    // Estimate transaction fees
    let estimated_fee = crate::wallet::estimate_transaction_fees().await
        .unwrap_or(0.01); // Default to 0.01 SOL if estimation fails
    
    info!("Estimated transaction fee for limit order: {} SOL", estimated_fee);
    
    // Check token balance based on order type
    if order_request.order_type == OrderType::Sell || order_request.order_type == OrderType::StopLoss {
        // For sell and stop loss orders, check if the wallet has enough of the source token
        let has_balance = crate::wallet::has_sufficient_balance(
            wallet, 
            &order_request.source_token, 
            order_request.amount
        ).await?;
        
        if !has_balance {
            let order_type_str = if order_request.order_type == OrderType::Sell { "sell" } else { "stop loss" };
            return Err(anyhow!("Insufficient balance to create {} order. Please add funds.", order_type_str));
        }
        
        // For stop loss orders, validate that the price target makes sense
        if order_request.order_type == OrderType::StopLoss {
            // Get current price of the target token
            let current_price = price::get_token_price(&app_state, &order_request.target_token)
                .map_err(|e| anyhow!("Failed to get price for target token: {}", e))?;
            
            // For stop loss, the price target should be below the current price
            if order_request.price_target >= current_price {
                return Err(anyhow!(
                    "Invalid stop loss price: {} is not below the current price {}. Stop loss should be set below current price.",
                    order_request.price_target,
                    current_price
                ));
            }
            
            info!(
                "Creating stop loss order with target price {} (current price: {})",
                order_request.price_target, current_price
            );
        }
    } else {
        // For buy orders, we need to calculate the estimated cost in the source token
        // Get current price of the target token
        let target_price = price::get_token_price(&app_state, &order_request.target_token)
            .map_err(|e| anyhow!("Failed to get price for target token: {}", e))?;
        
        // Get current price of the source token
        let source_price = price::get_token_price(&app_state, &order_request.source_token)
            .map_err(|e| anyhow!("Failed to get price for source token: {}", e))?;
        
        // Calculate estimated amount needed in source token
        let price_ratio = if source_price > 0.0 { target_price / source_price } else { 0.0 };
        let estimated_source_amount = order_request.amount * price_ratio * (1.0 + order_request.slippage.unwrap_or(0.5) / 100.0);
        
        info!(
            "Buy order calculation: Target price: ${}, Source price: ${}, Price ratio: {}, Estimated source amount needed: {}",
            target_price, source_price, price_ratio, estimated_source_amount
        );
        
        // Check if the wallet has enough of the source token for the estimated cost
        let has_enough_source = crate::wallet::has_sufficient_balance(
            wallet,
            &order_request.source_token,
            estimated_source_amount
        ).await?;
        
        if !has_enough_source {
            return Err(anyhow!(
                "Insufficient balance of {} to create buy order. Estimated amount needed: {} (based on current price: ${})",
                crate::wallet::KnownTokens::get_symbol(&order_request.source_token),
                estimated_source_amount,
                source_price
            ));
        }
        
        // Also ensure they have some SOL for transaction fees
        let has_sol = crate::wallet::has_sufficient_balance(
            wallet,
            "So11111111111111111111111111111111111111112",
            estimated_fee
        ).await?;
        
        if !has_sol {
            return Err(anyhow!("Insufficient SOL balance for transaction fees. Need at least {} SOL.", estimated_fee));
        }
    }
    
    let limit_order = LimitOrder {
        id: id.clone(),
        source_token: order_request.source_token,
        target_token: order_request.target_token,
        amount: order_request.amount,
        price_target: order_request.price_target,
        order_type: order_request.order_type,
        status: OrderStatus::Active,
        created_at: now,
        updated_at: now,
        expiry_time: order_request.expiry_time,
        slippage: order_request.slippage.unwrap_or(0.5),
        transaction_signature: None,
    };
    
    info!("Creating new {:?} limit order {} to swap {} {} for {} at price {}",
           limit_order.order_type,
           limit_order.id,
           limit_order.amount,
           crate::wallet::KnownTokens::get_symbol(&limit_order.source_token),
           crate::wallet::KnownTokens::get_symbol(&limit_order.target_token),
           limit_order.price_target);
    
    // Add the order to app state
    let mut orders = app_state.limit_orders.lock().unwrap();
    orders.insert(id, limit_order.clone());
    
    Ok(limit_order)
}

// Get all limit orders
pub fn get_limit_orders(app_state: Arc<AppState>) -> Vec<LimitOrder> {
    let orders = app_state.limit_orders.lock().unwrap();
    orders.values().cloned().collect()
}

// Cancel a limit order
pub fn cancel_limit_order(app_state: Arc<AppState>, order_id: &str) -> Result<LimitOrder> {
    let mut orders = app_state.limit_orders.lock().unwrap();
    
    if let Some(mut order) = orders.get(order_id).cloned() {
        // Only cancel active orders
        if order.status == OrderStatus::Active {
            order.status = OrderStatus::Cancelled;
            order.updated_at = Utc::now();
            orders.insert(order_id.to_string(), order.clone());
            
            info!("Cancelled limit order {}", order_id);
            Ok(order)
        } else {
            Err(anyhow!("Cannot cancel an order that is not active (current status: {:?})", order.status))
        }
    } else {
        Err(anyhow!("Order not found: {}", order_id))
    }
}

// Check if an order should be executed
fn should_execute_order(order: &LimitOrder, current_price: f64) -> bool {
    match order.order_type {
        OrderType::Buy => {
            // Buy when the price is below or equal to the target price
            current_price <= order.price_target
        }
        OrderType::Sell => {
            // Sell when the price is above or equal to the target price
            current_price >= order.price_target
        }
        OrderType::StopLoss => {
            // Stop loss triggers when the price drops to or below the target price
            current_price <= order.price_target
        }
    }
}

// Execute a limit order
async fn execute_order(app_state: Arc<AppState>, order: LimitOrder) -> Result<LimitOrder> {
    // Get the wallet
    let wallets = app_state.wallets.lock().unwrap();
    if wallets.is_empty() {
        return Err(anyhow!("No wallets found to execute order"));
    }
    
    // Just use the first wallet for now
    // In a real app, this would be tied to the user who created the order
    let wallet = wallets.values().next().unwrap();
    
    // Estimate transaction fees
    let estimated_fee = crate::wallet::estimate_transaction_fees().await
        .unwrap_or(0.01); // Default to 0.01 SOL if estimation fails
    
    info!("Estimated transaction fee for order execution: {} SOL", estimated_fee);
    
    // Get current prices for calculation
    let target_price = price::get_token_price(&app_state, &order.target_token)
        .map_err(|e| anyhow!("Failed to get price for target token: {}", e))?;
    
    // Double-check balance before executing based on order type
    if order.order_type == OrderType::Sell || order.order_type == OrderType::StopLoss {
        // For sell and stop loss orders, check if the wallet still has enough of the source token
        let has_balance = crate::wallet::has_sufficient_balance(
            wallet, 
            &order.source_token, 
            order.amount
        ).await?;
        
        if !has_balance {
            // Mark the order as failed due to insufficient balance
            let mut orders = app_state.limit_orders.lock().unwrap();
            if let Some(mut updated_order) = orders.get(&order.id).cloned() {
                updated_order.status = OrderStatus::Failed;
                updated_order.updated_at = Utc::now();
                orders.insert(order.id.clone(), updated_order.clone());
                
                let order_type_str = if order.order_type == OrderType::Sell { "Sell" } else { "Stop loss" };
                error!("{} order {} failed: Insufficient balance of {} to execute", 
                       order_type_str, order.id, crate::wallet::KnownTokens::get_symbol(&order.source_token));
                
                return Ok(updated_order);
            }
            return Err(anyhow!("Insufficient balance to execute sell order"));
        }
    } else {
        // For buy orders, we need to calculate the estimated cost in the source token
        // Get current price of the source token
        let source_price = price::get_token_price(&app_state, &order.source_token)
            .map_err(|e| anyhow!("Failed to get price for source token: {}", e))?;
        
        // Calculate estimated amount needed in source token using current prices
        let price_ratio = if source_price > 0.0 { target_price / source_price } else { 0.0 };
        let estimated_source_amount = order.amount * price_ratio * (1.0 + order.slippage / 100.0);
        
        info!(
            "Buy order execution calculation: Target price: ${}, Source price: ${}, Price ratio: {}, Estimated source amount needed: {}",
            target_price, source_price, price_ratio, estimated_source_amount
        );
        
        // Check if the wallet has enough of the source token for the estimated cost
        let has_enough_source = crate::wallet::has_sufficient_balance(
            wallet,
            &order.source_token,
            estimated_source_amount
        ).await?;
        
        if !has_enough_source {
            // Mark the order as failed due to insufficient balance
            let mut orders = app_state.limit_orders.lock().unwrap();
            if let Some(mut updated_order) = orders.get(&order.id).cloned() {
                updated_order.status = OrderStatus::Failed;
                updated_order.updated_at = Utc::now();
                orders.insert(order.id.clone(), updated_order.clone());
                
                let order_type_str = if order.order_type == OrderType::Buy { "Buy" } else { "Stop loss" };
                error!(
                    "{} order {} failed: Insufficient balance of {} to execute. Needed: {}, Current price: ${}",
                    order_type_str, order.id, 
                    crate::wallet::KnownTokens::get_symbol(&order.source_token),
                    estimated_source_amount,
                    source_price
                );
                
                return Ok(updated_order);
            }
            return Err(anyhow!("Insufficient balance to execute buy order"));
        }
        
        // Also ensure they have some SOL for transaction fees
        let has_sol = crate::wallet::has_sufficient_balance(
            wallet,
            "So11111111111111111111111111111111111111112",
            estimated_fee
        ).await?;
        
        if !has_sol {
            // Mark the order as failed due to insufficient SOL
            let mut orders = app_state.limit_orders.lock().unwrap();
            if let Some(mut updated_order) = orders.get(&order.id).cloned() {
                updated_order.status = OrderStatus::Failed;
                updated_order.updated_at = Utc::now();
                orders.insert(order.id.clone(), updated_order.clone());
                
                error!("Order {} failed: Insufficient SOL for transaction fees. Need at least {} SOL", 
                       order.id, estimated_fee);
                
                return Ok(updated_order);
            }
            return Err(anyhow!("Insufficient SOL for transaction fees"));
        }
    }
    
    // Create swap request
    let swap_request = SwapRequest {
        source_token: order.source_token.clone(),
        target_token: order.target_token.clone(),
        amount: order.amount,
        slippage: Some(order.slippage),
    };
    
    info!("Executing limit order {} - {:?} order for {} {} at price target {}",
           order.id,
           order.order_type,
           order.amount,
           crate::wallet::KnownTokens::get_symbol(&order.source_token),
           order.price_target);
    
    // Execute swap
    match swap::execute_swap(wallet, &swap_request).await {
        Ok(swap_result) => {
            // Update order
            let mut orders = app_state.limit_orders.lock().unwrap();
            if let Some(mut updated_order) = orders.get(&order.id).cloned() {
                updated_order.status = OrderStatus::Completed;
                updated_order.updated_at = Utc::now();
                updated_order.transaction_signature = Some(swap_result.transaction_signature.clone());
                
                orders.insert(order.id.clone(), updated_order.clone());
                
                info!(
                    "Successfully executed limit order {}: {} -> {} for {} at price {}. Signature: {}",
                    order.id, 
                    crate::wallet::KnownTokens::get_symbol(&order.source_token), 
                    crate::wallet::KnownTokens::get_symbol(&order.target_token), 
                    order.amount, 
                    order.price_target,
                    swap_result.transaction_signature
                );
                
                Ok(updated_order)
            } else {
                Err(anyhow!("Order not found after execution: {}", order.id))
            }
        }
        Err(err) => {
            error!("Failed to execute order {}: {}", order.id, err);
            
            // Mark order as failed
            let mut orders = app_state.limit_orders.lock().unwrap();
            if let Some(mut updated_order) = orders.get(&order.id).cloned() {
                updated_order.status = OrderStatus::Failed;
                updated_order.updated_at = Utc::now();
                
                orders.insert(order.id.clone(), updated_order.clone());
                
                Ok(updated_order)
            } else {
                Err(anyhow!("Order not found after failed execution: {}", order.id))
            }
        }
    }
}

// Background task to monitor limit orders
pub async fn monitor_limit_orders(app_state: Arc<AppState>) {
    info!("Starting limit order monitor task");
    
    // Wait a bit on startup to make sure everything is initialized
    time::sleep(time::Duration::from_secs(5)).await;
    
    loop {
        // Sleep for a few seconds to avoid hammering the APIs
        time::sleep(time::Duration::from_secs(30)).await;
        
        // Skip if no wallets are available
        {
            let wallets = app_state.wallets.lock().unwrap();
            if wallets.is_empty() {
                continue;
            }
        }
        
        // Update token prices
        if let Err(err) = price::update_prices(app_state.clone()).await {
            error!("Failed to update prices: {}", err);
            continue;
        }
        
        // Get active orders
        let orders = {
            let orders_lock = app_state.limit_orders.lock().unwrap();
            orders_lock
                .values()
                .filter(|order| order.status == OrderStatus::Active)
                .cloned()
                .collect::<Vec<_>>()
        };
        
        if !orders.is_empty() {
            info!("Checking {} active limit orders", orders.len());
        }
        
        for order in orders {
            // Check if the order has expired
            if let Some(expiry_time) = order.expiry_time {
                if Utc::now() > expiry_time {
                    info!("Order {} has expired, cancelling", order.id);
                    if let Err(err) = cancel_limit_order(app_state.clone(), &order.id) {
                        error!("Failed to cancel expired order {}: {}", order.id, err);
                    }
                    continue;
                }
            }
            
            // Get the current price of the target token
            match price::get_token_price(&app_state, &order.target_token) {
                Ok(current_price) => {
                    let should_execute = should_execute_order(&order, current_price);
                    
                    // Add debug logging based on order type
                    match order.order_type {
                        OrderType::Buy => {
                            if current_price <= order.price_target {
                                info!("Buy order {} triggered - current price {} <= target {}", 
                                       order.id, current_price, order.price_target);
                            } else {
                                // Only log occasionally to avoid spamming the logs
                                if rand::random::<u8>() < 5 { // ~2% chance
                                    info!("Buy order {} waiting - current price {} > target {}", 
                                          order.id, current_price, order.price_target);
                                }
                            }
                        }
                        OrderType::Sell => {
                            if current_price >= order.price_target {
                                info!("Sell order {} triggered - current price {} >= target {}", 
                                       order.id, current_price, order.price_target);
                            } else {
                                // Only log occasionally to avoid spamming the logs
                                if rand::random::<u8>() < 5 { // ~2% chance
                                    info!("Sell order {} waiting - current price {} < target {}", 
                                          order.id, current_price, order.price_target);
                                }
                            }
                        }
                        OrderType::StopLoss => {
                            if current_price <= order.price_target {
                                info!("Stop loss order {} triggered - current price {} <= target {}", 
                                       order.id, current_price, order.price_target);
                            } else {
                                // Only log occasionally to avoid spamming the logs
                                if rand::random::<u8>() < 5 { // ~2% chance
                                    info!("Stop loss order {} waiting - current price {} > target {}", 
                                          order.id, current_price, order.price_target);
                                }
                            }
                        }
                    }
                    
                    if should_execute {
                        // Clone the order before moving it to execute_order
                        let order_to_execute = order.clone();
                        
                        // Execute the order
                        if let Err(err) = execute_order(app_state.clone(), order_to_execute).await {
                            error!("Failed to execute order {}: {}", order.id, err);
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to get price for token {}: {}", order.target_token, err);
                }
            }
        }
    }
}

// Public version of should_execute_order for testing purposes
pub fn should_execute_order_test(order: &LimitOrder, current_price: f64) -> bool {
    should_execute_order(order, current_price)
} 
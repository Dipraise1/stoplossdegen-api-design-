use crate::models::{AppState, LimitOrder, LimitOrderRequest, OrderStatus, OrderType, SwapRequest};
use crate::price;
use crate::swap;
use anyhow::{anyhow, Result};
use chrono::Utc;
use std::sync::Arc;
use tokio::time;
use tracing::{error, info, warn};
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
    
    // Check token balance if this is a sell order
    if order_request.order_type == OrderType::Sell {
        let has_balance = crate::wallet::has_sufficient_balance(
            wallet, 
            &order_request.source_token, 
            order_request.amount
        ).await?;
        
        if !has_balance {
            return Err(anyhow!("Insufficient balance to create sell order. Please add funds."));
        }
    } else {
        // For buy orders, we need to make sure they have some SOL for transaction fees
        let has_sol = crate::wallet::has_sufficient_balance(
            wallet,
            "So11111111111111111111111111111111111111112",
            0.01 // Minimum SOL needed for transaction fees
        ).await?;
        
        if !has_sol {
            return Err(anyhow!("Insufficient SOL balance for transaction fees."));
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
    
    // Double-check balance before executing
    if order.order_type == OrderType::Sell {
        // For sell orders, check if the wallet still has enough of the source token
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
                
                error!("Order {} failed: Insufficient balance of {} to execute", 
                       order.id, crate::wallet::KnownTokens::get_symbol(&order.source_token));
                
                return Ok(updated_order);
            }
            return Err(anyhow!("Insufficient balance to execute sell order"));
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
                    
                    // Add debug logging
                    if order.order_type == OrderType::Buy {
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
                    } else {
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
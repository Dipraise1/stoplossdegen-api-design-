use crate::models::{AppState, LimitOrder, LimitOrderRequest, OrderStatus, OrderType};
use crate::orders;
use crate::price;
use chrono::Utc;
use std::sync::Arc;
use anyhow::Result;

// A function to override the wallet balance check for testing
fn mock_balance_check(_token_mint: &str, _amount: f64) -> bool {
    // Always return true for testing
    true
}

/// Test function to demonstrate stop loss functionality
pub async fn test_stop_loss() -> Result<()> {
    println!("Beginning stop loss testing...");
    
    // Initialize app state
    let app_state = Arc::new(AppState::new());
    
    // Generate a test wallet
    let (wallet, _) = crate::wallet::generate_new_wallet()?;
    let wallet_pubkey = wallet.pubkey.to_string();
    
    println!("Generated test wallet: {}", wallet_pubkey);
    
    // Add the wallet to the app state
    {
        let mut wallets = app_state.wallets.lock().unwrap();
        wallets.insert(wallet_pubkey.clone(), wallet);
    }
    
    // Set up some token prices for testing
    {
        let mut prices = app_state.token_prices.lock().unwrap();
        prices.insert("So11111111111111111111111111111111111111112".to_string(), 20.0); // SOL
        prices.insert("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), 1.0); // USDC
        prices.insert("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(), 0.00005); // BONK
    }
    
    // Create a stop loss order
    let stop_loss_request = LimitOrderRequest {
        source_token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
        target_token: "So11111111111111111111111111111111111111112".to_string(), // SOL
        amount: 50.0,
        price_target: 15.0, // Stop loss at $15 (below current SOL price of $20)
        order_type: OrderType::StopLoss,
        expiry_time: None,
        slippage: Some(1.0),
    };
    
    println!("Creating stop loss order: Sell 50 USDC if SOL price drops to $15");
    println!("Current SOL price is $20");
    
    // Temporarily modify the wallet has_sufficient_balance function for testing
    // This is a simplified approach for testing - in a real environment, we'd use
    // proper mocking frameworks or dependency injection
    
    // Simulate creating a limit order with our mocked balance check
    let order_result = create_test_order(app_state.clone(), stop_loss_request).await;
    
    match order_result {
        Ok(order) => {
            println!("Stop loss order created successfully:");
            println!("  Order ID: {}", order.id);
            println!("  Type: {:?}", order.order_type);
            println!("  Trigger Price: {}", order.price_target);
            println!("  Order Status: {:?}", order.status);
            
            // Check if the order would execute at different prices
            println!("\nTesting order execution logic:");
            
            let sol_price = 20.0;
            println!("Current SOL price: ${} - Should execute? {}", 
                     sol_price, 
                     orders::should_execute_order_test(&order, sol_price));
            
            let sol_price = 15.0;
            println!("SOL price drops to ${} - Should execute? {}", 
                     sol_price, 
                     orders::should_execute_order_test(&order, sol_price));
            
            let sol_price = 14.5;
            println!("SOL price drops to ${} - Should execute? {}", 
                     sol_price, 
                     orders::should_execute_order_test(&order, sol_price));
            
            println!("\nSimulating price drops to trigger order:");
            
            // Now let's simulate the price dropping to trigger the stop loss
            {
                let mut prices = app_state.token_prices.lock().unwrap();
                prices.insert("So11111111111111111111111111111111111111112".to_string(), 14.5); // SOL price drops
                println!("Updated SOL price to $14.5 (below stop loss threshold of $15)");
            }
            
            // Get the order by ID for monitoring
            let order_id = order.id.clone();
            
            // Check if order would execute
            let orders_map = app_state.limit_orders.lock().unwrap();
            if let Some(updated_order) = orders_map.get(&order_id) {
                let current_price = price::get_token_price(&app_state, &updated_order.target_token)?;
                println!("Current price: ${}, Stop loss trigger: ${}", current_price, updated_order.price_target);
                
                let should_execute = orders::should_execute_order_test(updated_order, current_price);
                println!("Order should execute: {}", should_execute);
            }
        }
        Err(err) => {
            println!("Error creating stop loss order: {}", err);
        }
    }
    
    println!("\nStop loss test completed!");
    Ok(())
}

// A modified version of create_limit_order that bypasses balance checks for testing
async fn create_test_order(app_state: Arc<AppState>, order_request: LimitOrderRequest) -> Result<crate::models::LimitOrder> {
    // This is a simplified version of the create_limit_order function that bypasses balance checks
    println!("Note: Balance checks are bypassed for testing purposes");
    
    use chrono::Utc;
    use uuid::Uuid;
    use crate::models::{LimitOrder, OrderStatus};
    
    let now = Utc::now();
    let id = Uuid::new_v4().to_string();
    
    // Create the limit order without balance checks
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
    
    // Add the order to app state
    let mut orders = app_state.limit_orders.lock().unwrap();
    orders.insert(id, limit_order.clone());
    
    Ok(limit_order)
}

// Simulate the full execution of a stop loss order
pub async fn test_stop_loss_execution() -> Result<()> {
    println!("Beginning stop loss execution simulation...");
    
    // Initialize app state
    let app_state = Arc::new(AppState::new());
    
    // Generate a test wallet
    let (wallet, _) = crate::wallet::generate_new_wallet()?;
    let wallet_pubkey = wallet.pubkey.to_string();
    
    println!("Generated test wallet: {}", wallet_pubkey);
    
    // Add the wallet to the app state
    {
        let mut wallets = app_state.wallets.lock().unwrap();
        wallets.insert(wallet_pubkey.clone(), wallet);
    }
    
    // Set up initial token prices for testing
    {
        let mut prices = app_state.token_prices.lock().unwrap();
        prices.insert("So11111111111111111111111111111111111111112".to_string(), 20.0); // SOL
        prices.insert("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), 1.0); // USDC
        prices.insert("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(), 0.00005); // BONK
    }
    
    // Create a stop loss order
    let stop_loss_request = LimitOrderRequest {
        source_token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
        target_token: "So11111111111111111111111111111111111111112".to_string(), // SOL
        amount: 50.0,
        price_target: 15.0, // Stop loss at $15 (below current SOL price of $20)
        order_type: OrderType::StopLoss,
        expiry_time: None,
        slippage: Some(1.0),
    };
    
    println!("Creating stop loss order: Sell 50 USDC if SOL price drops to $15");
    println!("Current SOL price is $20");
    
    // Create the order without balance checks
    let order = create_test_order(app_state.clone(), stop_loss_request).await?;
    
    println!("Stop loss order created with ID: {}", order.id);
    
    // Simulate the monitor_limit_orders function
    println!("\nSimulating price monitoring and execution...");
    
    // First check at current price (should not execute)
    {
        let current_price = price::get_token_price(&app_state, &order.target_token)?;
        println!("Time t=0: SOL price is ${} (stop loss at ${})", current_price, order.price_target);
        let should_execute = orders::should_execute_order_test(&order, current_price);
        println!("Should execute? {} (expected: false)", should_execute);
        assert!(!should_execute, "Order should not execute at price above stop loss");
    }
    
    // Simulate price staying above stop loss
    println!("\nTime t=1: Price drops slightly but remains above stop loss");
    {
        let mut prices = app_state.token_prices.lock().unwrap();
        prices.insert("So11111111111111111111111111111111111111112".to_string(), 17.0);
    }
    
    {
        let current_price = price::get_token_price(&app_state, &order.target_token)?;
        println!("SOL price is now ${} (stop loss at ${})", current_price, order.price_target);
        let should_execute = orders::should_execute_order_test(&order, current_price);
        println!("Should execute? {} (expected: false)", should_execute);
        assert!(!should_execute, "Order should not execute at price above stop loss");
    }
    
    // Simulate price dropping to stop loss level
    println!("\nTime t=2: Price drops to exactly the stop loss level");
    {
        let mut prices = app_state.token_prices.lock().unwrap();
        prices.insert("So11111111111111111111111111111111111111112".to_string(), 15.0);
    }
    
    {
        let current_price = price::get_token_price(&app_state, &order.target_token)?;
        println!("SOL price is now ${} (stop loss at ${})", current_price, order.price_target);
        let should_execute = orders::should_execute_order_test(&order, current_price);
        println!("Should execute? {} (expected: true)", should_execute);
        assert!(should_execute, "Order should execute at price equal to stop loss");
    }
    
    // Simulate price dropping below stop loss level
    println!("\nTime t=3: Price drops further below the stop loss level");
    {
        let mut prices = app_state.token_prices.lock().unwrap();
        prices.insert("So11111111111111111111111111111111111111112".to_string(), 14.0);
    }
    
    {
        let current_price = price::get_token_price(&app_state, &order.target_token)?;
        println!("SOL price is now ${} (stop loss at ${})", current_price, order.price_target);
        let should_execute = orders::should_execute_order_test(&order, current_price);
        println!("Should execute? {} (expected: true)", should_execute);
        assert!(should_execute, "Order should execute at price below stop loss");
    }
    
    // Simulate order execution
    println!("\nSimulating order execution...");
    
    // In a full implementation, we would call execute_order here
    // For testing purposes, we'll just update the order status
    {
        let mut orders = app_state.limit_orders.lock().unwrap();
        if let Some(mut updated_order) = orders.get(&order.id).cloned() {
            updated_order.status = OrderStatus::Completed;
            updated_order.updated_at = chrono::Utc::now();
            updated_order.transaction_signature = Some("SimulatedTransactionSignature123456789".to_string());
            orders.insert(order.id.clone(), updated_order.clone());
            
            println!("Order executed successfully!");
            println!("Order status: {:?}", updated_order.status);
            println!("Transaction signature: {}", updated_order.transaction_signature.unwrap());
        }
    }
    
    println!("\nStop loss execution simulation completed successfully!");
    Ok(())
} 
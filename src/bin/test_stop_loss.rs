use solana_wallet_api::test_stop_loss;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("===== Testing Stop Loss Functionality =====");
    println!("This test will verify that stop loss orders trigger correctly when prices fall below the threshold.");
    println!("===========================================\n");
    
    // Run basic stop loss test
    println!("TEST 1: Basic stop loss trigger test");
    test_stop_loss::test_stop_loss().await?;
    
    println!("\n-------------------------------------------\n");
    
    // Run full execution simulation test
    println!("TEST 2: Full stop loss execution simulation");
    test_stop_loss::test_stop_loss_execution().await?;
    
    println!("\n===========================================");
    println!("All tests completed successfully!");
    Ok(())
} 
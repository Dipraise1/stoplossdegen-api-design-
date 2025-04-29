use anyhow::{anyhow, Result};
use axum::Json;
use serde_json::json;

// Convert lamports to SOL
pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

// Convert SOL to lamports
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}

// Convert token amount to UI amount based on decimals
pub fn token_amount_to_ui_amount(amount: u64, decimals: u8) -> f64 {
    amount as f64 / 10f64.powi(decimals as i32)
}

// Convert UI amount to token amount based on decimals
pub fn ui_amount_to_token_amount(ui_amount: f64, decimals: u8) -> u64 {
    (ui_amount * 10f64.powi(decimals as i32)) as u64
}

// Helper to build a consistent API response
pub fn build_api_response<T: serde::Serialize>(
    data: Option<T>,
    error: Option<String>,
) -> Json<serde_json::Value> {
    let success = error.is_none();
    
    let response = json!({
        "success": success,
        "data": data,
        "error": error,
    });
    
    Json(response)
}

// Helper to build error responses
pub fn build_error_response(error: &str) -> Json<serde_json::Value> {
    build_api_response::<()>(None, Some(error.to_string()))
}

// Helper to build success responses
pub fn build_success_response<T: serde::Serialize>(data: T) -> Json<serde_json::Value> {
    build_api_response(Some(data), None)
}

// Validate amount is positive
pub fn validate_amount(amount: f64) -> Result<()> {
    if amount <= 0.0 {
        return Err(anyhow!("Amount must be greater than zero"));
    }
    Ok(())
} 
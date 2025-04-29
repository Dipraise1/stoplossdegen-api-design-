use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
};
use std::{
    collections::HashMap,
    fmt,
    sync::Mutex,
};

// Main application state
pub struct AppState {
    pub wallets: Mutex<HashMap<String, Wallet>>,
    pub limit_orders: Mutex<HashMap<String, LimitOrder>>,
    pub token_prices: Mutex<HashMap<String, f64>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            wallets: Mutex::new(HashMap::new()),
            limit_orders: Mutex::new(HashMap::new()),
            token_prices: Mutex::new(HashMap::new()),
        }
    }
}

// Wallet structure (private key never exposed)
pub struct Wallet {
    pub keypair: Keypair,
    pub pubkey: Pubkey,
}

// Token Balance for the API response
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenBalance {
    pub mint: String,
    pub symbol: String,
    pub amount: f64,
    pub decimals: u8,
    pub ui_amount: f64,
}

// Token Price for the API response
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenPrice {
    pub mint: String,
    pub symbol: String,
    pub price_usd: f64,
    pub last_updated: DateTime<Utc>,
}

// Swap request
#[derive(Deserialize, Debug)]
pub struct SwapRequest {
    pub source_token: String,
    pub target_token: String,
    pub amount: f64,
    pub slippage: Option<f64>,
}

// Swap response
#[derive(Serialize, Debug)]
pub struct SwapResponse {
    pub transaction_signature: String,
    pub source_amount: f64,
    pub target_amount: f64,
    pub fee: f64,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
}

// Order types
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum OrderType {
    Buy,
    Sell,
}

// Add Display implementation for OrderType
impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderType::Buy => write!(f, "Buy"),
            OrderType::Sell => write!(f, "Sell"),
        }
    }
}

// Order status
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum OrderStatus {
    Active,
    Completed,
    Cancelled,
    Failed,
}

// Limit order request
#[derive(Deserialize, Debug)]
pub struct LimitOrderRequest {
    pub source_token: String,
    pub target_token: String,
    pub amount: f64,
    pub price_target: f64,
    pub order_type: OrderType,
    pub expiry_time: Option<DateTime<Utc>>,
    pub slippage: Option<f64>,
}

// Limit order response
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LimitOrder {
    pub id: String,
    pub source_token: String,
    pub target_token: String,
    pub amount: f64,
    pub price_target: f64,
    pub order_type: OrderType,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expiry_time: Option<DateTime<Utc>>,
    pub slippage: f64,
    pub transaction_signature: Option<String>,
}

// Import wallet request
#[derive(Deserialize, Debug)]
pub struct ImportWalletRequest {
    pub private_key: Option<String>,
    pub mnemonic: Option<String>,
}

// Response for wallet creation
#[derive(Serialize)]
pub struct CreateWalletResponse {
    pub pubkey: String,
    pub mnemonic: String,
}

// API responses
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

// Cancel limit order request
#[derive(Deserialize, Debug)]
pub struct CancelOrderRequest {
    pub order_id: String,
} 
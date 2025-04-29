use crate::models::TokenPrice;
use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

// Jupiter API URLs for price data
const JUPITER_PRICE_API_URL: &str = "https://price.jup.ag/v4/price";

// CoinGecko API for fallback
const COINGECKO_API_URL: &str = "https://api.coingecko.com/api/v3/simple/price";

// Jupiter price response structures
#[derive(Deserialize, Debug)]
struct JupiterPriceResponse {
    data: HashMap<String, JupiterTokenData>,
}

#[derive(Deserialize, Debug)]
struct JupiterTokenData {
    id: String,
    mint: String,
    price: f64,
    #[serde(rename = "timeToPriceUpdated")]
    time_to_price_updated: u64,
}

// CoinGecko price response structure
#[derive(Deserialize, Debug)]
struct CoinGeckoPriceResponse {
    #[serde(flatten)]
    prices: HashMap<String, CoinGeckoTokenData>,
}

#[derive(Deserialize, Debug)]
struct CoinGeckoTokenData {
    usd: f64,
}

// Token mapping for CoinGecko IDs
fn get_coingecko_id(symbol: &str) -> Option<&'static str> {
    match symbol.to_uppercase().as_str() {
        "SOL" => Some("solana"),
        "USDC" => Some("usd-coin"),
        "BONK" => Some("bonk"),
        "GMT" => Some("stepn"),
        _ => None,
    }
}

// Get prices from Jupiter Aggregator API
pub async fn get_prices_from_jupiter(tokens: &[String]) -> Result<Vec<TokenPrice>> {
    let client = Client::new();
    let mut token_list = tokens.join(",");
    
    // Always include SOL
    if !token_list.contains("So11111111111111111111111111111111111111112") {
        if !token_list.is_empty() {
            token_list.push_str(",");
        }
        token_list.push_str("So11111111111111111111111111111111111111112");
    }
    
    let url = format!("{}?ids={}", JUPITER_PRICE_API_URL, token_list);
    
    let response = client
        .get(&url)
        .send()
        .await?
        .json::<JupiterPriceResponse>()
        .await?;
    
    let mut prices = Vec::new();
    
    for (_, token_data) in response.data {
        prices.push(TokenPrice {
            mint: token_data.mint.clone(),
            symbol: crate::wallet::KnownTokens::get_symbol(&token_data.mint),
            price_usd: token_data.price,
            last_updated: Utc::now(),
        });
    }
    
    Ok(prices)
}

// Get prices from CoinGecko API (fallback)
pub async fn get_prices_from_coingecko(symbols: &[String]) -> Result<Vec<TokenPrice>> {
    let client = Client::new();
    
    // Convert symbols to CoinGecko IDs
    let mut ids = Vec::new();
    
    for symbol in symbols {
        if let Some(id) = get_coingecko_id(symbol) {
            ids.push(id);
        }
    }
    
    if ids.is_empty() {
        return Err(anyhow!("No recognized tokens for CoinGecko API"));
    }
    
    let ids_str = ids.join(",");
    let url = format!("{}?ids={}&vs_currencies=usd", COINGECKO_API_URL, ids_str);
    
    let response = client
        .get(&url)
        .send()
        .await?
        .json::<CoinGeckoPriceResponse>()
        .await?;
    
    let mut prices = Vec::new();
    
    for (id, data) in response.prices {
        prices.push(TokenPrice {
            // We don't have the mint address here, so we use the id
            mint: id.clone(),
            symbol: id,
            price_usd: data.usd,
            last_updated: Utc::now(),
        });
    }
    
    Ok(prices)
}

// Update prices in the app state
pub async fn update_prices(app_state: Arc<crate::models::AppState>) -> Result<()> {
    // Get list of mints from all wallets
    let tokens = {
        let wallets = app_state.wallets.lock().unwrap();
        
        if wallets.is_empty() {
            // Default to SOL if no wallets
            vec!["So11111111111111111111111111111111111111112".to_string()]
        } else {
            // Get unique tokens from all wallets
            let mut tokens = Vec::new();
            
            for (_, _) in wallets.iter() {
                // This would require async in the lock, so in a real app
                // we might use a different approach to avoid deadlocks
                // For now, just use default tokens
                tokens.push("So11111111111111111111111111111111111111112".to_string());
                tokens.push("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()); // USDC
            }
            
            tokens
        }
    };
    
    // Try Jupiter first
    match get_prices_from_jupiter(&tokens).await {
        Ok(prices) => {
            let mut price_map = app_state.token_prices.lock().unwrap();
            for price in prices {
                price_map.insert(price.mint.clone(), price.price_usd);
            }
            info!("Updated prices from Jupiter");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to get prices from Jupiter: {}", e);
            // Fall back to CoinGecko
            let symbols = vec!["SOL".to_string(), "USDC".to_string()];
            match get_prices_from_coingecko(&symbols).await {
                Ok(prices) => {
                    let mut price_map = app_state.token_prices.lock().unwrap();
                    for price in prices {
                        price_map.insert(price.mint.clone(), price.price_usd);
                    }
                    info!("Updated prices from CoinGecko");
                    return Ok(());
                }
                Err(e) => {
                    error!("Failed to get prices from CoinGecko: {}", e);
                    return Err(anyhow!("Failed to update prices from all sources"));
                }
            }
        }
    }
}

// Get current price for a specific token
pub fn get_token_price(
    app_state: &crate::models::AppState,
    token_mint: &str,
) -> Result<f64> {
    let price_map = app_state.token_prices.lock().unwrap();
    
    if let Some(price) = price_map.get(token_mint) {
        Ok(*price)
    } else {
        Err(anyhow!("Price not found for token {}", token_mint))
    }
} 
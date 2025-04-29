use crate::models::{SwapRequest, SwapResponse, Wallet};
use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use tracing::info;
use solana_sdk::{
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};

// Jupiter API URLs
const JUPITER_QUOTE_API_URL: &str = "https://quote-api.jup.ag/v4/quote";
const JUPITER_SWAP_API_URL: &str = "https://quote-api.jup.ag/v4/swap";

// Jupiter quote response
#[derive(Deserialize, Serialize, Debug)]
pub struct JupiterQuoteResponse {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "inAmount")]
    in_amount: String,
    #[serde(rename = "outAmount")]
    out_amount: String,
    #[serde(rename = "routePlan")]
    route_plan: Vec<JupiterRoutePlan>,
    #[serde(rename = "otherAmountThreshold")]
    other_amount_threshold: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JupiterRoutePlan {
    #[serde(rename = "swapInfo")]
    swap_info: JupiterSwapInfo,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JupiterSwapInfo {
    #[serde(rename = "ammKey")]
    amm_key: String,
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    label: String,
}

// Jupiter swap request
#[derive(Serialize, Debug)]
struct JupiterSwapRequest {
    #[serde(rename = "quoteResponse")]
    quote_response: String,
    #[serde(rename = "userPublicKey")]
    user_public_key: String,
    #[serde(rename = "wrapUnwrapSOL")]
    wrap_unwrap_sol: bool,
}

// Jupiter swap response
#[derive(Deserialize, Debug)]
struct JupiterSwapResponse {
    #[serde(rename = "swapTransaction")]
    swap_transaction: String,
}

// Get a swap quote from Jupiter Aggregator
pub async fn get_swap_quote(
    source_token: &str,
    target_token: &str,
    amount: u64,
    slippage: f64,
) -> Result<JupiterQuoteResponse> {
    let client = Client::new();
    
    // Build URL
    let url = format!(
        "{}?inputMint={}&outputMint={}&amount={}&slippageBps={}",
        JUPITER_QUOTE_API_URL,
        source_token,
        target_token,
        amount,
        (slippage * 100.0) as u64
    );
    
    info!("Getting swap quote from Jupiter: {}", url);
    
    // Send request with error handling
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send request to Jupiter API: {}", e))?;
    
    // Check for HTTP errors
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unable to get error details".to_string());
        return Err(anyhow!("Jupiter API returned error status {}: {}", status, error_text));
    }
    
    // Parse the response
    let quote = response
        .json::<JupiterQuoteResponse>()
        .await
        .map_err(|e| anyhow!("Failed to parse Jupiter API response: {}", e))?;
    
    Ok(quote)
}

// Execute a swap using Jupiter Aggregator
pub async fn execute_swap(
    wallet: &Wallet,
    swap_request: &SwapRequest,
) -> Result<SwapResponse> {
    let client = Client::new();
    let rpc_client = RpcClient::new_with_commitment(
        crate::wallet::get_rpc_url(),
        CommitmentConfig::confirmed(),
    );
    
    // Estimate transaction fees
    let estimated_fee = crate::wallet::estimate_transaction_fees().await
        .unwrap_or(0.01); // Default to 0.01 SOL if estimation fails
    
    info!("Estimated transaction fee for swap: {} SOL", estimated_fee);
    
    // Check if the wallet has sufficient SOL for transaction fees
    let has_sol = crate::wallet::has_sufficient_balance(
        wallet,
        "So11111111111111111111111111111111111111112",
        estimated_fee
    ).await?;
    
    if !has_sol {
        return Err(anyhow!("Insufficient SOL balance for transaction fees. Need at least {} SOL.", estimated_fee));
    }
    
    // Check if the wallet has sufficient balance of the source token
    let has_balance = crate::wallet::has_sufficient_balance(
        wallet, 
        &swap_request.source_token,
        swap_request.amount
    ).await?;
    
    if !has_balance {
        return Err(anyhow!("Insufficient balance of {} to execute swap", 
                 crate::wallet::KnownTokens::get_symbol(&swap_request.source_token)));
    }
    
    // Convert amount based on decimals
    let source_token_decimals = crate::wallet::KnownTokens::get_decimals(&swap_request.source_token)?;
    let amount_lamports = (swap_request.amount * 10f64.powi(source_token_decimals as i32)) as u64;
    
    // Get slippage or use default
    let slippage = swap_request.slippage.unwrap_or(0.5) / 100.0; // Convert to percentage
    
    // Get quote
    let quote = get_swap_quote(
        &swap_request.source_token,
        &swap_request.target_token,
        amount_lamports,
        slippage,
    )
    .await?;
    
    info!("Got swap quote for {} {} to {}", 
          swap_request.amount, 
          crate::wallet::KnownTokens::get_symbol(&swap_request.source_token),
          crate::wallet::KnownTokens::get_symbol(&swap_request.target_token));
    
    // Serialize quote to string for the swap request
    let quote_json = serde_json::to_string(&quote)
        .map_err(|e| anyhow!("Failed to serialize quote to JSON: {}", e))?;
    
    // Build swap request
    let jupiter_swap_request = JupiterSwapRequest {
        quote_response: quote_json,
        user_public_key: wallet.pubkey.to_string(),
        wrap_unwrap_sol: true, // Auto-wrap/unwrap SOL as needed
    };
    
    // Get swap transaction
    info!("Requesting swap transaction from Jupiter");
    let swap_response = client
        .post(JUPITER_SWAP_API_URL)
        .json(&jupiter_swap_request)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to request swap transaction: {}", e))?;
    
    // Check for HTTP errors
    if !swap_response.status().is_success() {
        let status = swap_response.status();
        let error_text = swap_response.text().await.unwrap_or_else(|_| "Unable to get error details".to_string());
        return Err(anyhow!("Jupiter API returned error status {}: {}", status, error_text));
    }
    
    let jupiter_swap = swap_response
        .json::<JupiterSwapResponse>()
        .await
        .map_err(|e| anyhow!("Failed to parse swap response: {}", e))?;
    
    // Decode the transaction
    info!("Decoding and signing transaction");
    let transaction_data = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &jupiter_swap.swap_transaction
    ).map_err(|e| anyhow!("Failed to decode transaction: {}", e))?;
    
    let mut transaction: Transaction = bincode::deserialize(&transaction_data)
        .map_err(|e| anyhow!("Failed to deserialize transaction: {}", e))?;
    
    // Sign the transaction
    transaction.sign(&[&wallet.keypair], transaction.message.recent_blockhash);
    
    // Send the transaction
    info!("Sending transaction to the network");
    let signature = rpc_client
        .send_transaction(&transaction)
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;
    
    info!("Transaction sent with signature: {}", signature);
    
    // Parse amounts for response
    let source_amount = swap_request.amount;
    let target_amount = quote.out_amount.parse::<f64>()? / 10f64.powi(
        crate::wallet::KnownTokens::get_decimals(&swap_request.target_token)? as i32,
    );
    
    // Return the swap results
    Ok(SwapResponse {
        transaction_signature: signature.to_string(),
        source_amount,
        target_amount,
        fee: estimated_fee, // Include the estimated transaction fee
        success: true,
        timestamp: Utc::now(),
    })
}
use crate::models::{TokenBalance, Wallet};
use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address;
use std::time::Duration;
use tracing::{error, info};

// Constants
const SOLANA_MAINNET_URL: &str = "https://api.mainnet-beta.solana.com";
const SOLANA_DEVNET_URL: &str = "https://api.devnet.solana.com";
const SOL_DECIMALS: u8 = 9;

// Common token mint addresses for testing
pub struct KnownTokens;

impl KnownTokens {
    pub fn get_symbol(mint: &str) -> String {
        match mint {
            "So11111111111111111111111111111111111111112" => "SOL".to_string(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => "USDC".to_string(),
            "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => "USDT".to_string(),
            "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So" => "mSOL".to_string(),
            "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn" => "JitoSOL".to_string(),
            "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj" => "stSOL".to_string(),
            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => "BONK".to_string(),
            _ => {
                // If unknown, return the first 4 characters of the mint address
                format!("UNK:{}..", mint.chars().take(4).collect::<String>())
            }
        }
    }

    pub fn get_decimals(mint: &str) -> Result<i32> {
        match mint {
            "So11111111111111111111111111111111111111112" => Ok(9),  // SOL
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => Ok(6), // USDC
            "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => Ok(6), // USDT
            "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So" => Ok(9),  // mSOL
            "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn" => Ok(9), // JitoSOL
            "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj" => Ok(9), // stSOL
            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => Ok(5), // BONK
            _ => Err(anyhow!("Unknown token mint: {}", mint)),
        }
    }
}

// Helper function to get RPC URL based on environment
pub fn get_rpc_url() -> String {
    std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| SOLANA_DEVNET_URL.to_string())
}

// Generate a new wallet with a random keypair
pub fn generate_new_wallet() -> Result<(Wallet, String)> {
    // Generate a random keypair
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    
    // For the purpose of this demo, we'll create a simple mnemonic
    // In a real application, you would use proper BIP39 derivation
    let words = [
        "abandon", "ability", "able", "about", "above", "absent",
        "absorb", "abstract", "absurd", "abuse", "access", "accident",
        "account", "accuse", "achieve", "acid", "acoustic", "acquire",
        "across", "act", "action", "actor", "actress", "actual",
    ];
    
    // Generate 12 random indices
    let mut mnemonic = String::new();
    
    for i in 0..12 {
        let index = (rand::random::<u8>() as usize) % words.len();
        if i > 0 {
            mnemonic.push(' ');
        }
        mnemonic.push_str(words[index]);
    }
    
    Ok((Wallet { keypair, pubkey }, mnemonic))
}

// Import wallet from private key
pub fn import_from_private_key(private_key: &str) -> Result<Wallet> {
    let bytes = bs58::decode(private_key).into_vec()?;
    let keypair = Keypair::from_bytes(&bytes)?;
    let pubkey = keypair.pubkey();

    Ok(Wallet { keypair, pubkey })
}

// Import wallet from mnemonic (simplified for demo)
pub fn import_from_mnemonic(mnemonic_phrase: &str) -> Result<Wallet> {
    // For demo purposes, we'll generate a deterministic keypair from the mnemonic
    // In a real application, you'd use proper BIP39/44 derivation
    use sha2::{Sha256, Digest};
    
    // Create a hash of the mnemonic
    let mut hasher = Sha256::new();
    hasher.update(mnemonic_phrase.as_bytes());
    let result = hasher.finalize();
    
    // Use the hash as seed for the keypair
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&result[0..32]);
    
    // Create a keypair using the seed
    let keypair = Keypair::new();  // For demo, just create a new keypair
                                   // In production, use proper derivation from the seed
    let pubkey = keypair.pubkey();
    
    Ok(Wallet { keypair, pubkey })
}

// Get token balances for a wallet
pub async fn get_token_balances(wallet: &Wallet) -> Result<Vec<TokenBalance>> {
    let client = RpcClient::new_with_timeout(
        get_rpc_url(),
        Duration::from_secs(30),
    );
    
    let mut balances = Vec::new();
    
    // Get SOL balance first
    let sol_balance = client.get_balance(&wallet.pubkey)?;
    let sol_balance_float = sol_balance as f64 / 10f64.powi(9); // SOL has 9 decimals
    
    balances.push(TokenBalance {
        mint: "So11111111111111111111111111111111111111112".to_string(), // Native SOL mint address
        symbol: "SOL".to_string(),
        amount: sol_balance_float,
    });
    
    // Get SPL token accounts - simplified approach since the RPC methods might vary by version
    // In a production app, you would handle more token fetching details
    // For demo purposes, we'll just return the SOL balance
    // and add a few mock token balances for testing
    
    // Add some mock token balances for testing
    if rand::random::<u8>() % 2 == 0 {
        balances.push(TokenBalance {
            mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
            symbol: "USDC".to_string(),
            amount: 100.0,
        });
    }
    
    if rand::random::<u8>() % 2 == 0 {
        balances.push(TokenBalance {
            mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(), // BONK
            symbol: "BONK".to_string(),
            amount: 1000000.0,
        });
    }
    
    Ok(balances)
}

// Check if wallet has sufficient balance for a token
pub async fn has_sufficient_balance(wallet: &Wallet, token_mint: &str, amount_needed: f64) -> Result<bool> {
    let balances = get_token_balances(wallet).await?;
    
    // Get token decimals
    let decimals = match KnownTokens::get_decimals(token_mint) {
        Ok(value) => value,
        Err(_) => {
            error!("Unknown token mint: {}, assuming 9 decimals", token_mint);
            9 // Default to 9 decimals if unknown
        }
    };
    
    // Convert amount to raw units based on decimals
    let amount_raw = (amount_needed * 10f64.powi(decimals)) as u64;
    
    // Check if token exists in balances and has sufficient amount
    for balance in balances {
        if balance.mint == token_mint {
            let balance_raw = (balance.amount * 10f64.powi(decimals)) as u64;
            return Ok(balance_raw >= amount_raw);
        }
    }
    
    // Token not found in balances
    Ok(false)
}

// Get the associated token account for a mint and owner
pub fn get_token_account(wallet_pubkey: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address(wallet_pubkey, mint)
}

// Estimate transaction fees based on recent block data
pub async fn estimate_transaction_fees() -> Result<f64> {
    let client = RpcClient::new_with_timeout(
        get_rpc_url(),
        Duration::from_secs(30),
    );
    
    // Get recent blockhash - not used in this simplified approach but kept for future improvements
    let _recent_block_hash = client.get_latest_blockhash()?;
    
    // Since get_fee_calculator_for_blockhash is deprecated, we'll use a simpler approach
    // Estimate based on typical transaction costs
    // A typical swap transaction costs around 0.000005 SOL
    // We'll add a buffer for prioritization fees
    let estimated_sol = 0.001;
    
    // Add 50% buffer to account for network conditions
    let estimated_sol_with_buffer = estimated_sol * 1.5;
    
    info!("Estimated transaction fee: {} SOL", estimated_sol_with_buffer);
    Ok(estimated_sol_with_buffer)
} 
use crate::models::{TokenBalance, Wallet};
use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{
    solana_program::program_pack::Pack,
    state::Mint,
};
use std::str::FromStr;
use rand::rngs::OsRng;

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
            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => "BONK".to_string(),
            "7i5KKsX2weiTkry7jA4ZwSuXGhs5eJBEjY8vVxR4pfRx" => "GMT".to_string(),
            _ => mint[0..4].to_string() + "...",
        }
    }

    pub fn get_decimals(mint: &str) -> Result<u8> {
        match mint {
            "So11111111111111111111111111111111111111112" => Ok(9),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => Ok(6),
            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => Ok(5),
            "7i5KKsX2weiTkry7jA4ZwSuXGhs5eJBEjY8vVxR4pfRx" => Ok(8),
            _ => {
                // Query the blockchain for the token's info
                let client = RpcClient::new(get_rpc_url());
                let mint_pubkey = Pubkey::from_str(mint)?;
                let account_data = client.get_account_data(&mint_pubkey)?;
                let mint_data = Mint::unpack(&account_data)?;
                Ok(mint_data.decimals)
            }
        }
    }
}

// Helper function to get RPC URL based on environment
pub fn get_rpc_url() -> String {
    std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| SOLANA_MAINNET_URL.to_string())
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
    let mut rng = OsRng{};
    let mut mnemonic = String::new();
    
    for i in 0..12 {
        let index = (rand::Rng::gen::<u8>(&mut rng) as usize) % words.len();
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
    let rpc_client = RpcClient::new(get_rpc_url());
    let owner_pubkey = wallet.pubkey;
    let mut balances = Vec::new();

    // Add native SOL balance
    let sol_balance = rpc_client.get_balance(&owner_pubkey)?;
    balances.push(TokenBalance {
        mint: "So11111111111111111111111111111111111111112".to_string(),
        symbol: "SOL".to_string(),
        amount: sol_balance as f64,
        decimals: SOL_DECIMALS,
        ui_amount: sol_balance as f64 / 10f64.powi(SOL_DECIMALS as i32),
    });

    // Get SPL token accounts
    let token_accounts = rpc_client.get_token_accounts_by_owner(
        &owner_pubkey,
        TokenAccountsFilter::ProgramId(spl_token::id()),
    )?;

    for account in token_accounts {
        let token_balance = rpc_client.get_token_account_balance(&Pubkey::from_str(&account.pubkey)?)?;
        let token_account_data = rpc_client.get_account_data(&Pubkey::from_str(&account.pubkey)?)?;
        let token_account = spl_token::state::Account::unpack(&token_account_data)?;
        let mint = token_account.mint.to_string();
        let decimals = token_balance.decimals;
        let amount = token_balance.amount.parse::<f64>()?;
        let ui_amount = token_balance.ui_amount.unwrap_or(0.0);

        balances.push(TokenBalance {
            mint,
            symbol: KnownTokens::get_symbol(&token_account.mint.to_string()),
            amount,
            decimals,
            ui_amount,
        });
    }

    Ok(balances)
}

// Check if wallet has sufficient balance for a token
pub async fn has_sufficient_balance(wallet: &Wallet, token_mint: &str, amount_needed: f64) -> Result<bool> {
    let balances = get_token_balances(wallet).await?;
    
    // Find the token in the balances
    for balance in balances {
        if balance.mint == token_mint {
            return Ok(balance.ui_amount >= amount_needed);
        }
    }
    
    // Token not found in wallet
    Ok(false)
}

// Get the associated token account for a mint and owner
pub fn get_token_account(wallet_pubkey: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address(wallet_pubkey, mint)
} 
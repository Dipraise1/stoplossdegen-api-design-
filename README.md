# Solana Limit Order DEX

A lightweight Rust API server and web interface to manage Solana wallets, token swaps, and limit orders. This project allows users to set limit orders to buy or sell tokens on Solana when specific price conditions are met.

## Features

- **Wallet Management**: Generate new wallets or import existing ones using private keys or mnemonic phrases
- **Token Balances**: View balances for SOL and SPL tokens
- **Real-time Prices**: Track token prices from Jupiter and CoinGecko
- **Limit Orders**: Create, monitor, and cancel limit orders that execute automatically
- **Automatic Execution**: Orders execute automatically when price conditions are met
- **Web Interface**: Clean, modern UI for managing wallets and orders

## Prerequisites

- Rust (1.60+)
- A Solana RPC URL (mainnet or devnet)

## Installation

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/solana-limit-order-dex.git
   cd solana-limit-order-dex
   ```

2. Create a `.env` file:
   ```
   SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
   RUST_LOG=info
   ```

3. Install dependencies and build:
   ```
   cargo build --release
   ```

## Usage

1. Run the server:
   ```
   cargo run --release
   ```

2. Open the web interface:
   ```
   http://localhost:3000
   ```

3. Using the web interface:
   - Generate a new wallet or import an existing one
   - View your token balances
   - Create limit orders by specifying:
     - Order type (Buy/Sell)
     - Source and target tokens
     - Amount to swap
     - Price target
     - Optional slippage and expiry time
   - Monitor and cancel your active limit orders

## API Endpoints

- `POST /generate_wallet` - Generate a new wallet
- `POST /import_wallet` - Import an existing wallet
- `GET /get_balances` - Get token balances for the imported wallet
- `GET /get_prices` - Get current token prices
- `POST /swap_token` - Execute a token swap
- `POST /set_limit_order` - Create a new limit order
- `GET /list_limit_orders` - List all limit orders
- `POST /cancel_limit_order` - Cancel a specific limit order

## Technical Details

- Built with Axum web framework
- Uses Jupiter Aggregator for token swaps
- Fetches prices from multiple sources for accuracy
- Background task continuously monitors prices and executes orders
- Supports limit buy and sell orders with customizable parameters

## Security Considerations

**Important**: This is a prototype project. For production use:

1. Add proper authentication and user management
2. Use HTTPS for secure communication
3. Implement proper key management and encryption
4. Add rate limiting to prevent abuse
5. Consider additional security measures for handling private keys

## Development

### Running in Debug Mode

```
cargo run
```

### Running Tests

```
cargo test
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- [Solana](https://solana.com/)
- [Jupiter Aggregator](https://jup.ag/)
- [CoinGecko API](https://www.coingecko.com/en/api) # stoplossdegen-api-design-

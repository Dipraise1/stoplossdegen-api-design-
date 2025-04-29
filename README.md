# Solana Wallet API

A lightweight Rust API server to manage Solana wallets, token trades, limit orders, and portfolio tracking.

## Features

- Wallet management (create, import)
- Token balance tracking
- Token price monitoring
- Token swaps using Jupiter
- Limit order execution
- Stop loss order management
- Simple counter API example

## Prerequisites

- Rust and Cargo (latest stable version)
- Solana CLI (for interacting with the Solana blockchain)

## Getting Started

1. Clone the repository
```bash
git clone https://github.com/yourusername/solana_wallet_api.git
cd solana_wallet_api
```

2. Build the project
```bash
cargo build
```

3. Run the server
```bash
cargo run
```

The server will start on http://127.0.0.1:3301

## API Endpoints

### Counter Example API

- `GET /health` - Health check endpoint
- `GET /counter` - Get current counter value
- `POST /increment` - Increment counter by specified value
- `POST /decrement` - Decrement counter by specified value

#### Request Format for Increment/Decrement

```json
{
    "value": 5
}
```

### Wallet API

- `GET /health` - Health check endpoint
- `GET /get_balances` - Get token balances for the current wallet
- `GET /get_prices` - Get current token prices
- `POST /generate_wallet` - Generate a new wallet
- `POST /import_wallet` - Import a wallet using private key or mnemonic
- `POST /swap_token` - Execute a token swap
- `POST /set_limit_order` - Create a limit or stop loss order
- `GET /list_limit_orders` - List all active limit orders
- `POST /cancel_limit_order` - Cancel a specific limit order

## Development

The project is structured as follows:

- `src/main.rs` - Server entry point and route definitions
- `src/api.rs` - API endpoint implementations
- `src/handlers.rs` - Route handler functions
- `src/models.rs` - Data models and application state
- `src/price.rs` - Token price fetching functionality
- `src/swap.rs` - Token swap implementation
- `src/orders.rs` - Limit order management
- `src/wallet.rs` - Wallet functions (generation, import, balance checking)
- `src/utils.rs` - Utility functions

## License

MIT

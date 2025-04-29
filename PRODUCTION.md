# Production Deployment Guide

This guide covers how to deploy the Solana Limit Order DEX application in a production environment.

## Prerequisites

- Linux server (Ubuntu 20.04+ recommended)
- Docker and Docker Compose
- Domain name (optional, but recommended for production)
- Solana RPC URL (your own node or a service provider)

## Deployment Options

### Option 1: Docker Deployment (Recommended)

1. Clone the repository:
   ```
   git clone 
   cd solana-limit-order-dex
   ```

2. Configure environment variables:
   ```
   cp .env.example .env
   # Edit .env with your production settings
   ```

3. Build and start the application:
   ```
   docker-compose up -d
   ```

4. Check the logs:
   ```
   docker-compose logs -f
   ```

### Option 2: Native Deployment

1. Install Rust and dependencies:
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   apt-get update && apt-get install -y pkg-config libssl-dev
   ```

2. Clone the repository:
   ```
   git clone https://github.com/yourusername/solana-limit-order-dex.git
   cd solana-limit-order-dex
   ```

3. Configure environment variables:
   ```
   cp .env.example .env
   # Edit .env with your production settings
   ```

4. Build and run the application:
   ```
   cargo build --release
   ./target/release/solana_wallet_server
   ```

## Production Configuration

### Environment Variables

Edit your `.env` file for production settings:

```
# Solana RPC URL (use your own node or a service provider)
SOLANA_RPC_URL=https://your-rpc-provider.com

# Application settings
PORT=3000
HOST=0.0.0.0  # Use 0.0.0.0 to accept connections from any IP

# Logging
RUST_LOG=info

# API Keys for production
COINGECKO_API_KEY=your_api_key_here
JUPITER_API_KEY=your_api_key_here
```

### Configuring a Reverse Proxy (Nginx)

For production deployments, it's recommended to run the application behind a reverse proxy with SSL:

1. Install Nginx:
   ```
   apt-get install -y nginx certbot python3-certbot-nginx
   ```

2. Create an Nginx configuration file:
   ```
   nano /etc/nginx/sites-available/solana-dex
   ```

3. Add the following configuration:
   ```
   server {
       listen 80;
       server_name your-domain.com;

       location / {
           proxy_pass http://localhost:3000;
           proxy_http_version 1.1;
           proxy_set_header Upgrade $http_upgrade;
           proxy_set_header Connection 'upgrade';
           proxy_set_header Host $host;
           proxy_cache_bypass $http_upgrade;
       }
   }
   ```

4. Enable the site and get SSL certificate:
   ```
   ln -s /etc/nginx/sites-available/solana-dex /etc/nginx/sites-enabled/
   certbot --nginx -d your-domain.com
   nginx -t && systemctl restart nginx
   ```

## Health Checks

The application exposes a health check endpoint at `/health` that returns a 200 status code when the service is running normally.

## Monitoring

For production deployments, consider:

1. Setting up Prometheus and Grafana for metrics monitoring
2. Configuring log shipping to a central logging system (ELK stack, Loki, etc.)
3. Setting up alerting for critical errors

## Backup Strategy

Regularly back up important data:

1. Database (if added in the future)
2. Wallet keys and mnemonics (stored securely)
3. Application configuration

## Security Considerations

1. Never expose private keys or mnemonics
2. Use HTTPS for all communications
3. Implement proper authentication (JWT, OAuth, etc.)
4. Set up a firewall to restrict access
5. Regularly update dependencies
6. Consider a security audit

## Load Balancing

For high-traffic applications:

1. Deploy multiple instances behind a load balancer
2. Use stateless architecture
3. Consider using a container orchestration system like Kubernetes

## Troubleshooting

Common issues and solutions:

- **Application won't start**: Check logs with `docker-compose logs` or check the system journal
- **Connection issues**: Verify firewall settings and that the application is listening on the correct interface
- **API errors**: Check RPC URL configuration and API keys 
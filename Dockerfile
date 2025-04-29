FROM rust:1.72-slim-bullseye as builder

# Install dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev

# Create a new empty project
WORKDIR /app
RUN mkdir -p src
RUN echo "fn main() {}" > src/main.rs

# Copy the manifests and build dependencies to cache them
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# Now copy the actual source code
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/solana_wallet_server /usr/local/bin/solana_wallet_server

# Create directory for static files
WORKDIR /app
COPY static ./static

# Set environment variables
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=3000

# Expose the application port
EXPOSE 3000

# Command to run the application
CMD ["solana_wallet_server"] 
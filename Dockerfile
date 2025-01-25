# Build stage
FROM rust:1.74-slim as builder

WORKDIR /usr/src/authy

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy source for caching dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release

# Remove the dummy source code
RUN rm -rf src

# Copy actual source code
COPY src ./src

# Build the actual application
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /usr/src/authy/target/release/authy .

# Create a non-root user
RUN useradd -m -u 1000 -U -s /bin/sh -d /app authy && \
    chown -R authy:authy /app

USER authy

# Expose the port (will be overridden by runtime configuration)
EXPOSE 3000

# Set environment variables with defaults
ENV COGNITO_DOMAIN=""
ENV COGNITO_CLIENT_ID=""
ENV COGNITO_CLIENT_SECRET=""
ENV SERVER_DOMAIN=""
ENV PROTECTED_WEBSITE_URL=""
ENV PORT=3000
ENV RUST_LOG=info

# Run the binary
CMD ["./authy"]
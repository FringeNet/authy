# Authy - OAuth2 Authentication Gateway

Authy is a secure authentication gateway written in Rust that provides controlled access to protected websites through Amazon Cognito authentication. It acts as a secure proxy that ensures only authenticated users can access protected resources.

## Architecture Overview

The service is a high-performance Rust application that:
- Handles OAuth2 authentication flow with Amazon Cognito
- Validates user authentication
- Proxies authenticated requests to protected websites
- Provides security through token validation and access control

## How It Works

1. User attempts to access a protected resource
2. They are redirected to the Cognito login page
3. Upon successful authentication:
    - User is redirected back with an authorization code
    - Server exchanges the code for access tokens
    - Server validates the tokens
    - User is granted access to the protected resource
4. All subsequent requests are:
    - Validated using JWT tokens
    - Proxied to the protected website after validation

## Protected Service Access

The main benefit of this setup is that internal services remain unexposed to the public internet. Instead:
- Services run only on localhost
- Access is only possible through the authenticated proxy
- Additional security layer through token validation

## Setup Requirements

1. **AWS Cognito Setup**
    - User Pool configuration
    - App Client setup with OAuth2 enabled
    - Configure callback URLs
    - Set up hosted UI domain

2. **Rust Setup**
    - Install Rust using [rustup](https://rustup.rs/)
    - Clone this repository
    - Copy `.env.example` to `.env` and configure it

## Development

```bash
# Build the project
cargo build

# Run in development mode
cargo run

# Run tests
cargo test

# Build for production
cargo build --release
```

## Environment Variables

```env
# AWS Cognito Configuration
COGNITO_DOMAIN=https://your-domain.auth.region.amazoncognito.com
COGNITO_CLIENT_ID=your-client-id
COGNITO_CLIENT_SECRET=your-client-secret
SERVER_DOMAIN=http://your-server-domain

# Protected Resource
PROTECTED_WEBSITE_URL=https://website-to-protect.com

# Server Configuration
PORT=3000
RUST_LOG=info
```

## Security Considerations

- All communication uses HTTPS
- OAuth2 authorization code flow
- JWT token validation on every request
- Protected resources never directly exposed
- Secure session management
- IP-based access logging
- Unauthorized access monitoring
- Memory-safe implementation in Rust

## Project Structure

```
src/
├── auth/       # Authentication handling
├── config/     # Configuration management
├── error/      # Error types and handling
├── proxy/      # Proxy implementation
└── main.rs     # Application entry point
```

## Features

- **High Performance**: Built with Rust for optimal performance and resource usage
- **Memory Safety**: Leverages Rust's memory safety guarantees
- **Async I/O**: Uses Tokio for asynchronous I/O operations
- **Error Handling**: Comprehensive error handling with custom error types
- **Logging**: Structured logging with different log levels
- **CORS Support**: Configurable CORS settings
- **Header Filtering**: Intelligent handling of HTTP headers
- **Request Streaming**: Efficient streaming of request/response bodies

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT

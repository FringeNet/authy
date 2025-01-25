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

### Local Development

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

### Docker Deployment

The service can be run using Docker in two ways:

1. Using docker-compose (recommended):
```bash
# Copy example environment file
cp .env.example .env

# Edit environment variables
vim .env

# Start the service
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the service
docker-compose down
```

2. Using Docker directly:
```bash
# Build the image
docker build -t authy .

# Run the container
docker run -d \
  -p 3000:3000 \
  -e COGNITO_DOMAIN=https://your-domain.auth.region.amazoncognito.com \
  -e COGNITO_CLIENT_ID=your-client-id \
  -e COGNITO_CLIENT_SECRET=your-client-secret \
  -e SERVER_DOMAIN=http://your-server-domain \
  -e PROTECTED_WEBSITE_URL=https://website-to-protect.com \
  -e PORT=3000 \
  -e RUST_LOG=info \
  --name authy \
  authy

# View logs
docker logs -f authy

# Stop the container
docker stop authy
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `COGNITO_DOMAIN` | AWS Cognito domain URL | Required |
| `COGNITO_CLIENT_ID` | AWS Cognito client ID | Required |
| `COGNITO_CLIENT_SECRET` | AWS Cognito client secret | Required |
| `SERVER_DOMAIN` | Public domain where this service is hosted | Required |
| `PROTECTED_WEBSITE_URL` | URL of the website to protect | Required |
| `PORT` | Port to listen on | 3000 |
| `RUST_LOG` | Log level (error, warn, info, debug, trace) | info |

## AWS Cognito Setup

### 1. Create User Pool
```bash
# Navigate to AWS Cognito Console
AWS Console -> Amazon Cognito -> User Pools -> Create user pool
```
- Choose "Cognito user pool" as the provider type
- Configure sign-in options:
  - Allow users to sign in with: Email
  - Allow users to sign up with: Email
- Configure security requirements:
  - Password minimum length: 8
  - Enable MFA (recommended): Optional
- Configure sign-up experience:
  - Enable self-service account recovery
  - Enable self-service sign-up
- Configure message delivery:
  - Email provider: Amazon SES or Cognito defaults

### 2. Configure App Integration
```bash
# In User Pool settings
App integration -> App client list -> Create app client
```
- Create app client:
  - App client name: "Authy"
  - Public client: No
  - Generate client secret: Yes
  - Authentication flows:
    - ✓ ALLOW_USER_PASSWORD_AUTH
    - ✓ ALLOW_REFRESH_TOKEN_AUTH
- OAuth 2.0 settings:
  - Allowed OAuth flows:
    - ✓ Authorization code grant
  - Allowed OAuth scopes:
    - ✓ openid
    - ✓ email
    - ✓ profile
  - Callback URLs:
    - `https://your-domain.com/callback`
  - Sign out URLs (optional):
    - `https://your-domain.com/signout`

### 3. Configure Domain
```bash
# In User Pool settings
App integration -> Domain -> Actions -> Create custom domain
```
- Choose domain type:
  - Cognito domain: `your-prefix.auth.region.amazoncognito.com`
  - Custom domain (requires SSL cert): `auth.your-domain.com`

### 4. Configure Hosted UI
```bash
# In User Pool settings
App integration -> Hosted UI
```
- Customize appearance:
  - Logo image
  - CSS customizations
  - Color scheme
- Configure sign-in/sign-up options:
  - Email verification
  - Password requirements
  - Custom attributes

### 5. Note Required Values
```bash
# User Pool settings
General settings -> User pool ID
# Example: us-east-1_abcd1234

# App client settings
App integration -> App client list -> Client ID
# Example: 1234567890abcdef1234

# App client settings -> Show client secret
# Example: abcdef1234567890abcdef1234567890

# Domain
App integration -> Domain
# Example: https://your-prefix.auth.region.amazoncognito.com
```

### 6. Optional: Add Users
```bash
# In User Pool settings
Users -> Create user
```
- Create admin user:
  - Email: admin@your-domain.com
  - Temporary password: Yes
  - Mark email as verified
- Or enable self-service sign-up:
  - Users can create their own accounts
  - Email verification required
  - Optional admin approval

### 7. Security Best Practices
- Use strong password policies
- Enable MFA for sensitive applications
- Regularly rotate app client secrets
- Monitor user pool analytics
- Set up CloudWatch alarms for:
  - Failed authentication attempts
  - User pool modifications
  - Token usage patterns

### 8. Testing
```bash
# Test authentication flow
curl http://localhost:3000/
# Should redirect to Cognito login page
# After login, should redirect back to /callback
# Then redirect to protected website
```

### 9. Troubleshooting
- Check callback URL matches exactly
- Verify client ID and secret
- Ensure OAuth scopes are correct
- Check CORS settings if using SPA
- Monitor CloudWatch logs for errors

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

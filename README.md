# Authy - Authentication Proxy Service

Authy is a secure authentication proxy service that provides controlled access to internal services through Amazon Cognito authentication. It consists of a React frontend application and a proxy server that securely forwards authenticated requests to protected services.

## Architecture Overview

The application consists of two main components:

1. **Frontend (React Application)**
   - User interface for authentication
   - Integration with Amazon Cognito for secure user management
   - Protected routes that require authentication
   - Proxy request handling for authenticated users

2. **Backend (Proxy Server)**
   - Token validation
   - Request forwarding to protected services
   - Security middleware

## How It Works

1. Users access the React application
2. They are presented with a login screen powered by Amazon Cognito
3. Upon successful authentication:
   - User receives JWT tokens
   - Frontend validates tokens
   - Authenticated requests are proxied to protected services
4. All subsequent requests to protected services are:
   - Authenticated using JWT tokens
   - Proxied through the backend server
   - Forwarded to the appropriate internal service

## Protected Service Access

The main benefit of this setup is that internal services remain unexposed to the public internet. Instead:
- Services run only on localhost
- Access is only possible through the authenticated proxy
- Additional security layer through token validation

## Setup Requirements

1. **AWS Cognito Setup**
   - User Pool configuration
   - App Client setup
   - Required environment variables:
     - COGNITO_USER_POOL_ID
     - COGNITO_CLIENT_ID
     - COGNITO_REGION

2. **Frontend Configuration**
   - React application setup
   - Environment variables for Cognito configuration
   - Proxy configuration

3. **Backend Configuration**
   - Proxy server setup
   - Token validation middleware
   - Service routing configuration

## Development

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build
```

## Security Considerations

- All communication uses HTTPS
- JWT tokens are validated on every request
- Protected services are never directly exposed
- Regular token rotation
- Secure session management

## Environment Variables

```env
# AWS Cognito
COGNITO_USER_POOL_ID=your-pool-id
COGNITO_CLIENT_ID=your-client-id
COGNITO_REGION=your-region

# Proxy Configuration
PROXY_TARGET=http://localhost:your-service-port
```

## License

MIT
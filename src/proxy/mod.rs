use crate::{config::Config, error::AppError};
use axum::{
    body::{Body, to_bytes},
    extract::State,
    http::{HeaderName, HeaderValue, Request, Response, StatusCode},
    response::IntoResponse,
};
use std::str::FromStr;

pub async fn proxy_request(
    State(config): State<Config>,
    req: Request<Body>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Validate JWT token from session/cookie
    
    // Create client
    let client = reqwest::Client::new();

    // Build the proxy URL
    let path = req.uri().path();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let proxy_url = format!("{}{}{}", config.protected_website_url, path, query);

    // Convert method
    let method = reqwest::Method::from_str(req.method().as_str())
        .map_err(|e| AppError::Internal(format!("Invalid method: {}", e)))?;

    // Create proxied request
    let mut proxy_req = client.request(method, proxy_url);

    // Forward relevant headers
    for (key, value) in req.headers() {
        if !is_hop_header(key) {
            if let Ok(name) = reqwest::header::HeaderName::from_str(key.as_str()) {
                if let Ok(val) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                    proxy_req = proxy_req.header(name, val);
                }
            }
        }
    }

    // Convert body
    let body_bytes = to_bytes(req.into_body(), 1024 * 1024 * 10) // 10MB limit
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read request body: {}", e)))?;
    proxy_req = proxy_req.body(body_bytes);

    // Send request
    let proxy_response = proxy_req
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Proxy request failed: {}", e)))?;

    // Get response parts
    let status = StatusCode::from_u16(proxy_response.status().as_u16())
        .map_err(|e| AppError::Internal(format!("Invalid status code: {}", e)))?;
    
    let headers = proxy_response.headers().clone();
    let body = proxy_response.bytes().await?;

    // Build response
    let mut builder = Response::builder().status(status);
    let response_headers = builder.headers_mut().unwrap();

    // Forward response headers
    for (key, value) in headers.iter() {
        if !is_hop_header_str(key.as_str()) {
            if let Ok(name) = HeaderName::from_str(key.as_str()) {
                if let Ok(val) = HeaderValue::from_bytes(value.as_bytes()) {
                    response_headers.insert(name, val);
                }
            }
        }
    }

    Ok(builder.body(Body::from(body))
        .map_err(|e| AppError::Internal(format!("Failed to build response: {}", e)))?)
}

fn is_hop_header(name: &HeaderName) -> bool {
    is_hop_header_str(name.as_str())
}

fn is_hop_header_str(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}
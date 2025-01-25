use std::time::Instant;
use axum::{
    extract::ConnectInfo,
    http::{Request, Response},
    middleware::Next,
    body::Body,
};
use tracing::{info, warn};

pub async fn access_log(req: Request<Body>, next: Next) -> Response<Body> {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = req.version();
    
    // Get client IP from headers or socket
    let client_ip = req.headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            req.extensions()
                .get::<ConnectInfo<std::net::SocketAddr>>()
                .map(|addr| addr.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

    // Process the request
    let response = next.run(req).await;
    let status = response.status();
    let duration = start.elapsed();

    // Log the access
    if status.is_success() {
        info!(
            target: "access_log",
            "IP={} {} {} {:?} {} {}ms",
            client_ip,
            method,
            uri,
            version,
            status.as_u16(),
            duration.as_millis()
        );
    } else {
        warn!(
            target: "access_log",
            "IP={} {} {} {:?} {} {}ms",
            client_ip,
            method,
            uri,
            version,
            status.as_u16(),
            duration.as_millis()
        );
    }

    response
}
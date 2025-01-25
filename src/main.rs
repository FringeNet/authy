mod auth;
mod config;
mod error;
mod proxy;
mod session;
mod middleware;

use axum::{
    routing::get,
    Router,
    http::{Method, HeaderName, HeaderValue, StatusCode, header::{AUTHORIZATION, ACCEPT, CONTENT_TYPE}, Request},
    response::IntoResponse,
    extract::State,
    body::Body,
};
use crate::{config::Config, proxy::proxy_request};
use dotenv::dotenv;
use std::{net::SocketAddr, time::Duration};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");
    let port = config.port;

    // Configure CORS
    let cors = build_cors_layer(&config);

    // Build application
    let app = Router::new()
        .route("/", get(auth::login))
        .route("/callback", get(auth::callback))
        .route("/health", get(health_check))
        .fallback(|State(config): State<Config>, req: Request<Body>| async move {
            proxy_request(State(config), req).await
        })
        .layer(cors)
        .layer(axum::middleware::from_fn(middleware::access_log))
        .with_state(config);

fn build_cors_layer(config: &Config) -> CorsLayer {
    if config.cors_allowed_origins.contains(&"*".to_string()) {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_credentials(true)
            .max_age(Duration::from_secs(3600))
    } else {
        CorsLayer::new()
            .allow_origin(config.cors_allowed_origins.iter().map(|origin| {
                origin.parse::<HeaderValue>().unwrap_or_else(|_| {
                    HeaderValue::from_static("http://localhost:3000")
                })
            }).collect::<Vec<_>>())
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                AUTHORIZATION,
                ACCEPT,
                CONTENT_TYPE,
                HeaderName::from_static("x-requested-with"),
            ])
            .allow_credentials(true)
            .max_age(Duration::from_secs(3600))
    }
}

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting server on {}", addr);
    
    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

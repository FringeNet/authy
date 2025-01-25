mod auth;
mod config;
mod error;
mod proxy;

use axum::{
    routing::{get, any},
    Router,
    http::{Method, HeaderName, HeaderValue},
};
use config::Config;
use dotenv::dotenv;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use http::header::{AUTHORIZATION, ACCEPT, CONTENT_TYPE};

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
    let cors = if config.cors_allowed_origins.contains(&"*".to_string()) {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_credentials(true)
            .max_age(3600)
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
            .max_age(3600)
    };

    // Build application
    let app = Router::new()
        .route("/", get(auth::login))
        .route("/callback", get(auth::callback))
        .route("/health", get(health_check))
        .fallback(any(proxy::proxy_request))
        .layer(cors)
        .with_state(config);

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting server on {}", addr);
    
    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app.into_make_service(),
    )
    .await
    .unwrap();
}

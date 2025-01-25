use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Unauthorized access: {message}")]
    Unauthorized {
        message: String,
        client_ip: String,
        path: String,
    },
    
    #[error("Configuration error: {0}")]
    Config(#[from] std::env::VarError),
    
    #[error("HTTP request error: {0}")]
    Request(#[from] reqwest::Error),
    
    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::Unauthorized { message, client_ip, path } => {
                tracing::warn!(
                    target: "security_log",
                    "Unauthorized access attempt from IP={} to path={}. Reason: {}",
                    client_ip,
                    path,
                    message
                );
                (StatusCode::UNAUTHORIZED, message)
            },
            AppError::Config(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Request(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, message).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::Response;
    use http_body_util::BodyExt;
    use std::env;

    async fn get_response_body(response: Response) -> String {
        let body = response.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn test_auth_error_response() {
        let error = AppError::Auth("Invalid token".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(get_response_body(response).await, "Invalid token");
    }

    #[tokio::test]
    async fn test_unauthorized_error_response() {
        let error = AppError::Unauthorized {
            message: "Invalid token".to_string(),
            client_ip: "192.168.1.1".to_string(),
            path: "/protected".to_string(),
        };
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(get_response_body(response).await, "Invalid token");
    }

    #[tokio::test]
    async fn test_config_error_response() {
        let error = AppError::Config(env::VarError::NotPresent);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(get_response_body(response).await, "environment variable not found");
    }

    #[tokio::test]
    async fn test_request_error_response() {
        let client = reqwest::Client::new();
        let error = AppError::Request(client.get("http://invalid-url").send().await.unwrap_err());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        assert!(get_response_body(response).await.contains("error"));
    }

    #[tokio::test]
    async fn test_internal_error_response() {
        let error = AppError::Internal("Server error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(get_response_body(response).await, "Server error");
    }
}
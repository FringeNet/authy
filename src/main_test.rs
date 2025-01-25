#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_cors_wildcard() {
        let config = Config {
            cognito_domain: "https://test.auth.amazoncognito.com".to_string(),
            cognito_client_id: "test-client-id".to_string(),
            cognito_client_secret: "test-client-secret".to_string(),
            server_domain: "http://localhost:3000".to_string(),
            protected_website_url: "http://internal.example.com".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["*".to_string()],
            behind_proxy: false,
        };

        let app = Router::new()
            .route("/health", get(health_check))
            .layer(build_cors_layer(&config))
            .with_state(config);

        // Test preflight request
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/health")
                    .header("Origin", "https://example.com")
                    .header("Access-Control-Request-Method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("access-control-allow-origin").unwrap(),
            "*"
        );
        assert!(response
            .headers()
            .get("access-control-allow-credentials")
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<bool>()
            .unwrap());
    }

    #[tokio::test]
    async fn test_cors_specific_origin() {
        let config = Config {
            cognito_domain: "https://test.auth.amazoncognito.com".to_string(),
            cognito_client_id: "test-client-id".to_string(),
            cognito_client_secret: "test-client-secret".to_string(),
            server_domain: "http://localhost:3000".to_string(),
            protected_website_url: "http://internal.example.com".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["https://app.example.com".to_string()],
            behind_proxy: false,
        };

        let app = Router::new()
            .route("/health", get(health_check))
            .layer(build_cors_layer(&config))
            .with_state(config);

        // Test preflight request with allowed origin
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/health")
                    .header("Origin", "https://app.example.com")
                    .header("Access-Control-Request-Method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("access-control-allow-origin").unwrap(),
            "https://app.example.com"
        );
        assert!(response
            .headers()
            .get("access-control-allow-credentials")
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<bool>()
            .unwrap());

        // Test preflight request with disallowed origin
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/health")
                    .header("Origin", "https://evil.example.com")
                    .header("Access-Control-Request-Method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
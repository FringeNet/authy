use crate::{config::Config, error::AppError};
use axum::{
    body::{Body, to_bytes},
    extract::State,
    http::{HeaderName, HeaderValue, Request, Response, StatusCode},
    response::IntoResponse,
};
use std::str::FromStr;
use reqwest::header::HeaderValue as ReqHeaderValue;

pub async fn proxy_request(
    State(config): State<Config>,
    mut req: Request<Body>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Validate JWT token from session/cookie
    
    // Create client
    let client = reqwest::Client::new();

    // Build the proxy URL
    let path = req.uri().path();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let proxy_url = format!("{}{}{}", config.protected_website_url, path, query);

    // Get request parts
    let (parts, body) = req.into_parts();
    let is_https_request = if config.behind_proxy {
        parts.headers.get("x-forwarded-proto").map_or(false, |h| h.to_str().unwrap_or("") == "https")
    } else {
        false
    };

    // Get request method
    let method = reqwest::Method::from_str(parts.method.as_str())
        .map_err(|e| AppError::Internal(format!("Invalid method: {}", e)))?;

    // Create proxied request with method
    let mut proxy_req = client.request(method, &proxy_url);

    // Forward headers and handle protocol transitions
    let mut host_header = None;
    for (key, value) in parts.headers.iter() {
        if key == "host" {
            host_header = Some(value.to_str().unwrap_or("").to_string());
        }
        if !is_hop_header(key) {
            if let Ok(name) = reqwest::header::HeaderName::from_str(key.as_str()) {
                if let Ok(val) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                    proxy_req = proxy_req.header(name, val);
                }
            }
        }
    }

    // Add protocol transition headers if needed
    if config.behind_proxy {
        proxy_req = proxy_req
            .header("x-forwarded-proto", if is_https_request { "https" } else { "http" })
            .header("x-forwarded-host", host_header.unwrap_or_default());
    }

    // Convert body
    let body_bytes = to_bytes(body, 1024 * 1024 * 10) // 10MB limit
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

    // Use the same HTTPS flag from the request
    let is_https = is_https_request;

    // Forward response headers
    for (key, value) in headers.iter() {
        if !is_hop_header_str(key.as_str()) {
            if let Ok(name) = HeaderName::from_str(key.as_str()) {
                if let Ok(val) = HeaderValue::from_bytes(value.as_bytes()) {
                    // Special handling for cookies when behind proxy
                    if config.behind_proxy && key.as_str().eq_ignore_ascii_case("set-cookie") {
                        if let Ok(cookie_str) = val.to_str() {
                            // Parse and modify cookie
                            if let Ok(mut cookie) = cookie::Cookie::parse(cookie_str) {
                                // If the request came through HTTPS, ensure cookie is secure
                                if is_https {
                                    cookie.set_secure(true);
                                }
                                // Set SameSite attribute
                                cookie.set_same_site(Some(cookie::SameSite::Lax));
                                
                                // Convert back to header value
                                let mut parts = vec![cookie.to_string()];
                                if is_https {
                                    parts.push("secure".to_string());
                                }
                                parts.push("SameSite=Lax".to_string());
                                let cookie_str = parts.join("; ");
                                if let Ok(new_val) = HeaderValue::from_str(&cookie_str) {
                                    response_headers.insert(name.clone(), new_val);
                                    continue;
                                }
                            }
                        }
                    }
                    // Handle redirects when behind proxy
                    else if config.behind_proxy && key.as_str().eq_ignore_ascii_case("location") {
                        if let Ok(location) = val.to_str() {
                            if location.starts_with("http://") && is_https {
                                // Convert http:// to https:// in redirects
                                if let Ok(new_val) = HeaderValue::from_str(&location.replace("http://", "https://")) {
                                    response_headers.insert(name.clone(), new_val);
                                    continue;
                                }
                            }
                            response_headers.insert(name.clone(), val);
                            continue;
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request};
    use http_body_util::BodyExt;
    use wiremock::{
        matchers::{method, path, header},
        Mock, MockServer, ResponseTemplate,
    };

    async fn get_response_body(response: &mut Response<Body>) -> String {
        let body = std::mem::replace(response.body_mut(), Body::empty());
        let bytes = to_bytes(body, 1024 * 1024 * 10).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn create_test_config(url: String) -> Config {
        Config {
            cognito_domain: "https://test.auth.amazoncognito.com".to_string(),
            cognito_client_id: "test-client-id".to_string(),
            cognito_client_secret: "test-client-secret".to_string(),
            server_domain: "http://localhost:3000".to_string(),
            protected_website_url: url,
            port: 3000,
            cors_allowed_origins: vec!["https://app.example.com".to_string()],
            behind_proxy: false,
        }
    }

    #[tokio::test]
    async fn test_proxy_request_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string("test response")
                .insert_header("content-type", "text/plain"))
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("accept", "text/plain")
            .body(Body::empty())
            .unwrap();

        let mut response = proxy_request(State(config), request).await.unwrap().into_response();
        
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/plain"
        );
        assert_eq!(get_response_body(&mut response).await, "test response");
    }

    #[tokio::test]
    async fn test_proxy_request_with_query() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("test with query"))
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test?param=value")
            .body(Body::empty())
            .unwrap();

        let mut response = proxy_request(State(config), request).await.unwrap().into_response();
        
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(get_response_body(&mut response).await, "test with query");
    }

    #[tokio::test]
    async fn test_proxy_request_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());

        let request = Request::builder()
            .method(Method::GET)
            .uri("/error")
            .body(Body::empty())
            .unwrap();

        let mut response = proxy_request(State(config), request).await.unwrap().into_response();
        
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(get_response_body(&mut response).await, "server error");
    }

    #[tokio::test]
    async fn test_proxy_request_with_https_behind_proxy() {
        let mock_server = MockServer::start().await;

        // Expect forwarded headers
        Mock::given(method("GET"))
            .and(path("/secure"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string("secure response")
                .insert_header("set-cookie", "session=123; Path=/"))
            .mount(&mock_server)
            .await;

        let mut config = create_test_config(mock_server.uri());
        config.behind_proxy = true;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/secure")
            .header("x-forwarded-proto", "https")
            .header("host", "auth.example.com")
            .body(Body::empty())
            .unwrap();

        let mut response = proxy_request(State(config), request).await.unwrap().into_response();
        
        assert_eq!(response.status(), StatusCode::OK);
        let body = get_response_body(&mut response).await;
        
        // Check that cookie was modified
        let cookie = response.headers().get("set-cookie").unwrap().to_str().unwrap();
        assert!(cookie.contains("secure"));
        assert!(cookie.contains("SameSite=Lax"));
        assert_eq!(body, "secure response");
    }

    #[tokio::test]
    async fn test_proxy_request_with_redirect() {
        let mock_server = MockServer::start().await;

        let location = format!("http://{}/target", mock_server.uri().split("://").nth(1).unwrap());
        let mock = Mock::given(method("GET"))
            .and(path("/redirect"))
            .respond_with(ResponseTemplate::new(302)
                .append_header("location", location.as_str()))
            .expect(1)
            .mount(&mock_server)
            .await;

        let mut config = create_test_config(mock_server.uri());
        config.behind_proxy = true;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/redirect")
            .header("x-forwarded-proto", "https")
            .header("host", "auth.example.com")
            .body(Body::empty())
            .unwrap();

        let response = proxy_request(State(config), request).await.unwrap().into_response();
        
        assert_eq!(response.status(), StatusCode::FOUND);
        let location = response.headers().get("location").unwrap();
        assert!(location.to_str().unwrap().starts_with("https://"));

        // Verify that the mock was called
        mock.assert();
    }

    #[test]
    fn test_is_hop_header() {
        assert!(is_hop_header_str("connection"));
        assert!(is_hop_header_str("keep-alive"));
        assert!(is_hop_header_str("proxy-authenticate"));
        assert!(is_hop_header_str("proxy-authorization"));
        assert!(is_hop_header_str("te"));
        assert!(is_hop_header_str("trailer"));
        assert!(is_hop_header_str("transfer-encoding"));
        assert!(is_hop_header_str("upgrade"));

        assert!(!is_hop_header_str("content-type"));
        assert!(!is_hop_header_str("authorization"));
        assert!(!is_hop_header_str("accept"));
    }
}
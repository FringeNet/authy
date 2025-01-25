use crate::{config::Config, error::AppError};
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    http::StatusCode,
    body::Body,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize)]
pub struct AuthCallback {
    code: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct TokenRequest {
    grant_type: String,
    client_id: String,
    code: String,
    redirect_uri: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u32,
    id_token: Option<String>,
}

pub async fn login(State(config): State<Config>) -> Redirect {
    let mut url = Url::parse(&format!("{}/login", config.cognito_domain))
        .expect("Failed to parse Cognito domain");

    url.query_pairs_mut()
        .append_pair("client_id", &config.cognito_client_id)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid")
        .append_pair(
            "redirect_uri",
            &format!("{}/callback", config.server_domain),
        );

    Redirect::to(url.as_str())
}

pub async fn callback(
    State(config): State<Config>,
    Query(params): Query<AuthCallback>,
) -> Result<impl IntoResponse, AppError> {
    let code = params
        .code
        .ok_or_else(|| AppError::Auth("No authorization code provided".into()))?;

    if let Some(error) = params.error {
        return Err(AppError::Auth(error));
    }

    let token = exchange_code_for_token(&config, &code).await?;
    
    // Create a session cookie with the access token
    let is_https = config.server_domain.starts_with("https://");
    let cookie = crate::session::create_session_cookie(&token.access_token, is_https);
    
    // Build response with cookie and redirect
    let response = Response::builder()
        .status(StatusCode::FOUND)
        .header("location", &config.protected_website_url)
        .header("set-cookie", cookie.to_string())
        .body(Body::empty())
        .map_err(|e| AppError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

async fn exchange_code_for_token(config: &Config, code: &str) -> Result<TokenResponse, AppError> {
    let client = reqwest::Client::new();
    let token_url = format!("{}/oauth2/token", config.cognito_domain);
    
    let response = client
        .post(&token_url)
        .basic_auth(&config.cognito_client_id, Some(&config.cognito_client_secret))
        .form(&TokenRequest {
            grant_type: "authorization_code".into(),
            client_id: config.cognito_client_id.clone(),
            code: code.into(),
            redirect_uri: format!("{}/callback", config.server_domain),
        })
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(AppError::Auth(error));
    }

    response.json::<TokenResponse>().await.map_err(AppError::Request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    fn create_test_config(cognito_domain: String) -> Config {
        Config {
            cognito_domain,
            cognito_client_id: "test-client-id".to_string(),
            cognito_client_secret: "test-client-secret".to_string(),
            server_domain: "http://localhost:3000".to_string(),
            protected_website_url: "https://test-website.com".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["*".to_string()],
            behind_proxy: false,
        }
    }

    #[tokio::test]
    async fn test_login_redirect() {
        let config = create_test_config("https://test.auth.region.amazoncognito.com".to_string());

        let response = login(State(config)).await.into_response();
        let location = response.headers().get("location").unwrap().to_str().unwrap();
        
        assert!(location.starts_with("https://test.auth.region.amazoncognito.com/login"));
        assert!(location.contains("client_id=test-client-id"));
        assert!(location.contains("response_type=code"));
        assert!(location.contains("scope=openid"));
        assert!(location.contains("redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fcallback"));
    }

    #[tokio::test]
    async fn test_callback_no_code() {
        let config = create_test_config("https://test.auth.amazoncognito.com".to_string());

        let params = AuthCallback {
            code: None,
            error: None,
        };

        let result = callback(State(config), Query(params)).await;
        assert!(matches!(result, Err(AppError::Auth(msg)) if msg == "No authorization code provided"));
    }

    #[tokio::test]
    async fn test_callback_with_error() {
        let config = create_test_config("https://test.auth.amazoncognito.com".to_string());

        let params = AuthCallback {
            code: Some("test-code".to_string()),
            error: Some("access_denied".to_string()),
        };

        let result = callback(State(config), Query(params)).await;
        assert!(matches!(result, Err(AppError::Auth(msg)) if msg == "access_denied"));
    }

    #[tokio::test]
    async fn test_exchange_code_success() {
        let mock_server = MockServer::start().await;
        
        let token_response = TokenResponse {
            access_token: "test-access-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            id_token: Some("test-id-token".to_string()),
        };

        Mock::given(method("POST"))
            .and(path("/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&token_response))
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());

        let result = exchange_code_for_token(&config, "test-code").await.unwrap();
        assert_eq!(result, token_response);
    }

    #[tokio::test]
    async fn test_exchange_code_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/oauth2/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string("invalid_grant"))
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());

        let result = exchange_code_for_token(&config, "invalid-code").await;
        assert!(matches!(result, Err(AppError::Auth(msg)) if msg == "invalid_grant"));
    }
}

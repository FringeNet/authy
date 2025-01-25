use crate::{config::Config, error::AppError};
use axum::{
    extract::{Query, State},
    response::Redirect,
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

#[derive(Debug, Deserialize)]
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
) -> Result<Redirect, AppError> {
    let code = params
        .code
        .ok_or_else(|| AppError::Auth("No authorization code provided".into()))?;

    if let Some(error) = params.error {
        return Err(AppError::Auth(error));
    }

    let _token = exchange_code_for_token(&config, &code).await?;
    
    // Here you would typically set up a session or cookie with the token
    // For now, we'll redirect to the protected website
    Ok(Redirect::to(&config.protected_website_url))
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